use std::time::Duration;

use chrono::{Datelike, Months, NaiveDate, Utc};
use diesel::PgConnection;
use tokio::time::sleep;

use crate::{
    api::equran,
    models::{
        prayer_times::{UpsertPrayerTime, select_last_prayer_time_for_zone, upsert_prayer_times},
        zones::select_zones_by_country,
    },
};

fn add_month(date: NaiveDate) -> NaiveDate {
    date.checked_add_months(Months::new(1)).expect("date overflow adding 1 month")
}


pub async fn sync(conn: &mut PgConnection) {
    let zones = match select_zones_by_country(conn, "ID") {
        Ok(z) => z,
        Err(e) => {
            tracing::error!("[sync_equran] db error loading ID zones: {}", e);
            return;
        }
    };
    let current_date = Utc::now().with_timezone(&chrono_tz::Asia::Jakarta).date_naive();
    let end = NaiveDate::from_ymd_opt(current_date.year() + 1, 12, 31).expect("invalid year for end date");

    tracing::info!("[sync_equran] syncing {} zones", zones.len());

    for zone in zones.iter() {
        let last_prayer_time = match select_last_prayer_time_for_zone(conn, &zone.zone_code) {
            Ok(v) => v,
            Err(e) => {
                tracing::error!("[sync_equran] db error for {}: {}", zone.zone_code, e);
                continue;
            }
        };

        let mut cursor = match last_prayer_time {
            Some(ref last) => {
                let next = last.date + chrono::Duration::days(1);
                NaiveDate::from_ymd_opt(next.year(), next.month(), 1).expect("invalid date for sync cursor")
            }
            None => NaiveDate::from_ymd_opt(current_date.year(), current_date.month(), 1).expect("invalid current date for sync cursor"),
        };

        while cursor <= end {
            let year = cursor.year();
            let month = cursor.month() as i32;

            tracing::info!("[sync_equran] fetching zone {}, {}-{:02}", zone.zone_code, year, month);

            let records = match crate::service::retry::with_retries(5, || {
                equran::fetch_equran_prayer_times(
                    &zone.zone_code,
                    &zone.state,
                    &zone.location,
                    month,
                    year,
                )
            }).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!(
                        "[sync_equran] fetch error for zone {}, {}-{:02}: {:?}",
                        zone.zone_code, year, month, e
                    );
                    sleep(Duration::from_millis(200)).await;
                    cursor = add_month(cursor);
                    continue;
                }
            };

            // Empty month means no more data available (e.g. 2027 not published yet)
            if records.is_empty() {
                tracing::info!("[sync_equran] no data for zone {}, {}-{:02}, stopping", zone.zone_code, year, month);
                break;
            }

            let prayer_times: Vec<UpsertPrayerTime> = records
                .iter()
                .filter(|r| {
                    if let Some(ref last) = last_prayer_time {
                        r.tanggal_lengkap > last.date
                    } else {
                        true
                    }
                })
                .map(|r| r.into())
                .collect();

            if !prayer_times.is_empty() {
                if let Err(e) = upsert_prayer_times(conn, &prayer_times) {
                    tracing::error!("[sync_equran] db error upserting for {}: {}", zone.zone_code, e);
                }
            }

            sleep(Duration::from_millis(200)).await;
            cursor = add_month(cursor);
        }
    }

    tracing::info!("[sync_equran] done");
}
