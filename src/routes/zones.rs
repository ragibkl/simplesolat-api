use axum::{Json, extract::{Query, State}};
use serde::{Deserialize, Serialize};

use crate::{
    models::zones::{UpsertZone, select_zones, select_zones_by_country},
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

impl From<&UpsertZone> for Zone {
    fn from(value: &UpsertZone) -> Self {
        Self {
            zone: value.zone_code.to_string(),
            country: value.country.to_string(),
            state: value.state.to_string(),
            location: value.location.to_string(),
            timezone: value.timezone.to_string(),
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

    let mut conn = state.db_pool.get()?;

    let zones = match params.country {
        Some(ref country) => select_zones_by_country(&mut conn, country)?,
        None => select_zones(&mut conn)?,
    };

    let response = ZonesResponse {
        data: zones.iter().map(|z| z.into()).collect(),
    };

    Ok(Json(response))
}
