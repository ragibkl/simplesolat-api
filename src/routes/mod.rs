pub mod countries;
pub mod health;
pub mod prayer_times;
pub mod zones;

use std::sync::Arc;

use axum::{Json, Router, http::StatusCode, response::IntoResponse, routing::get};
use tower_http::cors::CorsLayer;

use crate::{
    routes::{
        countries::get_countries,
        health::health_check,
        prayer_times::get_prayer_times,
        zones::get_zones,
    },
    store::{DataStore, Index},
};

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<DataStore>,
}

impl AppState {
    pub async fn index(&self) -> Result<Arc<Index>, AppError> {
        self.store.index().await.map_err(|e| {
            tracing::error!("failed to load zone index: {}", e);
            AppError::BadGateway("failed to load zones from data source".to_string())
        })
    }
}

pub fn create_app_router(store: Arc<DataStore>) -> Router {
    let state = AppState { store };

    Router::new()
        .route("/health", get(health_check))
        .route("/countries", get(get_countries))
        .route("/prayer-times/by-zone/{zone}", get(get_prayer_times))
        .route("/zones", get(get_zones))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

// Error handling
#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    BadRequest(String),
    /// The data CDN failed or returned something unreadable.
    BadGateway(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::BadGateway(msg) => (StatusCode::BAD_GATEWAY, msg),
        };
        (
            status,
            Json(serde_json::json!({ "error": message })),
        )
            .into_response()
    }
}
