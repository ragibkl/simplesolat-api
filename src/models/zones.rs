use diesel::prelude::*;

#[derive(Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = crate::schema::zones)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UpsertZone {
    pub zone_code: String,
    pub country: String,
    pub state: String,
    pub location: String,
}

pub fn upsert_zone(
    conn: &mut PgConnection,
    zone: UpsertZone,
) -> Result<(), diesel::result::Error> {
    use crate::schema::zones;

    diesel::insert_into(zones::table)
        .values(&zone)
        .on_conflict(zones::zone_code)
        .do_update()
        .set(&zone)
        .execute(conn)?;
    Ok(())
}

pub fn select_zones(conn: &mut PgConnection) -> Result<Vec<UpsertZone>, diesel::result::Error> {
    use crate::schema::zones;

    zones::table
        .select(UpsertZone::as_select())
        .order(zones::zone_code.asc())
        .load(conn)
}

impl UpsertZone {
    /// Returns the IANA timezone for this zone based on country and province.
    pub fn timezone(&self) -> chrono_tz::Tz {
        match self.country.as_str() {
            "ID" => match self.state.as_str() {
                // WIB (UTC+7): Sumatera, Jawa, Banten, DKI Jakarta
                "Aceh" | "Bengkulu" | "Jambi" | "Lampung" | "Riau"
                | "Kepulauan Bangka Belitung" | "Kepulauan Riau"
                | "Sumatera Barat" | "Sumatera Selatan" | "Sumatera Utara"
                | "Banten" | "DKI Jakarta"
                | "Jawa Barat" | "Jawa Tengah" | "Jawa Timur"
                | "D.I. Yogyakarta" => chrono_tz::Asia::Jakarta,
                // WIT (UTC+9): Maluku, Papua
                "Maluku" | "Maluku Utara" | "Papua" | "Papua Barat" => chrono_tz::Asia::Jayapura,
                // WITA (UTC+8): Kalimantan, Sulawesi, Bali, Nusa Tenggara, Gorontalo
                _ => chrono_tz::Asia::Makassar,
            },
            _ => chrono_tz::Asia::Kuala_Lumpur, // MY, SG, BN are all UTC+8
        }
    }
}

pub fn select_zone_by_code(
    conn: &mut PgConnection,
    zone_code: &str,
) -> Result<Option<UpsertZone>, diesel::result::Error> {
    use crate::schema::zones;

    zones::table
        .filter(zones::zone_code.eq(zone_code))
        .select(UpsertZone::as_select())
        .first(conn)
        .optional()
}

pub fn select_zones_by_country(
    conn: &mut PgConnection,
    country: &str,
) -> Result<Vec<UpsertZone>, diesel::result::Error> {
    use crate::schema::zones;

    zones::table
        .filter(zones::country.eq(country))
        .select(UpsertZone::as_select())
        .order(zones::zone_code.asc())
        .load(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(zone("ID", "Kalimantan Barat").timezone(), chrono_tz::Asia::Makassar);
        assert_eq!(zone("ID", "Kalimantan Tengah").timezone(), chrono_tz::Asia::Makassar);
        assert_eq!(zone("ID", "Kalimantan Timur").timezone(), chrono_tz::Asia::Makassar);
    }

    #[test]
    fn test_indonesia_wit() {
        assert_eq!(zone("ID", "Papua").timezone(), chrono_tz::Asia::Jayapura);
        assert_eq!(zone("ID", "Maluku").timezone(), chrono_tz::Asia::Jayapura);
    }
}
