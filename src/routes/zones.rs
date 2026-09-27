use axum::{Json, extract::{Query, State}};
use serde::{Deserialize, Serialize};

use crate::{
    api::data_repo,
    routes::{AppError, AppState},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Zone {
    pub zone: String,
    pub country: String,
    pub state: String,
    pub location: String,
    pub timezone: String,
}

impl From<&data_repo::Zone> for Zone {
    fn from(value: &data_repo::Zone) -> Self {
        Self {
            zone: value.code.clone(),
            country: value.country.clone(),
            state: value.state.clone(),
            location: value.location.clone(),
            timezone: value.timezone.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ZonesResponse {
    pub data: Vec<Zone>,
}

#[derive(Debug, Deserialize)]
pub struct ZonesQuery {
    pub country: Option<String>,
}

pub async fn get_zones(
    Query(params): Query<ZonesQuery>,
    State(state): State<AppState>,
) -> Result<Json<ZonesResponse>, AppError> {
    tracing::info!("fetching zones");

    let index = state.index().await?;
    let data = index
        .zones
        .iter()
        .filter(|z| params.country.as_ref().is_none_or(|c| &z.country == c))
        .map(|z| z.into())
        .collect();

    Ok(Json(ZonesResponse { data }))
}
