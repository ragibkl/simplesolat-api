use diesel::prelude::*;

#[derive(Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = crate::schema::zones)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UpsertZone {
    pub zone_code: String,
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
