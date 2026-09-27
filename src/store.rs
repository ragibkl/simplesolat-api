//! In-memory view of the simplesolat-data CDN.
//!
//! Everything is fetched on demand by the request that needs it and cached
//! with a TTL; there are no background fetches. Concurrent requests for the
//! same thing share one fetch.
//!
//! - Zone index (countries.yaml + zones/*.yaml): one entry, `index_ttl`. If a
//!   reload fails, the last good index keeps being served.
//! - Prayer times: one entry per month file. A found month is cached for
//!   `month_ttl`; a 404 (month not published yet) is cached as empty for the
//!   shorter `missing_ttl`, so newly published months show up sooner. Errors
//!   are never cached: if a refetch fails, the last good copy (kept for
//!   `STALE_RETENTION`) is served instead.

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use chrono::{Datelike, NaiveDate};
use futures::{StreamExt, future, stream};
use moka::{Expiry, future::Cache};
use tokio::sync::RwLock;

use crate::api::data_repo::{self, Country, Error, PrayerTimeRecord, Zone};

/// How long a month's last good copy is kept to serve when refetching fails.
const STALE_RETENTION: Duration = Duration::from_secs(30 * 86400);
/// Month cache size. A month file is ~3 KB of JSON: tens of MB per cache.
const MONTH_CACHE_CAPACITY: u64 = 20_000;
/// How many country zone files an index refresh fetches in parallel.
const ZONE_FETCH_CONCURRENCY: usize = 4;

pub struct Index {
    /// Sorted by code.
    pub countries: Vec<Country>,
    /// Sorted by zone code.
    pub zones: Vec<Zone>,
    by_code: HashMap<String, usize>,
}

impl Index {
    fn new(mut countries: Vec<Country>, mut zones: Vec<Zone>) -> Self {
        countries.sort_by(|a, b| a.code.cmp(&b.code));
        zones.sort_by(|a, b| a.code.cmp(&b.code));
        let by_code = zones
            .iter()
            .enumerate()
            .map(|(i, z)| (z.code.clone(), i))
            .collect();
        Self {
            countries,
            zones,
            by_code,
        }
    }

    pub fn zone(&self, code: &str) -> Option<&Zone> {
        self.by_code.get(code).map(|&i| &self.zones[i])
    }
}

type MonthKey = (String, String, i32, u32);
type Month = Arc<Vec<PrayerTimeRecord>>;

/// Found months live for `found`, 404s (cached as empty) for `missing`.
struct MonthExpiry {
    found: Duration,
    missing: Duration,
}

impl Expiry<MonthKey, Month> for MonthExpiry {
    fn expire_after_create(&self, _key: &MonthKey, value: &Month, _now: Instant) -> Option<Duration> {
        Some(if value.is_empty() { self.missing } else { self.found })
    }
}

pub struct DataStore {
    client: reqwest::Client,
    base_url: String,
    index: Cache<(), Arc<Index>>,
    last_index: RwLock<Option<Arc<Index>>>,
    months: Cache<MonthKey, Month>,
    stale_months: Cache<MonthKey, Month>,
}

