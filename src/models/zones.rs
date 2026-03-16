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

pub fn upsert_zone(conn: &mut PgConnection, zone: UpsertZone) {
    use crate::schema::zones;

    diesel::insert_into(zones::table)
        .values(&zone)
        .on_conflict(zones::zone_code)
        .do_update()
        .set(&zone)
        .execute(conn)
        .unwrap();
}

pub fn select_zones(conn: &mut PgConnection) -> Vec<UpsertZone> {
    use crate::schema::zones;

    zones::table
        .select(UpsertZone::as_select())
        .order(zones::zone_code.asc())
        .load(conn)
        .unwrap()
}

impl UpsertZone {
    /// Returns the IANA timezone for this zone based on country and province.
    pub fn timezone(&self) -> chrono_tz::Tz {
        match self.country.as_str() {
            "ID" => match self.state.as_str() {
                // WIB (UTC+7)
                "Aceh" | "Bengkulu" | "Jambi" | "Lampung" | "Riau"
                | "Kepulauan Bangka Belitung" | "Kepulauan Riau"
                | "Sumatera Barat" | "Sumatera Selatan" | "Sumatera Utara"
                | "Banten" | "DKI Jakarta"
                | "Jawa Barat" | "Jawa Tengah" | "Jawa Timur" | "D.I. Yogyakarta"
                | "Kalimantan Barat" | "Kalimantan Tengah" => chrono_tz::Asia::Jakarta,
                // WIT (UTC+9)
                "Maluku" | "Maluku Utara" | "Papua" | "Papua Barat" => chrono_tz::Asia::Jayapura,
                // WITA (UTC+8) — default for remaining provinces
                _ => chrono_tz::Asia::Makassar,
            },
            _ => chrono_tz::Asia::Kuala_Lumpur, // MY, SG, BN are all UTC+8
        }
    }
}

pub fn select_zone_by_code(conn: &mut PgConnection, zone_code: &str) -> Option<UpsertZone> {
    use crate::schema::zones;

    zones::table
        .filter(zones::zone_code.eq(zone_code))
        .select(UpsertZone::as_select())
        .first(conn)
        .optional()
        .unwrap()
}

pub fn select_zones_by_country(conn: &mut PgConnection, country: &str) -> Vec<UpsertZone> {
    use crate::schema::zones;

    zones::table
        .filter(zones::country.eq(country))
        .select(UpsertZone::as_select())
        .order(zones::zone_code.asc())
        .load(conn)
        .unwrap()
}
