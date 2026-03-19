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
