//! Calculated prayer times for places without an official timetable, the
//! same way the app does it (simplesolat lib/domain/adhanCalculator.ts):
//! the country's method from simplesolat-data's methods.yaml, Shafi'i Asr,
//! imsak 10 minutes before fajr, no dhuha. salah is a Rust port of adhan, the
//! library the app uses, and gives the same times.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, NaiveTime, Offset, TimeZone, Utc};
use chrono_tz::Tz;
use salah::{Configuration, Coordinates, Madhab, Parameters, PrayerSchedule, Prayer, TimeAdjustment};
use serde::Deserialize;

/// methods.yaml: methods by name, the method of each country, the default.
#[derive(Debug, Deserialize, Clone)]
pub struct MethodsDoc {
    pub default: String,
    pub methods: HashMap<String, MethodDef>,
    pub countries: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MethodDef {
    pub label: String,
    pub fajr_angle: f64,
    pub isha_angle: Option<f64>,
    pub isha_interval: Option<i32>,
    pub maghrib_angle: Option<f64>,
    #[serde(default)]
    pub adjustments: HashMap<String, i64>,
    pub rounding: Option<String>,
}

impl MethodsDoc {
    /// The method for a country (ISO 3166-1 alpha-2), or the default.
    pub fn method_for(&self, country: Option<&str>) -> (&str, &MethodDef) {
        let name = country
            .and_then(|c| self.countries.get(c))
            .filter(|n| self.methods.contains_key(n.as_str()))
            .unwrap_or(&self.default);
        (name.as_str(), &self.methods[name])
    }
}

fn parameters(def: &MethodDef) -> Parameters {
    let adj = |key: &str| def.adjustments.get(key).copied().unwrap_or(0);
    let mut config = Configuration::new(def.fajr_angle, def.isha_angle.unwrap_or(0.0));
    config.madhab(Madhab::Shafi).method_adjustments(TimeAdjustment::new(
        adj("fajr"),
        adj("sunrise"),
        adj("dhuhr"),
        adj("asr"),
        adj("maghrib"),
        adj("isha"),
    ));
    // Only when set: salah's isha_interval() also zeroes the isha angle.
    if let Some(minutes) = def.isha_interval.filter(|m| *m > 0) {
        config.isha_interval(minutes);
    }
    // salah keeps maghrib_angle but computes maghrib at sunset regardless:
    // only Tehran's method sets one (Iran, 4.5°), where maghrib comes out at
    // sunset instead of a few minutes later.
    if let Some(angle) = def.maghrib_angle {
        config.maghrib_angle(angle);
    }
    // salah doesn't export its rounding setting, so "rounding: up" (only the
    // Singapore method, which no calculated country uses: Singapore is
    // official) is rounded to the nearest minute like the rest.
    config.done()
}

/// One calculated day, in the place's local time.
pub struct CalculatedDay {
    pub date: NaiveDate,
    pub utc_offset: String,
    pub imsak: NaiveTime,
    pub fajr: NaiveTime,
    pub syuruk: NaiveTime,
    pub dhuhr: NaiveTime,
    pub asr: NaiveTime,
    pub maghrib: NaiveTime,
    pub isha: NaiveTime,
}

/// "+08:00" for the zone's offset on that date (changes with daylight saving).
pub fn utc_offset(tz: Tz, date: NaiveDate) -> String {
    let noon = date.and_hms_opt(12, 0, 0).expect("valid time");
    let seconds = tz.from_utc_datetime(&noon).offset().fix().local_minus_utc();
    let sign = if seconds < 0 { '-' } else { '+' };
    let minutes = seconds.abs() / 60;
    format!("{sign}{:02}:{:02}", minutes / 60, minutes % 60)
}

pub fn calculate(lat: f64, lng: f64, date: NaiveDate, tz: Tz, def: &MethodDef) -> Option<CalculatedDay> {
    let times = PrayerSchedule::new()
        .on(date)
        .for_location(Coordinates::new(lat, lng))
        .with_configuration(parameters(def))
        .calculate()
        .ok()?;
    let local = |t: DateTime<Utc>| t.with_timezone(&tz).time();
    let fajr = times.time(Prayer::Fajr);
    Some(CalculatedDay {
        date,
        utc_offset: utc_offset(tz, date),
        imsak: local(fajr - chrono::Duration::minutes(10)),
        fajr: local(fajr),
        syuruk: local(times.time(Prayer::Sunrise)),
        dhuhr: local(times.time(Prayer::Dhuhr)),
        asr: local(times.time(Prayer::Asr)),
        maghrib: local(times.time(Prayer::Maghrib)),
        isha: local(times.time(Prayer::Isha)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mwl() -> MethodDef {
        MethodDef {
            label: "Muslim World League".into(),
            fajr_angle: 18.0,
            isha_angle: Some(17.0),
            isha_interval: None,
            maghrib_angle: None,
            adjustments: HashMap::from([("dhuhr".to_string(), 1)]),
            rounding: None,
        }
    }

    #[test]
    fn test_offsets_follow_daylight_saving() {
        let nz: Tz = "Pacific/Auckland".parse().unwrap();
        assert_eq!(utc_offset(nz, NaiveDate::from_ymd_opt(2026, 11, 10).unwrap()), "+13:00");
        assert_eq!(utc_offset(nz, NaiveDate::from_ymd_opt(2026, 6, 10).unwrap()), "+12:00");
        let ny: Tz = "America/New_York".parse().unwrap();
        assert_eq!(utc_offset(ny, NaiveDate::from_ymd_opt(2026, 1, 10).unwrap()), "-05:00");
    }

    #[test]
    fn test_times_in_order_and_imsak_ten_minutes_before_fajr() {
        let tz: Tz = "Pacific/Auckland".parse().unwrap();
        let d = calculate(-36.8485, 174.7633, NaiveDate::from_ymd_opt(2026, 11, 10).unwrap(), tz, &mwl()).unwrap();
        assert!(d.imsak < d.fajr && d.fajr < d.syuruk && d.syuruk < d.dhuhr);
        assert!(d.dhuhr < d.asr && d.asr < d.maghrib && d.maghrib < d.isha);
        assert_eq!((d.fajr - d.imsak).num_minutes(), 10);
    }
}
