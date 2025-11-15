pub mod health;
pub mod prayer_times;
pub mod zones;

use axum::{Json, Router, http::StatusCode, response::IntoResponse, routing::get};
use tower_http::cors::CorsLayer;

use crate::{
    models::db::{DbPool, connect_db},
    routes::{health::health_check, prayer_times::get_prayer_times, zones::get_zones},
};

#[derive(Clone)]
pub struct AppState {
    pub db_pool: DbPool,
}

pub async fn create_app_router() -> Router {
    tracing::info!("Connecting to database...");
    let db_pool = connect_db();

    // Initialize app state
    let state = AppState { db_pool };

    // Build the router
    Router::new()
        .route("/health", get(health_check))
        .route("/prayer-times/by-zone/{zone}", get(get_prayer_times))
        .route("/zones", get(get_zones))
        .layer(CorsLayer::permissive()) // TODO: Configure CORS properly for production
        .with_state(state)
}

// Error handling
#[derive(Debug)]
pub struct AppError {
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": self.message
            })),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: std::error::Error,
{
    fn from(err: E) -> Self {
        AppError {
            message: err.to_string(),
        }
    }
}
