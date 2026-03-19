use diesel::PgConnection;

use crate::{
    api::muis,
    models::prayer_times::{UpsertPrayerTime, select_last_prayer_time_for_zone, upsert_prayer_times},
};

pub async fn sync(conn: &mut PgConnection) {
    let zone_code = "SGP01";

    tracing::info!("[sync_muis] fetching prayer times");
    let records = match crate::service::retry::with_retries(2, || {
        muis::fetch_muis_prayer_times()
    }).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("[sync_muis] fetch error: {:?}", e);
            return;
        }
    };

    let last_prayer_time = match select_last_prayer_time_for_zone(conn, zone_code) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("[sync_muis] db error looking up last prayer time: {}", e);
            return;
        }
    };

    let prayer_times: Vec<UpsertPrayerTime> = records
        .iter()
        .filter(|r| {
            if let Some(ref last) = last_prayer_time {
                r.date > last.date
            } else {
                true
            }
        })
        .map(|r| r.into())
        .collect();

    tracing::info!("[sync_muis] upserting {} prayer times for {}", prayer_times.len(), zone_code);
    if let Err(e) = upsert_prayer_times(conn, &prayer_times) {
        tracing::error!("[sync_muis] db error upserting: {}", e);
    }
}
