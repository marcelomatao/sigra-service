use axum::{routing::get, Json, Router};
use crate::state::AppState;

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "sigra-service",
        "version": crate::VERSION,
    }))
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/health", get(health))
}
