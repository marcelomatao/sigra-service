use axum::{extract::State, routing::{get, post}, Json, Router};
use crate::{error::ServiceError, services::anchoring::anchor_batch, state::AppState};

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "sigra-service",
        "version": crate::VERSION,
    }))
}

async fn trigger_anchor(
    State(st): State<AppState>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let n = anchor_batch(&st.db, &st.config).await?;
    Ok(Json(serde_json::json!({ "anchored": n })))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/admin/anchor", post(trigger_anchor))
}
