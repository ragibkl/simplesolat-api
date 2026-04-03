use axum::{Json, extract::State};
use serde::Serialize;

use crate::{
    models::countries::{UpsertCountry, select_countries},
    routes::{AppError, AppState},
};

#[derive(Debug, Serialize)]
pub struct Country {
    pub code: String,
    pub name: String,
    pub source: String,
    pub geojson: String,
    pub mapping: String,
    pub shape_property: String,
}

impl From<&UpsertCountry> for Country {
    fn from(c: &UpsertCountry) -> Self {
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

#[derive(Debug, Serialize)]
pub struct CountriesResponse {
    pub data: Vec<Country>,
}

pub async fn get_countries(
    State(state): State<AppState>,
) -> Result<Json<CountriesResponse>, AppError> {
    tracing::info!("fetching countries");

    let mut conn = state.db_pool.get()?;

    let countries = select_countries(&mut conn)?;
    let response = CountriesResponse {
        data: countries.iter().map(|c| c.into()).collect(),
    };

    Ok(Json(response))
}