impl DataStore {
    pub fn new(
        base_url: impl Into<String>,
        index_ttl: Duration,
        month_ttl: Duration,
        missing_ttl: Duration,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build HTTP client");

        Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            index: Cache::builder().max_capacity(1).time_to_live(index_ttl).build(),
            last_index: RwLock::new(None),
            months: Cache::builder()
                .max_capacity(MONTH_CACHE_CAPACITY)
                .expire_after(MonthExpiry {
                    found: month_ttl,
                    missing: missing_ttl,
                })
                .build(),
            stale_months: Cache::builder()
                .max_capacity(MONTH_CACHE_CAPACITY)
                .time_to_live(STALE_RETENTION)
                .build(),
        }
    }

    /// The zone index, loading it if it isn't cached. If loading fails but an
    /// earlier load succeeded, returns that one.
    pub async fn index(&self) -> Result<Arc<Index>, Arc<Error>> {
        match self.index.try_get_with((), self.load_index()).await {
            Ok(index) => {
                *self.last_index.write().await = Some(index.clone());
                Ok(index)
            }
            Err(e) => match self.last_index.read().await.clone() {
                Some(stale) => {
                    tracing::warn!("failed to reload zone index, serving the previous one: {}", e);
                    Ok(stale)
                }
                None => Err(e),
            },
        }
    }

    async fn load_index(&self) -> Result<Arc<Index>, Error> {
        let countries = data_repo::fetch_countries(&self.client, &self.base_url).await?;

        // Owned inputs: borrowing closures here make the handler futures !Send.
        let codes: Vec<String> = countries.iter().map(|c| c.code.clone()).collect();
        let zone_lists: Vec<Result<Vec<Zone>, Error>> = stream::iter(codes)
            .map(|code| {
                let client = self.client.clone();
                let base_url = self.base_url.clone();
                async move { data_repo::fetch_zones(&client, &base_url, &code).await }
            })
            .buffered(ZONE_FETCH_CONCURRENCY)
            .collect()
            .await;
        let mut zones = Vec::new();
        for list in zone_lists {
            zones.extend(list?);
        }
        // Never cache an empty index: every zone lookup would 404 for a day.
        if zones.is_empty() {
            return Err("data source returned no zones".into());
        }

        tracing::info!(
            "loaded {} countries, {} zones",
            countries.len(),
            zones.len()
        );
        Ok(Arc::new(Index::new(countries, zones)))
    }

    async fn month(&self, zone: &Zone, year: i32, month: u32) -> Result<Month, Arc<Error>> {
        let key = (zone.country.clone(), zone.code.clone(), year, month);
        let fetched = self
            .months
            .try_get_with(key.clone(), async {
                let month: Month = Arc::new(
                    data_repo::fetch_prayer_times(
                        &self.client,
                        &self.base_url,
                        &zone.country,
                        &zone.code,
                        year,
                        month,
                    )
                    .await?,
                );
                self.stale_months.insert(key.clone(), month.clone()).await;
                Ok(month)
            })
            .await;

        match fetched {
            Ok(month) => Ok(month),
            Err(e) => match self.stale_months.get(&key).await {
                Some(stale) => {
                    tracing::warn!(
                        "failed to fetch {}/{} {}-{:02}, serving the previous copy: {}",
                        zone.country, zone.code, year, month, e
                    );
                    Ok(stale)
                }
                None => Err(e),
            },
        }
    }

    /// Prayer times for `zone` from `from` to `to` inclusive, sorted by date.
    /// Months not published yet are skipped, like dates missing from the DB were.
    pub async fn prayer_times(
        &self,
        zone: &Zone,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<PrayerTimeRecord>, Arc<Error>> {
        // All months at once: at most ~26 (the 750-day limit), and it keeps a
        // cold request to about one CDN round trip.
        let months: Vec<Month> = future::try_join_all(
            months_between(from, to)
                .into_iter()
                .map(|(year, month)| self.month(zone, year, month)),
        )
        .await?;

        let mut records: Vec<PrayerTimeRecord> = months
            .iter()
            .flat_map(|m| m.iter())
            .filter(|r| r.date >= from && r.date <= to)
            .cloned()
            .collect();
        // Stable sort, then keep the first record per date: the DB version
        // inserted with ON CONFLICT DO NOTHING.
        records.sort_by_key(|r| r.date);
        records.dedup_by_key(|r| r.date);
        Ok(records)
    }
}

