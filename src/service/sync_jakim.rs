use std::time::Duration;

use chrono::{Datelike, Days, NaiveDate, Utc};
use chrono_tz::Asia::Kuala_Lumpur;
use diesel::PgConnection;
use tokio::time::sleep;

use crate::{
    api::jakim,
    models::{
        prayer_times::{UpsertPrayerTime, select_last_prayer_time_for_zone, upsert_prayer_times},
        zones::select_zones_by_country,
    },
};

async fn sync_zone(
    conn: &mut PgConnection,
    zone_code: &str,
    date_start: NaiveDate,
    date_end: NaiveDate,
) {
    let last_prayer_time = match select_last_prayer_time_for_zone(conn, zone_code) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("[sync_jakim] db error looking up last prayer time for {}: {}", zone_code, e);
            return;
        }
    };

    let date_start = match last_prayer_time {
        None => date_start,
        Some(pt) => {
            if pt.date >= date_end {
                tracing::info!("[sync_jakim] skipping zone {}, already synced to {}", zone_code, date_end);
                return;
            }

            if pt.date < date_start {
                date_start
            } else {
                pt.date.checked_add_days(Days::new(1)).expect("date overflow adding 1 day")
            }
        }
    };

    tracing::info!("[sync_jakim] fetching zone {}, {} to {}", zone_code, date_start, date_end);
    let records = match crate::service::retry::with_retries(2, || {
        jakim::fetch_jakim_prayer_times(zone_code, date_start, date_end)
    }).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("[sync_jakim] fetch error for zone {}: {:?}", zone_code, e);
            return;
        }
    };

    let prayer_times: Vec<UpsertPrayerTime> = records.iter().map(|r| r.into()).collect();

    if let Err(e) = upsert_prayer_times(conn, &prayer_times) {
        tracing::error!("[sync_jakim] db error upserting for {}: {}", zone_code, e);
    }

    sleep(Duration::from_secs(1)).await;
}

async fn sync_year(conn: &mut PgConnection, year: i32) {
    let date_start = NaiveDate::from_ymd_opt(year, 1, 1).expect("invalid year for date_start");
    let date_end = NaiveDate::from_ymd_opt(year, 12, 31).expect("invalid year for date_end");

    let zones = match select_zones_by_country(conn, "MY") {
        Ok(z) => z,
        Err(e) => {
            tracing::error!("[sync_jakim] db error loading MY zones: {}", e);
            return;
        }
    };
    for zone in zones.iter() {
        sync_zone(conn, &zone.zone_code, date_start, date_end).await;
    }
}

pub async fn sync(conn: &mut PgConnection) {
    let current_date = Utc::now().with_timezone(&Kuala_Lumpur).date_naive();
    sync_year(conn, current_date.year()).await;
    sync_year(conn, current_date.year() + 1).await;
}
