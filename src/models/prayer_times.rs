use chrono::{NaiveDate, NaiveTime};
use diesel::prelude::*;

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::prayer_times)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct SelectPrayerTime {
    pub id: i64,
    pub zone_code: String,
    pub date: NaiveDate,
    pub imsak: NaiveTime,
    pub fajr: NaiveTime,
    pub syuruk: NaiveTime,
    pub dhuhr: NaiveTime,
    pub asr: NaiveTime,
    pub maghrib: NaiveTime,
    pub isha: NaiveTime,
}

#[derive(Insertable, AsChangeset)]
#[diesel(table_name = crate::schema::prayer_times)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UpsertPrayerTime {
    pub zone_code: String,
    pub date: NaiveDate,
    pub imsak: NaiveTime,
    pub fajr: NaiveTime,
    pub syuruk: NaiveTime,
    pub dhuhr: NaiveTime,
    pub asr: NaiveTime,
    pub maghrib: NaiveTime,
    pub isha: NaiveTime,
}

impl From<&crate::api::jakim::JakimPrayerTime> for UpsertPrayerTime {
    fn from(r: &crate::api::jakim::JakimPrayerTime) -> Self {
        Self {
            zone_code: r.zone_code.to_string(),
            date: r.date,
            imsak: r.imsak,
            fajr: r.fajr,
            syuruk: r.syuruk,
            dhuhr: r.dhuhr,
            asr: r.asr,
            maghrib: r.maghrib,
            isha: r.isha,
        }
    }
}

impl From<&crate::api::muis::MuisPrayerTime> for UpsertPrayerTime {
    fn from(r: &crate::api::muis::MuisPrayerTime) -> Self {
        Self {
            zone_code: r.zone_code.to_string(),
            date: r.date,
            imsak: r.imsak,
            fajr: r.subuh,
            syuruk: r.syuruk,
            dhuhr: r.zohor,
            asr: r.asar,
            maghrib: r.maghrib,
            isha: r.isyak,
        }
    }
}

impl From<&crate::api::equran::EquranPrayerTime> for UpsertPrayerTime {
    fn from(r: &crate::api::equran::EquranPrayerTime) -> Self {
        Self {
            zone_code: r.zone_code.to_string(),
            date: r.tanggal_lengkap,
            imsak: r.imsak,
            fajr: r.subuh,
            syuruk: r.terbit,
            dhuhr: r.dzuhur,
            asr: r.ashar,
            maghrib: r.maghrib,
            isha: r.isya,
        }
    }
}

