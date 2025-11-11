use std::time::Duration;

use chrono::{Datelike, Days, NaiveDate, Utc};
use chrono_tz::Asia::Kuala_Lumpur;
use diesel::PgConnection;
use tokio::time::sleep;

use crate::{
    api::jakim,
    models::{
        prayer_times::{UpsertPrayerTime, select_last_prayer_time_for_zone, upsert_prayer_times},
        zones::select_zones,
    },
};

pub async fn sync_prayer_times_for_zone(
    conn: &mut PgConnection,
    zone_code: &str,
    date_start: NaiveDate,
    date_end: NaiveDate,
) {
    let last_prayer_time = select_last_prayer_time_for_zone(conn, &zone_code);

    let date_start = match last_prayer_time {
        None => date_start,
        Some(pt) => {
            if pt.date >= date_end {
                println!(
                    "[sync_prayer_times_for_zone] skip for zone: {}, date_start: {}, date_end: {}",
                    zone_code, date_start, date_end
                );
                return;
            }

            if pt.date < date_start {
                date_start
            } else {
                pt.date.checked_add_days(Days::new(1)).unwrap()
            }
        }
    };

    println!(
        "[sync_prayer_times_from_jakim] Fetch for zone: {}, date_start: {}, date_end: {}",
        zone_code, date_start, date_end
    );
    let response = jakim::fetch_jakim_prayer_times(zone_code, date_start, date_end).await;
    let Ok(pts) = response else {
        println!(
            "[sync_prayer_times_from_jakim] Error for zone: {}, date_start: {}, date_end: {}",
            zone_code, date_start, date_end
        );
        return;
    };

    let prayer_times: Vec<UpsertPrayerTime> = pts
        .iter()
        .map(|jpt| UpsertPrayerTime {
            zone_code: zone_code.to_string(),
            date: NaiveDate::parse_from_str(&jpt.date, "%d-%b-%Y").unwrap(), // "08-Dec-2025"
            imsak: jpt.imsak.parse().unwrap(),
            fajr: jpt.fajr.parse().unwrap(),
            syuruk: jpt.syuruk.parse().unwrap(),
            dhuhr: jpt.dhuhr.parse().unwrap(),
            asr: jpt.asr.parse().unwrap(),
            maghrib: jpt.maghrib.parse().unwrap(),
            isha: jpt.isha.parse().unwrap(),
        })
        .collect();

    upsert_prayer_times(conn, &prayer_times);

    sleep(Duration::from_secs(2)).await;
}

pub async fn sync_prayer_times_for_year(conn: &mut PgConnection, year: i32) {
    let date_start = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
    let date_end = NaiveDate::from_ymd_opt(year, 12, 31).unwrap();

    let zones = select_zones(conn);
    for zone in zones.iter() {
        sync_prayer_times_for_zone(conn, &zone.zone_code, date_start, date_end).await;
    }
}

pub async fn sync_prayer_times_from_jakim(conn: &mut PgConnection) {
    let current_date = Utc::now().with_timezone(&Kuala_Lumpur).date_naive();
    sync_prayer_times_for_year(conn, current_date.year()).await;
    sync_prayer_times_for_year(conn, current_date.year() + 1).await;
}
