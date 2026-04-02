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

/// Convert data repo prayer time record to DB upsert format.
/// Zone code is not in the record — must be provided separately.
pub fn to_upsert(zone_code: &str, r: &crate::api::data_repo::PrayerTimeRecord) -> UpsertPrayerTime {
    UpsertPrayerTime {
        zone_code: zone_code.to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_upsert() {
        use crate::api::data_repo::PrayerTimeRecord;
        let record = PrayerTimeRecord {
            date: NaiveDate::from_ymd_opt(2026, 4, 1).unwrap(),
            imsak: NaiveTime::from_hms_opt(5, 55, 0).unwrap(),
            fajr: NaiveTime::from_hms_opt(6, 5, 0).unwrap(),
            syuruk: NaiveTime::from_hms_opt(7, 12, 0).unwrap(),
            dhuhr: NaiveTime::from_hms_opt(13, 20, 0).unwrap(),
            asr: NaiveTime::from_hms_opt(16, 22, 0).unwrap(),
            maghrib: NaiveTime::from_hms_opt(19, 23, 0).unwrap(),
            isha: NaiveTime::from_hms_opt(20, 33, 0).unwrap(),
        };
        let upsert = to_upsert("SGR01", &record);
        assert_eq!(upsert.zone_code, "SGR01");
        assert_eq!(upsert.date, record.date);
        assert_eq!(upsert.fajr, record.fajr);
        assert_eq!(upsert.maghrib, record.maghrib);
    }
}
