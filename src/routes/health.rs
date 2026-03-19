use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use diesel::RunQueryDsl;

use super::AppState;

pub async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let db_ok = state.db_pool.get().ok().and_then(|mut conn| {
        diesel::sql_query("SELECT 1").execute(&mut conn).ok()
    }).is_some();

    let status = if db_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status,
        Json(serde_json::json!({
            "status": if db_ok { "ok" } else { "unavailable" },
            "service": "simplesolat-api",
            "db": if db_ok { "connected" } else { "disconnected" },
        })),
    )
}
