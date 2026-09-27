use axum::{Json, response::IntoResponse};

/// Liveness only. Deliberately doesn't touch the data source, so probes never
/// trigger CDN fetches.
pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "simplesolat-api",
    }))
}
