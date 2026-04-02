use diesel::prelude::*;

#[derive(Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = crate::schema::zones)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UpsertZone {
    pub zone_code: String,
    pub country: String,
    pub state: String,
    pub location: String,
    pub timezone: String,
}

impl UpsertZone {
    /// Returns the IANA timezone for this zone.
    pub fn timezone(&self) -> chrono_tz::Tz {
        self.timezone.parse().unwrap_or(chrono_tz::Asia::Kuala_Lumpur)
    }
}

impl From<&crate::api::data_repo::Zone> for UpsertZone {
    fn from(z: &crate::api::data_repo::Zone) -> Self {
        Self {
            zone_code: z.code.clone(),
            country: z.country.clone(),
            state: z.state.clone(),
            location: z.location.clone(),
            timezone: z.timezone.clone(),
        }
    }
}

pub fn upsert_zone(conn: &mut PgConnection, zone: UpsertZone) -> Result<(), diesel::result::Error> {
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

pub fn select_zone_by_code(conn: &mut PgConnection, zone_code: &str) -> Result<Option<UpsertZone>, diesel::result::Error> {
    use crate::schema::zones;

    zones::table
        .filter(zones::zone_code.eq(zone_code))
        .select(UpsertZone::as_select())
        .first(conn)
        .optional()
}

pub fn select_zones_by_country(conn: &mut PgConnection, country: &str) -> Result<Vec<UpsertZone>, diesel::result::Error> {
    use crate::schema::zones;

    zones::table
        .filter(zones::country.eq(country))
        .select(UpsertZone::as_select())
        .order(zones::zone_code.asc())
        .load(conn)
}
