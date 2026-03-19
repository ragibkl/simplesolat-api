use chrono::{Datelike, Days, NaiveDate, Utc};
use chrono_tz::Asia::Kuala_Lumpur;
use diesel::PgConnection;

use crate::{
    api::kheu,
    models::prayer_times::{UpsertPrayerTime, select_last_prayer_time_for_zone, upsert_prayer_times},
};

pub async fn sync(conn: &mut PgConnection) {
    let current_date = Utc::now().with_timezone(&Kuala_Lumpur).date_naive();

    let last_base = match select_last_prayer_time_for_zone(conn, "BRN01") {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("[sync_kheu] db error looking up BRN01: {}", e);
            return;
        }
    };

    let date_start = match last_base {
        Some(ref last) => last.date.checked_add_days(Days::new(1)).expect("date overflow adding 1 day"),
        None => NaiveDate::from_ymd_opt(current_date.year(), 1, 1).expect("invalid year for date_start"),
    };
    let date_end = NaiveDate::from_ymd_opt(current_date.year() + 1, 12, 31).expect("invalid year for date_end");

    if date_start > date_end {
        tracing::info!("[sync_kheu] already synced to {}", date_end);
        return;
    }

    tracing::info!("[sync_kheu] fetching {} to {}", date_start, date_end);

    let records = match kheu::fetch_kheu_prayer_times(date_start, date_end).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("[sync_kheu] fetch error: {:?}", e);
            return;
        }
    };

    let prayer_times: Vec<UpsertPrayerTime> = records.iter().map(|r| r.into()).collect();

    if !prayer_times.is_empty() {
        tracing::info!("[sync_kheu] upserting {} records", prayer_times.len());
        if let Err(e) = upsert_prayer_times(conn, &prayer_times) {
            tracing::error!("[sync_kheu] db error upserting: {}", e);
        }
    }

    tracing::info!("[sync_kheu] done");
}
