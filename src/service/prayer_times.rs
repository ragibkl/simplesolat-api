use std::time::Duration;

use chrono::{Datelike, Days, NaiveDate, NaiveTime, Timelike, Utc};
use chrono_tz::Asia::Kuala_Lumpur;
use diesel::PgConnection;
use tokio::time::sleep;

use crate::{
    api::{equran, jakim, kheu, muis},
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
                tracing::info!(
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

    tracing::info!(
        "[sync_prayer_times_from_jakim] Fetch for zone: {}, date_start: {}, date_end: {}",
        zone_code, date_start, date_end
    );
    let response = jakim::fetch_jakim_prayer_times(zone_code, date_start, date_end).await;
    let Ok(pts) = response else {
        tracing::info!(
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

    tracing::info!("[sync_prayer_times_from_muis] Fetching MUIS prayer times");
    let records = match muis::fetch_muis_prayer_times().await {
        Ok(r) => r,
        Err(e) => {
            tracing::info!("[sync_prayer_times_from_muis] Error: {:?}", e);
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

    tracing::info!(
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

    tracing::info!(
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
                tracing::info!(
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
                        tracing::info!(
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

    tracing::info!("[sync_prayer_times_from_equran] Done");
}

/// Parse KHEU time string (dot-separated, 12-hour without AM/PM) to NaiveTime.
/// KHEU returns times like "5.04" for Imsak (5:04 AM), "3.50" for Asar (3:50 PM).
/// Suboh & Syuruk & Imsak are AM; Zohor, Asar, Maghrib, Isyak are PM.
fn parse_kheu_time(time_str: &str, is_pm: bool) -> Option<NaiveTime> {
    let normalized = time_str.replace('.', ":");
    let t = NaiveTime::parse_from_str(&normalized, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(&normalized, "%-H:%M"))
        .ok()?;
    if is_pm && t.hour() < 12 {
        Some(t + chrono::Duration::hours(12))
    } else {
        Some(t)
    }
}

/// Returns the minute offset for a Brunei zone.
/// KHEU base times are for Brunei-Muara. Offsets:
/// - BRN01 (Brunei-Muara): 0 min
/// - BRN02 (Tutong): +1 min
/// - BRN03 (Belait): +3 min
/// - BRN04 (Temburong): 0 min
fn brunei_zone_offset(zone_code: &str) -> i64 {
    match zone_code {
        "BRN02" => 1,
        "BRN03" => 3,
        _ => 0,
    }
}

fn apply_offset(time: NaiveTime, offset_minutes: i64) -> NaiveTime {
    if offset_minutes == 0 {
        return time;
    }
    time + chrono::Duration::minutes(offset_minutes)
}

pub async fn sync_prayer_times_from_kheu(conn: &mut PgConnection) {
    let zones = select_zones_by_country(conn, "BN");
    let current_date = Utc::now().with_timezone(&Kuala_Lumpur).date_naive();
    let current_year = current_date.year();

    // Check base zone (BRN01) to determine which months need syncing
    let last_base = select_last_prayer_time_for_zone(conn, "BRN01");

    tracing::info!(
        "[sync_prayer_times_from_kheu] Syncing {} zones for years {} and {}",
        zones.len(),
        current_year,
        current_year + 1
    );

    // Fetch base times once per month, then apply offsets per zone
    for year in [current_year, current_year + 1] {
        let year_end = NaiveDate::from_ymd_opt(year, 12, 31).unwrap();

        // Skip this year entirely if base zone is already fully synced
        if let Some(ref last) = last_base {
            if last.date >= year_end {
                tracing::info!(
                    "[sync_prayer_times_from_kheu] Skip year {} (already synced)",
                    year
                );
                continue;
            }
        }

        // Start from the month after the last synced data
        let start_month = if let Some(ref last) = last_base {
            if last.date.year() == year {
                last.date.month()
            } else {
                1
            }
        } else {
            1
        };

        for month in start_month..=12u32 {
            tracing::info!(
                "[sync_prayer_times_from_kheu] Fetch {}-{:02}",
                year, month
            );

            let records = match kheu::fetch_kheu_prayer_times(year, month).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::info!(
                        "[sync_prayer_times_from_kheu] Error for {}-{:02}: {:?}",
                        year, month, e
                    );
                    continue;
                }
            };

            for zone in zones.iter() {
                let offset = brunei_zone_offset(&zone.zone_code);
                let last_prayer_time = select_last_prayer_time_for_zone(conn, &zone.zone_code);

                let prayer_times: Vec<UpsertPrayerTime> = records
                    .iter()
                    .filter_map(|r| {
                        // KHEU date is ISO with UTC offset: "2026-03-01T16:00:00Z" = midnight BN time
                        let date = NaiveDate::parse_from_str(&r.date[..10], "%Y-%m-%d")
                            .ok()?
                            .checked_add_days(Days::new(1))?;

                        // Skip dates we already have
                        if let Some(ref last) = last_prayer_time {
                            if date <= last.date {
                                return None;
                            }
                        }

                        Some(UpsertPrayerTime {
                            zone_code: zone.zone_code.to_string(),
                            date,
                            imsak: apply_offset(parse_kheu_time(&r.imsak, false)?, offset),
                            fajr: apply_offset(parse_kheu_time(&r.suboh, false)?, offset),
                            syuruk: apply_offset(parse_kheu_time(&r.syuruk, false)?, offset),
                            dhuhr: apply_offset(parse_kheu_time(&r.zohor, true)?, offset),
                            asr: apply_offset(parse_kheu_time(&r.asar, true)?, offset),
                            maghrib: apply_offset(parse_kheu_time(&r.maghrib, true)?, offset),
                            isha: apply_offset(parse_kheu_time(&r.isyak, true)?, offset),
                        })
                    })
                    .collect();

                if !prayer_times.is_empty() {
                    tracing::info!(
                        "[sync_prayer_times_from_kheu] Upserting {} records for {}",
                        prayer_times.len(),
                        zone.zone_code
                    );
                    upsert_prayer_times(conn, &prayer_times);
                }
            }

            sleep(Duration::from_millis(500)).await;
        }
    }

    tracing::info!("[sync_prayer_times_from_kheu] Done");
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;

    // -- parse_muis_time tests (real upstream samples from data.gov.sg) --

    #[test]
    fn test_parse_muis_time_subuh_am() {
        // Subuh "05:44" -> 05:44 (AM, no adjustment)
        let t = parse_muis_time("05:44", false).unwrap();
        assert_eq!(t, NaiveTime::from_hms_opt(5, 44, 0).unwrap());
    }

    #[test]
    fn test_parse_muis_time_zohor_pm_low_hour() {
        // Zohor "01:00" -> 13:00 (PM, add 12)
        let t = parse_muis_time("01:00", true).unwrap();
        assert_eq!(t, NaiveTime::from_hms_opt(13, 0, 0).unwrap());
    }

    #[test]
    fn test_parse_muis_time_zohor_pm_noon() {
        // Zohor "12:59" -> 12:59 (PM but hour already 12, no adjustment)
        let t = parse_muis_time("12:59", true).unwrap();
        assert_eq!(t, NaiveTime::from_hms_opt(12, 59, 0).unwrap());
    }

    #[test]
    fn test_parse_muis_time_isyak_pm() {
        // Isyak "08:02" -> 20:02
        let t = parse_muis_time("08:02", true).unwrap();
        assert_eq!(t, NaiveTime::from_hms_opt(20, 2, 0).unwrap());
    }

    // -- parse_kheu_time tests (real upstream samples from mora.gov.bn) --

    #[test]
    fn test_parse_kheu_time_imsak_am() {
        // Imsak "5.04" -> 05:04
        let t = parse_kheu_time("5.04", false).unwrap();
        assert_eq!(t, NaiveTime::from_hms_opt(5, 4, 0).unwrap());
    }

    #[test]
    fn test_parse_kheu_time_asar_pm() {
        // Asar "3.50" -> 15:50
        let t = parse_kheu_time("3.50", true).unwrap();
        assert_eq!(t, NaiveTime::from_hms_opt(15, 50, 0).unwrap());
    }

    #[test]
    fn test_parse_kheu_time_zohor_pm_noon() {
        // Zohor "12.25" -> 12:25 (PM but hour already 12)
        let t = parse_kheu_time("12.25", true).unwrap();
        assert_eq!(t, NaiveTime::from_hms_opt(12, 25, 0).unwrap());
    }

    #[test]
    fn test_parse_kheu_time_maghrib_pm() {
        // Maghrib "6.33" -> 18:33
        let t = parse_kheu_time("6.33", true).unwrap();
        assert_eq!(t, NaiveTime::from_hms_opt(18, 33, 0).unwrap());
    }

    // -- brunei_zone_offset tests --

    #[test]
    fn test_brunei_zone_offsets() {
        assert_eq!(brunei_zone_offset("BRN01"), 0);
        assert_eq!(brunei_zone_offset("BRN02"), 1);
        assert_eq!(brunei_zone_offset("BRN03"), 3);
        assert_eq!(brunei_zone_offset("BRN04"), 0);
    }

    // -- apply_offset tests --

    #[test]
    fn test_apply_offset_zero() {
        let t = NaiveTime::from_hms_opt(5, 4, 0).unwrap();
        assert_eq!(apply_offset(t, 0), t);
    }

    #[test]
    fn test_apply_offset_one_minute() {
        let t = NaiveTime::from_hms_opt(5, 4, 0).unwrap();
        assert_eq!(
            apply_offset(t, 1),
            NaiveTime::from_hms_opt(5, 5, 0).unwrap()
        );
    }

    #[test]
    fn test_apply_offset_three_minutes() {
        let t = NaiveTime::from_hms_opt(18, 33, 0).unwrap();
        assert_eq!(
            apply_offset(t, 3),
            NaiveTime::from_hms_opt(18, 36, 0).unwrap()
        );
    }
}

#[cfg(test)]
mod timezone_tests {
    use crate::models::zones::UpsertZone;

    fn zone(country: &str, state: &str) -> UpsertZone {
        UpsertZone {
            zone_code: "TEST01".to_string(),
            country: country.to_string(),
            state: state.to_string(),
            location: "Test".to_string(),
        }
    }

    #[test]
    fn test_malaysia_timezone() {
        assert_eq!(zone("MY", "Selangor").timezone(), chrono_tz::Asia::Kuala_Lumpur);
    }

    #[test]
    fn test_singapore_timezone() {
        assert_eq!(zone("SG", "Singapore").timezone(), chrono_tz::Asia::Kuala_Lumpur);
    }

    #[test]
    fn test_brunei_timezone() {
        assert_eq!(zone("BN", "Brunei-Muara").timezone(), chrono_tz::Asia::Kuala_Lumpur);
    }

    #[test]
    fn test_indonesia_wib() {
        assert_eq!(zone("ID", "Aceh").timezone(), chrono_tz::Asia::Jakarta);
        assert_eq!(zone("ID", "DKI Jakarta").timezone(), chrono_tz::Asia::Jakarta);
        assert_eq!(zone("ID", "Jawa Barat").timezone(), chrono_tz::Asia::Jakarta);
    }

    #[test]
    fn test_indonesia_wita() {
        assert_eq!(zone("ID", "Bali").timezone(), chrono_tz::Asia::Makassar);
        assert_eq!(zone("ID", "Sulawesi Selatan").timezone(), chrono_tz::Asia::Makassar);
        assert_eq!(zone("ID", "Kalimantan Timur").timezone(), chrono_tz::Asia::Makassar);
    }

    #[test]
    fn test_indonesia_wit() {
        assert_eq!(zone("ID", "Papua").timezone(), chrono_tz::Asia::Jayapura);
        assert_eq!(zone("ID", "Maluku").timezone(), chrono_tz::Asia::Jayapura);
    }
}
