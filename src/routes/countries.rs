use axum::Json;
use serde::Serialize;

use crate::api::data_repo;

#[derive(Debug, Serialize)]
pub struct Country {
    pub code: String,
    pub name: String,
    pub geojson: String,
    pub mapping: String,
    pub shape_property: String,
}

impl From<&data_repo::Country> for Country {
    fn from(c: &data_repo::Country) -> Self {
        Self {
            code: c.code.clone(),
            name: c.name.clone(),
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

pub async fn get_countries() -> Result<Json<CountriesResponse>, crate::routes::AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| crate::routes::AppError::Internal(e.to_string()))?;

    let countries = data_repo::fetch_countries(&client)
        .await
        .map_err(|e| crate::routes::AppError::Internal(format!("failed to fetch countries: {}", e)))?;

    let response = CountriesResponse {
        data: countries.iter().map(|c| c.into()).collect(),
    };

    Ok(Json(response))
}
