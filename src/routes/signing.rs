use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::post,
    Json, Router,
};
use serde::Deserialize;

use crate::error::ServiceError;
use crate::models::*;
use crate::repo::{EnvelopeRepo, SignerRepo};
use crate::routes::documents::user_id;
use crate::state::AppState;

#[derive(Deserialize)]
struct SendReq {
    #[allow(dead_code)]
    message: Option<String>,
}

#[derive(Deserialize)]
struct SignReq {
    signer_id: String,
    signature_data: String,
}

/// POST /api/envelopes/:id/send — transition Draft → Pending.
async fn send_envelope(
    State(st): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(_body): Json<SendReq>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let owner = user_id(&headers)?;
    let env = EnvelopeRepo::find_by_id(&st.db, &id).await?;
    if env.owner_id != owner {
        return Err(ServiceError::Forbidden("not your envelope".into()));
    }
    if env.status != EnvelopeStatus::Draft {
        return Err(ServiceError::BadRequest("only draft envelopes can be sent".into()));
    }

    let signers = SignerRepo::find_by_envelope(&st.db, &id).await?;
    if signers.is_empty() {
        return Err(ServiceError::BadRequest("add at least one signer first".into()));
    }

    EnvelopeRepo::update_status(&st.db, &id, &EnvelopeStatus::Pending).await?;
    tracing::info!(id = %id, signers = signers.len(), "envelope sent");

    Ok(Json(serde_json::json!({
        "id": id, "status": "pending", "signers": signers.len()
    })))
}

/// POST /api/envelopes/:id/sign — record one signer's signature.
async fn sign_envelope(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<SignReq>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let env = EnvelopeRepo::find_by_id(&st.db, &id).await?;
    if env.status != EnvelopeStatus::Pending {
        return Err(ServiceError::BadRequest("envelope not pending".into()));
    }

    let signer = SignerRepo::find_by_id(&st.db, &body.signer_id).await?;
    if signer.envelope_id != id {
        return Err(ServiceError::BadRequest("signer not in this envelope".into()));
    }
    if signer.status != SignerStatus::Pending {
        return Err(ServiceError::BadRequest("signer already acted".into()));
    }

    let signers = SignerRepo::find_by_envelope(&st.db, &id).await?;
    if env.signing_order == SigningOrder::Sequential {
        check_turn(&signer, &signers)?;
    }

    SignerRepo::update_signed(&st.db, &body.signer_id, &body.signature_data).await?;
    tracing::info!(signer = %body.signer_id, envelope = %id, "signed");

    let all_signed = signers
        .iter()
        .all(|s| s.id == body.signer_id || s.status == SignerStatus::Signed);

    if all_signed {
        EnvelopeRepo::update_status(&st.db, &id, &EnvelopeStatus::Completed).await?;
        tracing::info!(envelope = %id, "all signed — completed");
    }

    Ok(Json(serde_json::json!({
        "signer_id": body.signer_id,
        "status": "signed",
        "envelope_completed": all_signed,
    })))
}

/// For sequential flow, verify all prior signers have signed.
fn check_turn(current: &Signer, all: &[Signer]) -> Result<(), ServiceError> {
    for s in all {
        if s.order < current.order && s.status != SignerStatus::Signed {
            return Err(ServiceError::BadRequest(format!(
                "signer {} (order {}) must sign first",
                s.name, s.order
            )));
        }
    }
    Ok(())
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/envelopes/{id}/send", post(send_envelope))
        .route("/api/envelopes/{id}/sign", post(sign_envelope))
}
