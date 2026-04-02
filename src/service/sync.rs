use chrono::{Datelike, Months, NaiveDate, Utc};
use diesel::PgConnection;

use crate::{
    api::data_repo,
    models::{
        prayer_times::{self, select_last_prayer_time_for_zone, upsert_prayer_times},
        zones::{self, UpsertZone},
    },
};

fn add_month(date: NaiveDate) -> NaiveDate {
    date.checked_add_months(Months::new(1)).expect("date overflow adding 1 month")
}

/// Sync zones from the data repo for a specific country.
async fn sync_zones(
    client: &reqwest::Client,
    conn: &mut PgConnection,
    country_code: &str,
) -> Result<Vec<UpsertZone>, Box<dyn std::error::Error>> {
    let repo_zones = data_repo::fetch_zones(client, country_code).await?;
    let mut db_zones = Vec::new();
    for z in &repo_zones {
        let upsert: UpsertZone = z.into();
        if let Err(e) = zones::upsert_zone(conn, upsert) {
            tracing::error!("[sync] db error upserting zone {}: {}", z.code, e);
            continue;
        }
        db_zones.push(z.into());
    }
    tracing::info!("[sync] upserted {} zones for {}", db_zones.len(), country_code);
    Ok(db_zones)
}

/// Sync prayer times for a single zone from the data repo.
/// Sequential month-by-month fetch. Stops on first empty month (no more data available).
async fn sync_zone_prayer_times(
    client: &reqwest::Client,
    conn: &mut PgConnection,
    country_code: &str,
    zone: &UpsertZone,
    end: NaiveDate,
) {
    let last = match select_last_prayer_time_for_zone(conn, &zone.zone_code) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("[sync] db error for {}: {}", zone.zone_code, e);
            return;
        }
    };

    let now = Utc::now().date_naive();
    let mut cursor = match last {
        Some(ref last) => {
            let next = last.date + chrono::Duration::days(1);
            NaiveDate::from_ymd_opt(next.year(), next.month(), 1)
                .expect("invalid date for sync cursor")
        }
        None => NaiveDate::from_ymd_opt(now.year(), 1, 1)
            .expect("invalid current year start"),
    };

    while cursor <= end {
        let year = cursor.year();
        let month = cursor.month();

        let records = match data_repo::fetch_prayer_times(
            client,
            country_code,
            &zone.zone_code,
            year,
            month,
        )
        .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::error!(
                    "[sync] fetch error for {} {}-{:02}: {:?}",
                    zone.zone_code, year, month, e
                );
                break;
            }
        };

        // Empty month means no more data available
        if records.is_empty() {
            break;
        }

        let prayer_times: Vec<prayer_times::UpsertPrayerTime> = records
            .iter()
            .filter(|r| {
                if let Some(ref last) = last {
                    r.date > last.date
                } else {
                    true
                }
            })
            .map(|r| prayer_times::to_upsert(&zone.zone_code, r))
            .collect();

        if !prayer_times.is_empty() {
            tracing::info!(
                "[sync] upserting {} records for {}",
                prayer_times.len(),
                zone.zone_code
            );
            if let Err(e) = upsert_prayer_times(conn, &prayer_times) {
                tracing::error!("[sync] db error upserting for {}: {}", zone.zone_code, e);
            }
        }

        cursor = add_month(cursor);
    }
}

/// Sync all prayer times for a country from the data repo.
pub async fn sync_country(conn: &mut PgConnection, country_code: &str) {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("failed to build HTTP client");

    let now = Utc::now().date_naive();
    let end = NaiveDate::from_ymd_opt(now.year() + 1, 12, 31)
        .expect("invalid year for end date");

    // Sync zones first
    let zones = match sync_zones(&client, conn, country_code).await {
        Ok(z) => z,
        Err(e) => {
            tracing::error!("[sync] failed to fetch zones for {}: {:?}", country_code, e);
            return;
        }
    };

    tracing::info!("[sync] syncing prayer times for {} ({} zones)", country_code, zones.len());

    for zone in &zones {
        sync_zone_prayer_times(&client, conn, &zone.country, zone, end).await;
    }

    tracing::info!("[sync] done for {}", country_code);
}

/// Sync all countries from the data repo.
pub async fn sync_all(conn: &mut PgConnection) {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("failed to build HTTP client");

    let countries = match data_repo::fetch_countries(&client).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("[sync] failed to fetch countries: {:?}", e);
            return;
        }
    };

    tracing::info!("[sync] found {} countries", countries.len());

    for country in &countries {
        sync_country(conn, &country.code).await;
    }

    tracing::info!("[sync] all done");
}
