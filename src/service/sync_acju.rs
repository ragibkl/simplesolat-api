use chrono::{Datelike, Utc};
use diesel::PgConnection;

use crate::{
    api::acju,
    models::{
        prayer_times::{UpsertPrayerTime, select_last_prayer_time_for_zone, upsert_prayer_times},
        zones::select_zones_by_country,
    },
};

pub async fn sync(conn: &mut PgConnection) {
    let zones = match select_zones_by_country(conn, "LK") {
        Ok(z) => z,
        Err(e) => {
            tracing::error!("[sync_acju] db error loading LK zones: {}", e);
            return;
        }
    };

    let current_year = Utc::now()
        .with_timezone(&chrono_tz::Asia::Colombo)
        .date_naive()
        .year();

    tracing::info!("[sync_acju] syncing {} zones", zones.len());

    for zone in zones.iter() {
        let last_prayer_time = match select_last_prayer_time_for_zone(conn, &zone.zone_code) {
            Ok(v) => v,
            Err(e) => {
                tracing::error!("[sync_acju] db error for {}: {}", zone.zone_code, e);
                continue;
            }
        };

        for year in [current_year, current_year + 1] {
            let records = match acju::load_acju_prayer_times(&zone.zone_code, year) {
                Ok(r) => r,
                Err(e) => {
                    tracing::info!("[sync_acju] no data for zone {} year {}: {}", zone.zone_code, year, e);
                    continue;
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

            if prayer_times.is_empty() {
                tracing::info!("[sync_acju] skip zone {} year {} (already synced)", zone.zone_code, year);
                continue;
            }

            tracing::info!(
                "[sync_acju] upserting {} records for zone {} year {}",
                prayer_times.len(),
                zone.zone_code,
                year
            );
            if let Err(e) = upsert_prayer_times(conn, &prayer_times) {
                tracing::error!("[sync_acju] db error upserting for {}: {}", zone.zone_code, e);
            }
        }
    }

    tracing::info!("[sync_acju] done");
}
