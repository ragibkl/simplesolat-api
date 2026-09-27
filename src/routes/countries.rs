use axum::{Json, extract::State};
use serde::Serialize;

use crate::{
    api::data_repo::Country,
    routes::{AppError, AppState},
};

#[derive(Debug, Serialize)]
pub struct CountriesResponse {
    pub data: Vec<Country>,
}

pub async fn get_countries(
    State(state): State<AppState>,
) -> Result<Json<CountriesResponse>, AppError> {
    tracing::info!("fetching countries");

    let index = state.index().await?;
    Ok(Json(CountriesResponse {
        data: index.countries.clone(),
    }))
}
