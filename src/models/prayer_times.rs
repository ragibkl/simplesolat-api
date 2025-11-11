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

pub fn select_last_prayer_time_for_zone(
    conn: &mut PgConnection,
    zone_code: &str,
) -> Option<SelectPrayerTime> {
    use crate::schema::prayer_times;

    prayer_times::table
        .filter(prayer_times::zone_code.eq(zone_code))
        .select(SelectPrayerTime::as_select())
        .order(prayer_times::date.desc())
        .first(conn)
        .optional()
        .unwrap()
}

pub fn upsert_prayer_times(conn: &mut PgConnection, prayer_times: &[UpsertPrayerTime]) {
    use crate::schema::prayer_times;

    diesel::insert_into(prayer_times::table)
        .values(prayer_times)
        .on_conflict((prayer_times::zone_code, prayer_times::date))
        .do_nothing()
        .execute(conn)
        .unwrap();
}
