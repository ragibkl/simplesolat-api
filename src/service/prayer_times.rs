use std::time::Duration;

use chrono::{Datelike, Days, NaiveDate, NaiveTime, Timelike, Utc};
use chrono_tz::Asia::Kuala_Lumpur;
use diesel::PgConnection;
use tokio::time::sleep;

use crate::{
    api::{equran, jakim, muis},
    models::{
        prayer_times::{UpsertPrayerTime, select_last_prayer_time_for_zone, upsert_prayer_times},
        zones::select_zones_by_country,
    },
};

pub async fn sync_prayer_times_for_zone(
    conn: &mut PgConnection,
    zone_code: &str,
    date_start: NaiveDate,
    date_end: NaiveDate,
) {
    let last_prayer_time = select_last_prayer_time_for_zone(conn, zone_code);

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

    let zones = select_zones_by_country(conn, "MY");
    for zone in zones.iter() {
        sync_prayer_times_for_zone(conn, &zone.zone_code, date_start, date_end).await;
    }
}

pub async fn sync_prayer_times_from_jakim(conn: &mut PgConnection) {
    let current_date = Utc::now().with_timezone(&Kuala_Lumpur).date_naive();
    sync_prayer_times_for_year(conn, current_date.year()).await;
    sync_prayer_times_for_year(conn, current_date.year() + 1).await;
}

/// Parse MUIS time string (12-hour format without AM/PM) to NaiveTime.
/// MUIS returns times like "01:10" for Zohor (1:10 PM) — PM times need +12 hours.
fn parse_muis_time(time_str: &str, is_pm: bool) -> Option<NaiveTime> {
    let t = NaiveTime::parse_from_str(time_str, "%H:%M").ok()?;
    if is_pm && t.hour() < 12 {
        Some(t + chrono::Duration::hours(12))
    } else {
        Some(t)
    }
}

pub async fn sync_prayer_times_from_muis(conn: &mut PgConnection) {
    let zone_code = "SGP01";

    println!("[sync_prayer_times_from_muis] Fetching MUIS prayer times");
    let records = match muis::fetch_muis_prayer_times().await {
        Ok(r) => r,
        Err(e) => {
            println!("[sync_prayer_times_from_muis] Error: {:?}", e);
            return;
        }
    };

    let last_prayer_time = select_last_prayer_time_for_zone(conn, zone_code);

    let prayer_times: Vec<UpsertPrayerTime> = records
        .iter()
        .filter_map(|r| {
            let date = NaiveDate::parse_from_str(&r.date, "%Y-%m-%d").ok()?;

            // Skip dates we already have
            if let Some(ref last) = last_prayer_time {
                if date <= last.date {
                    return None;
                }
            }

            // MUIS times are in 12-hour format without AM/PM indicators.
            // Subuh & Syuruk are AM; Zohor, Asar, Maghrib, Isyak are PM.
            let subuh = parse_muis_time(&r.subuh, false)?;
            let imsak = subuh - chrono::Duration::minutes(10);

            Some(UpsertPrayerTime {
                zone_code: zone_code.to_string(),
                date,
                imsak,
                fajr: subuh,
                syuruk: parse_muis_time(&r.syuruk, false)?,
                dhuhr: parse_muis_time(&r.zohor, true)?,
                asr: parse_muis_time(&r.asar, true)?,
                maghrib: parse_muis_time(&r.maghrib, true)?,
                isha: parse_muis_time(&r.isyak, true)?,
            })
        })
        .collect();

    println!(
        "[sync_prayer_times_from_muis] Upserting {} prayer times for {}",
        prayer_times.len(),
        zone_code
    );
    upsert_prayer_times(conn, &prayer_times);
}

pub async fn sync_prayer_times_from_equran(conn: &mut PgConnection) {
    let zones = select_zones_by_country(conn, "ID");
    let current_date = Utc::now().with_timezone(&chrono_tz::Asia::Jakarta).date_naive();
    let current_year = current_date.year();

    println!(
        "[sync_prayer_times_from_equran] Syncing {} zones for years {} and {}",
        zones.len(),
        current_year,
        current_year + 1
    );

    for zone in zones.iter() {
        let last_prayer_time = select_last_prayer_time_for_zone(conn, &zone.zone_code);

        for year in [current_year, current_year + 1] {
            let year_end = NaiveDate::from_ymd_opt(year, 12, 31).unwrap();

            // Skip this year if already fully synced
            if let Some(ref last) = last_prayer_time {
                if last.date >= year_end {
                    continue;
                }
            }

            // Determine which month to start from
            let start_month = if let Some(ref last) = last_prayer_time {
                if last.date.year() == year {
                    last.date.month() as i32
                } else {
                    1
                }
            } else {
                1
            };

            for month in start_month..=12 {
                println!(
                    "[sync_prayer_times_from_equran] Fetch zone: {}, {}-{:02}",
                    zone.zone_code, year, month
                );

                let records = match equran::fetch_equran_prayer_times(
                    &zone.state,
                    &zone.location,
                    month,
                    year,
                )
                .await
                {
                    Ok(r) => r,
                    Err(e) => {
                        println!(
                            "[sync_prayer_times_from_equran] Error for zone: {}, {}-{:02}: {:?}",
                            zone.zone_code, year, month, e
                        );
                        sleep(Duration::from_millis(200)).await;
                        continue;
                    }
                };

                let prayer_times: Vec<UpsertPrayerTime> = records
                    .iter()
                    .filter_map(|r| {
                        let date = NaiveDate::parse_from_str(&r.tanggal_lengkap, "%Y-%m-%d").ok()?;

                        // Skip dates we already have
                        if let Some(ref last) = last_prayer_time {
                            if date <= last.date {
                                return None;
                            }
                        }

                        Some(UpsertPrayerTime {
                            zone_code: zone.zone_code.to_string(),
                            date,
                            imsak: r.imsak.parse().ok()?,
                            fajr: r.subuh.parse().ok()?,
                            syuruk: r.terbit.parse().ok()?,
                            dhuhr: r.dzuhur.parse().ok()?,
                            asr: r.ashar.parse().ok()?,
                            maghrib: r.maghrib.parse().ok()?,
                            isha: r.isya.parse().ok()?,
                        })
                    })
                    .collect();

                if !prayer_times.is_empty() {
                    upsert_prayer_times(conn, &prayer_times);
                }

                sleep(Duration::from_millis(200)).await;
            }
        }
    }

    println!("[sync_prayer_times_from_equran] Done");
}
