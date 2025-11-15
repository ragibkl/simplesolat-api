use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use crate::{
    models::zones::{UpsertZone, select_zones},
    routes::{AppError, AppState},
};

// Types matching your mobile app's expected format
#[derive(Debug, Serialize, Deserialize)]
pub struct Zone {
    pub zone: String,
    pub state: String,
    pub location: String,
}

impl From<&UpsertZone> for Zone {
    fn from(value: &UpsertZone) -> Self {
        Self {
            zone: value.zone_code.to_string(),
            state: value.state.to_string(),
            location: value.location.to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ZonesResponse {
    pub data: Vec<Zone>,
}

pub async fn get_zones(State(state): State<AppState>) -> Result<Json<ZonesResponse>, AppError> {
    tracing::info!("Fetching zones");

    // Get a connection from the pool
    let mut conn = state.db_pool.get().map_err(|e| AppError {
        message: format!("Failed to get database connection: {}", e),
    })?;

    let zones = select_zones(&mut conn);
    let response = ZonesResponse {
        data: zones.iter().map(|pt| pt.into()).collect(),
    };

    Ok(Json(response))
}
