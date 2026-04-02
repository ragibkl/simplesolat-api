pub mod countries;
pub mod health;
pub mod prayer_times;
pub mod zones;

use axum::{Json, Router, http::StatusCode, response::IntoResponse, routing::get};
use tower_http::cors::CorsLayer;

use crate::{
    models::db::{DbPool, connect_db},
    routes::{
        countries::get_countries,
        health::health_check,
        prayer_times::get_prayer_times,
        zones::get_zones,
    },
};

#[derive(Clone)]
pub struct AppState {
    pub db_pool: DbPool,
}

pub async fn create_app_router() -> Router {
    tracing::info!("connecting to database");
    let db_pool = connect_db();

    // Initialize app state
    let state = AppState { db_pool };

    // Build the router
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
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };
        (
            status,
            Json(serde_json::json!({ "error": message })),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: std::error::Error,
{
    fn from(err: E) -> Self {
        AppError::Internal(err.to_string())
    }
}