/// (year, month) for every month touched by `from..=to`.
fn months_between(from: NaiveDate, to: NaiveDate) -> Vec<(i32, u32)> {
    let mut months = Vec::new();
    let (mut year, mut month) = (from.year(), from.month());
    while (year, month) <= (to.year(), to.month()) {
        months.push((year, month));
        if month == 12 {
            year += 1;
            month = 1;
        } else {
            month += 1;
        }
    }
    months
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use axum::{Router, extract::State, http::{StatusCode, Uri}};

    /// A fake CDN: one country (XX), one zone (XX01), one month (2026-01).
    #[derive(Default)]
    struct MockCdn {
        fail: AtomicBool,
        hits: AtomicUsize,
    }

    async fn serve(State(cdn): State<Arc<MockCdn>>, uri: Uri) -> (StatusCode, String) {
        cdn.hits.fetch_add(1, Ordering::SeqCst);
        if cdn.fail.load(Ordering::SeqCst) {
            return (StatusCode::INTERNAL_SERVER_ERROR, String::new());
        }
        let body = match uri.path() {
            "/countries.yaml" => "countries:\n  - {code: XX, name: X, source: X, geojson: g, mapping: m, shape_property: p}\n",
            "/zones/XX.yaml" => "zones:\n  - {code: XX01, country: XX, state: S, location: L, timezone: Asia/Kuala_Lumpur}\n",
            "/prayer-times/XX/XX01/2026-01.json" => r#"[{"date": "2026-01-01", "imsak": "05:50", "fajr": "06:00", "syuruk": "07:10", "dhuhr": "13:15", "asr": "16:40", "maghrib": "19:15", "isha": "20:30"}]"#,
            _ => return (StatusCode::NOT_FOUND, String::new()),
        };
        (StatusCode::OK, body.to_string())
    }

    async fn mock_cdn() -> (Arc<MockCdn>, String) {
        let cdn = Arc::new(MockCdn::default());
        let app = Router::new().fallback(serve).with_state(cdn.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        (cdn, url)
    }

    const TTL: Duration = Duration::from_millis(300);
    const LONG: Duration = Duration::from_secs(3600);

    async fn zone(store: &DataStore) -> Zone {
        store.index().await.unwrap().zone("XX01").unwrap().clone()
    }

    #[tokio::test]
    async fn test_found_month_is_fetched_once() {
        let (cdn, url) = mock_cdn().await;
        let store = DataStore::new(url, LONG, LONG, LONG);
        let z = zone(&store).await;
        let before = cdn.hits.load(Ordering::SeqCst);

        for _ in 0..3 {
            let records = store.prayer_times(&z, date(2026, 1, 1), date(2026, 1, 31)).await.unwrap();
            assert_eq!(records.len(), 1);
        }
        assert_eq!(cdn.hits.load(Ordering::SeqCst) - before, 1);
    }

    #[tokio::test]
    async fn test_missing_month_expires_on_missing_ttl() {
        let (cdn, url) = mock_cdn().await;
        let store = DataStore::new(url, LONG, LONG, TTL);
        let z = zone(&store).await;
        let before = cdn.hits.load(Ordering::SeqCst);

        let feb = || store.prayer_times(&z, date(2026, 2, 1), date(2026, 2, 28));
        assert!(feb().await.unwrap().is_empty());
        assert!(feb().await.unwrap().is_empty());
        assert_eq!(cdn.hits.load(Ordering::SeqCst) - before, 1, "404 is cached");

        tokio::time::sleep(TTL * 2).await;
        assert!(feb().await.unwrap().is_empty());
        assert_eq!(cdn.hits.load(Ordering::SeqCst) - before, 2, "404 refetched after missing TTL");
    }

    #[tokio::test]
    async fn test_failed_refetch_serves_stale_month() {
        let (cdn, url) = mock_cdn().await;
        let store = DataStore::new(url, LONG, TTL, TTL);
        let z = zone(&store).await;
        let jan = || store.prayer_times(&z, date(2026, 1, 1), date(2026, 1, 31));
        assert_eq!(jan().await.unwrap().len(), 1);

        cdn.fail.store(true, Ordering::SeqCst);
        tokio::time::sleep(TTL * 2).await;
        let before = cdn.hits.load(Ordering::SeqCst);
        assert_eq!(jan().await.unwrap().len(), 1, "stale copy served");
        assert!(cdn.hits.load(Ordering::SeqCst) > before, "refetch was attempted");
    }

    #[tokio::test]
    async fn test_failed_fetch_without_stale_copy_errors() {
        let (cdn, url) = mock_cdn().await;
        let store = DataStore::new(url, LONG, LONG, LONG);
        let z = zone(&store).await;

        cdn.fail.store(true, Ordering::SeqCst);
        assert!(store.prayer_times(&z, date(2026, 1, 1), date(2026, 1, 31)).await.is_err());
        // Errors aren't cached: once the CDN recovers, the next request works.
        cdn.fail.store(false, Ordering::SeqCst);
        assert_eq!(store.prayer_times(&z, date(2026, 1, 1), date(2026, 1, 31)).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_failed_index_reload_serves_stale_index() {
        let (cdn, url) = mock_cdn().await;
        let store = DataStore::new(url, TTL, LONG, LONG);
        assert!(store.index().await.unwrap().zone("XX01").is_some());

        cdn.fail.store(true, Ordering::SeqCst);
        tokio::time::sleep(TTL * 2).await;
        assert!(store.index().await.unwrap().zone("XX01").is_some());
    }

    #[tokio::test]
    async fn test_index_load_failure_without_stale_copy_errors() {
        let (cdn, url) = mock_cdn().await;
        cdn.fail.store(true, Ordering::SeqCst);
        let store = DataStore::new(url, LONG, LONG, LONG);
        assert!(store.index().await.is_err());
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn test_months_between_same_month() {
        assert_eq!(months_between(date(2026, 4, 1), date(2026, 4, 30)), vec![(2026, 4)]);
    }

    #[test]
    fn test_months_between_across_years() {
        assert_eq!(
            months_between(date(2026, 11, 15), date(2027, 2, 1)),
            vec![(2026, 11), (2026, 12), (2027, 1), (2027, 2)]
        );
    }

    #[test]
    fn test_months_between_empty_when_reversed() {
        assert!(months_between(date(2026, 5, 1), date(2026, 4, 1)).is_empty());
    }
}
