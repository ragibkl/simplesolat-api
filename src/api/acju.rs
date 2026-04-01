use chrono::{NaiveDate, NaiveTime};
use std::fs;

pub fn city_for_zone(zone_code: &str) -> Option<&'static str> {
    match zone_code {
        "LK01" => Some("colombo"),
        "LK02" => Some("jaffna"),
        "LK03" => Some("mullaitivu"),
        "LK04" => Some("mannar"),
        "LK05" => Some("anuradhapura"),
        "LK06" => Some("kurunegala"),
        "LK07" => Some("kandy"),
        "LK08" => Some("batticaloa"),
        "LK09" => Some("trincomalee"),
        "LK10" => Some("badulla"),
        "LK11" => Some("ratnapura"),
        "LK12" => Some("galle"),
        "LK13" => Some("hambantota"),
        _ => None,
    }
}

/// Parsed ACJU prayer times.
/// No imsak provided upstream — derived as fajr - 10 min (same convention as MUIS).
pub struct AcjuPrayerTime {
    pub zone_code: String,
    pub imsak: NaiveTime,
    pub date: NaiveDate,
    pub fajr: NaiveTime,
    pub sunrise: NaiveTime,
    pub dhuhr: NaiveTime,
    pub asr: NaiveTime,
    pub maghrib: NaiveTime,
    pub isha: NaiveTime,
}

fn minutes_to_time(minutes: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(minutes / 60, minutes % 60, 0)
        .expect("invalid minutes value")
}

/// Load prayer times from local JSON file for a zone and year.
/// JSON format: 12 months x N days, each day is [fajr, sunrise, dhuhr, asr, maghrib, isha] in minutes from midnight.
pub fn load_acju_prayer_times(
    zone_code: &str,
    year: i32,
) -> Result<Vec<AcjuPrayerTime>, Box<dyn std::error::Error>> {
    let city = city_for_zone(zone_code)
        .ok_or_else(|| format!("unknown ACJU zone: {}", zone_code))?;

    let path = format!("data/acju/{}/acju.{}.json", year, city);
    let data = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {}", path, e))?;

    let months: Vec<Vec<[u32; 6]>> = serde_json::from_str(&data)?;

    let mut results = Vec::new();
    for (month_idx, days) in months.iter().enumerate() {
        let month = (month_idx + 1) as u32;
        for (day_idx, times) in days.iter().enumerate() {
            let day = (day_idx + 1) as u32;
            let date = match NaiveDate::from_ymd_opt(year, month, day) {
                Some(d) => d,
                None => continue, // skip invalid dates (e.g. Feb 30)
            };

            let fajr = minutes_to_time(times[0]);
            // ACJU doesn't provide imsak. Convention: imsak = fajr - 10 minutes.
            let imsak = fajr - chrono::Duration::minutes(10);

            results.push(AcjuPrayerTime {
                zone_code: zone_code.to_string(),
                imsak,
                date,
                fajr,
                sunrise: minutes_to_time(times[1]),
                dhuhr: minutes_to_time(times[2]),
                asr: minutes_to_time(times[3]),
                maghrib: minutes_to_time(times[4]),
                isha: minutes_to_time(times[5]),
            });
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minutes_to_time() {
        assert_eq!(minutes_to_time(300), NaiveTime::from_hms_opt(5, 0, 0).unwrap());
        assert_eq!(minutes_to_time(735), NaiveTime::from_hms_opt(12, 15, 0).unwrap());
        assert_eq!(minutes_to_time(1161), NaiveTime::from_hms_opt(19, 21, 0).unwrap());
    }

    #[test]
    fn test_load_acju_colombo() {
        let results = load_acju_prayer_times("LK01", 2026).unwrap();
        assert_eq!(results.len(), 365);
        assert_eq!(results[0].date, NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        assert_eq!(results[364].date, NaiveDate::from_ymd_opt(2026, 12, 31).unwrap());

        let pt = &results[0]; // Jan 1
        assert_eq!(pt.zone_code, "LK01");
        assert!(pt.fajr < pt.sunrise);
        assert!(pt.sunrise < pt.dhuhr);
        assert!(pt.dhuhr < pt.asr);
        assert!(pt.asr < pt.maghrib);
        assert!(pt.maghrib < pt.isha);
        assert_eq!(pt.imsak, pt.fajr - chrono::Duration::minutes(10));
    }

    #[test]
    fn test_city_for_zone() {
        assert_eq!(city_for_zone("LK01"), Some("colombo"));
        assert_eq!(city_for_zone("LK13"), Some("hambantota"));
        assert_eq!(city_for_zone("FAKE"), None);
    }
}
