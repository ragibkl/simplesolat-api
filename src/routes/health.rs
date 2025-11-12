use axum::{Json, response::IntoResponse};

// Health check endpoint
pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "simplesolat-api"
    }))
}
