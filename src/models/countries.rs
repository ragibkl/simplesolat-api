use diesel::prelude::*;

#[derive(Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = crate::schema::countries)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UpsertCountry {
    pub code: String,
    pub name: String,
    pub source: String,
    pub geojson: String,
    pub mapping: String,
    pub shape_property: String,
}

impl From<&crate::api::data_repo::Country> for UpsertCountry {
    fn from(c: &crate::api::data_repo::Country) -> Self {
        Self {
            code: c.code.clone(),
            name: c.name.clone(),
            source: c.source.clone(),
            geojson: c.geojson.clone(),
            mapping: c.mapping.clone(),
            shape_property: c.shape_property.clone(),
        }
    }
}

pub fn upsert_country(conn: &mut PgConnection, country: UpsertCountry) -> Result<(), diesel::result::Error> {
    use crate::schema::countries;

    diesel::insert_into(countries::table)
        .values(&country)
        .on_conflict(countries::code)
        .do_update()
        .set(&country)
        .execute(conn)?;
    Ok(())
}

pub fn select_countries(conn: &mut PgConnection) -> Result<Vec<UpsertCountry>, diesel::result::Error> {
    use crate::schema::countries;

    countries::table
        .select(UpsertCountry::as_select())
        .order(countries::code.asc())
        .load(conn)
}
