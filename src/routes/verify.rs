use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};

use crate::{error::ServiceError, services::verification, state::AppState};

async fn verify_hash(
    State(st): State<AppState>,
    Path(hash): Path<String>,
) -> Result<Json<verification::HashResult>, ServiceError> {
    Ok(Json(
        verification::verify_by_hash(&st.db, &hash, st.config.eas_chain_id).await?,
    ))
}

async fn verify_attestation(
    State(st): State<AppState>,
    Path(uid): Path<String>,
) -> Result<Json<verification::AttestResult>, ServiceError> {
    Ok(Json(
        verification::verify_by_attestation(&st.db, &uid, st.config.eas_chain_id).await?,
    ))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/verify/hash/{hash}", get(verify_hash))
        .route("/api/verify/attestation/{uid}", get(verify_attestation))
}