impl From<&crate::api::kheu::KheuPrayerTime> for UpsertPrayerTime {
    fn from(r: &crate::api::kheu::KheuPrayerTime) -> Self {
        Self {
            zone_code: r.zone_code.to_string(),
            date: r.date,
            imsak: r.imsak,
            fajr: r.suboh,
            syuruk: r.syuruk,
            dhuhr: r.zohor,
            asr: r.asar,
            maghrib: r.maghrib,
            isha: r.isyak,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;

    fn time(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    fn date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 3, 19).unwrap()
    }

    #[test]
    fn test_from_jakim() {
        use crate::api::jakim::JakimPrayerTime;
        let src = JakimPrayerTime {
            zone_code: "SGR01".to_string(),
            date: date(),
            imsak: time(6, 0), fajr: time(6, 10), syuruk: time(7, 16),
            dhuhr: time(13, 24), asr: time(16, 29), maghrib: time(19, 26), isha: time(20, 35),
        };
        let dst = UpsertPrayerTime::from(&src);
        assert_eq!(dst.zone_code, "SGR01");
        assert_eq!(dst.date, date());
        assert_eq!(dst.imsak, time(6, 0));
        assert_eq!(dst.fajr, time(6, 10));
        assert_eq!(dst.syuruk, time(7, 16));
        assert_eq!(dst.dhuhr, time(13, 24));
        assert_eq!(dst.asr, time(16, 29));
        assert_eq!(dst.maghrib, time(19, 26));
        assert_eq!(dst.isha, time(20, 35));
    }

    #[test]
    fn test_from_muis() {
        use crate::api::muis::MuisPrayerTime;
        let src = MuisPrayerTime {
            zone_code: "SGP01".to_string(),
            date: date(),
            imsak: time(5, 34), subuh: time(5, 44), syuruk: time(7, 7),
            zohor: time(13, 10), asar: time(16, 34), maghrib: time(19, 10), isyak: time(20, 25),
        };
        let dst = UpsertPrayerTime::from(&src);
        assert_eq!(dst.zone_code, "SGP01");
        assert_eq!(dst.fajr, time(5, 44));   // subuh -> fajr
        assert_eq!(dst.dhuhr, time(13, 10));  // zohor -> dhuhr
        assert_eq!(dst.asr, time(16, 34));    // asar -> asr
        assert_eq!(dst.isha, time(20, 25));   // isyak -> isha
    }

    #[test]
    fn test_from_equran() {
        use crate::api::equran::EquranPrayerTime;
        let src = EquranPrayerTime {
            zone_code: "ACH01".to_string(),
            tanggal_lengkap: date(),
            imsak: time(5, 24), subuh: time(5, 34), terbit: time(6, 46),
            dzuhur: time(12, 53), ashar: time(16, 10), maghrib: time(18, 54), isya: time(20, 2),
        };
        let dst = UpsertPrayerTime::from(&src);
        assert_eq!(dst.zone_code, "ACH01");
        assert_eq!(dst.date, date());         // tanggal_lengkap -> date
        assert_eq!(dst.fajr, time(5, 34));    // subuh -> fajr
        assert_eq!(dst.syuruk, time(6, 46));  // terbit -> syuruk
        assert_eq!(dst.dhuhr, time(12, 53));  // dzuhur -> dhuhr
        assert_eq!(dst.asr, time(16, 10));    // ashar -> asr
        assert_eq!(dst.isha, time(20, 2));    // isya -> isha
    }

    #[test]
    fn test_from_kheu() {
        use crate::api::kheu::KheuPrayerTime;
        let src = KheuPrayerTime {
            zone_code: "BRN01".to_string(),
            date: date(),
            imsak: time(5, 4), suboh: time(5, 14), syuruk: time(6, 32),
            zohor: time(12, 34), asar: time(15, 50), maghrib: time(18, 33), isyak: time(19, 43),
        };
        let dst = UpsertPrayerTime::from(&src);
        assert_eq!(dst.zone_code, "BRN01");
        assert_eq!(dst.fajr, time(5, 14));    // suboh -> fajr
        assert_eq!(dst.dhuhr, time(12, 34));  // zohor -> dhuhr
        assert_eq!(dst.asr, time(15, 50));    // asar -> asr
        assert_eq!(dst.isha, time(19, 43));   // isyak -> isha
    }
}

pub fn select_last_prayer_time_for_zone(
    conn: &mut PgConnection,
    zone_code: &str,
) -> Result<Option<SelectPrayerTime>, diesel::result::Error> {
    use crate::schema::prayer_times;

    prayer_times::table
        .filter(prayer_times::zone_code.eq(zone_code))
        .select(SelectPrayerTime::as_select())
        .order(prayer_times::date.desc())
        .first(conn)
        .optional()
}

pub fn upsert_prayer_times(
    conn: &mut PgConnection,
    prayer_times: &[UpsertPrayerTime],
) -> Result<(), diesel::result::Error> {
    use crate::schema::prayer_times;

    diesel::insert_into(prayer_times::table)
        .values(prayer_times)
        .on_conflict((prayer_times::zone_code, prayer_times::date))
        .do_nothing()
        .execute(conn)?;
    Ok(())
}

pub fn select_prayer_times_for_zone(
    conn: &mut PgConnection,
    zone_code: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<SelectPrayerTime>, diesel::result::Error> {
    use crate::schema::prayer_times;

    prayer_times::table
        .filter(prayer_times::zone_code.eq(zone_code))
        .filter(prayer_times::date.ge(from))
        .filter(prayer_times::date.le(to))
        .select(SelectPrayerTime::as_select())
        .order(prayer_times::date.asc())
        .load(conn)
}
