use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ServiceError;
use crate::models::*;
use crate::repo::{DocumentRepo, EnvelopeRepo, SignerRepo};
use crate::routes::documents::user_id;
use crate::state::AppState;

#[derive(Deserialize)]
struct CreateReq {
    document_id: String,
    title: String,
    signing_order: Option<SigningOrder>,
    deadline: Option<chrono::DateTime<Utc>>,
}

#[derive(Deserialize)]
struct AddSignerReq {
    name: String,
    email: Option<String>,
    wallet_address: Option<String>,
    order: Option<i32>,
}

#[derive(Serialize)]
struct EnvelopeRes {
    id: String,
    document_id: String,
    title: String,
    status: EnvelopeStatus,
    signing_order: SigningOrder,
    deadline: Option<String>,
    attestation_uid: Option<String>,
    signers: Vec<SignerRes>,
    created_at: String,
}

#[derive(Serialize)]
struct SignerRes {
    id: String,
    name: String,
    email: Option<String>,
    wallet_address: Option<String>,
    order: i32,
    status: SignerStatus,
    signed_at: Option<String>,
}

/// POST /api/envelopes
async fn create_envelope(
    State(st): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateReq>,
) -> Result<Json<EnvelopeRes>, ServiceError> {
    let owner = user_id(&headers)?;
    let doc = DocumentRepo::find_by_id(&st.db, &body.document_id).await?;
    if doc.owner_id != owner {
        return Err(ServiceError::Forbidden("not your document".into()));
    }

    let env = Envelope {
        id: Uuid::new_v4().to_string(),
        document_id: body.document_id,
        owner_id: owner,
        title: body.title,
        status: EnvelopeStatus::Draft,
        signing_order: body.signing_order.unwrap_or(SigningOrder::Parallel),
        deadline: body.deadline,
        attestation_uid: None,
        merkle_root: None,
        merkle_proof: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    EnvelopeRepo::insert(&st.db, &env).await?;
    tracing::info!(id = %env.id, "envelope created");
    Ok(Json(to_res(env, vec![])))
}

/// POST /api/envelopes/:id/signers
async fn add_signer(
    State(st): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<AddSignerReq>,
) -> Result<Json<SignerRes>, ServiceError> {
    let owner = user_id(&headers)?;
    let env = EnvelopeRepo::find_by_id(&st.db, &id).await?;
    if env.owner_id != owner {
        return Err(ServiceError::Forbidden("not your envelope".into()));
    }
    if env.status != EnvelopeStatus::Draft {
        return Err(ServiceError::BadRequest("envelope not in draft".into()));
    }
    if body.email.is_none() && body.wallet_address.is_none() {
        return Err(ServiceError::BadRequest("email or wallet_address required".into()));
    }

    let existing = SignerRepo::find_by_envelope(&st.db, &id).await?;
    let order = body.order.unwrap_or(existing.len() as i32 + 1);

    let signer = Signer {
        id: Uuid::new_v4().to_string(),
        envelope_id: id,
        email: body.email,
        wallet_address: body.wallet_address,
        name: body.name,
        order,
        status: SignerStatus::Pending,
        signed_at: None,
        signature_data: None,
    };
    SignerRepo::insert(&st.db, &signer).await?;
    Ok(Json(signer_res(&signer)))
}

/// GET /api/envelopes/:id
async fn get_envelope(
    State(st): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<EnvelopeRes>, ServiceError> {
    let owner = user_id(&headers)?;
    let env = EnvelopeRepo::find_by_id(&st.db, &id).await?;
    if env.owner_id != owner {
        return Err(ServiceError::Forbidden("not your envelope".into()));
    }
    let signers = SignerRepo::find_by_envelope(&st.db, &id).await?;
    Ok(Json(to_res(env, signers)))
}

fn to_res(e: Envelope, signers: Vec<Signer>) -> EnvelopeRes {
    EnvelopeRes {
        id: e.id,
        document_id: e.document_id,
        title: e.title,
        status: e.status,
        signing_order: e.signing_order,
        deadline: e.deadline.map(|d| d.to_rfc3339()),
        attestation_uid: e.attestation_uid,
        created_at: e.created_at.to_rfc3339(),
        signers: signers.iter().map(signer_res).collect(),
    }
}

fn signer_res(s: &Signer) -> SignerRes {
    SignerRes {
        id: s.id.clone(),
        name: s.name.clone(),
        email: s.email.clone(),
        wallet_address: s.wallet_address.clone(),
        order: s.order,
        status: s.status.clone(),
        signed_at: s.signed_at.map(|d| d.to_rfc3339()),
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/envelopes", post(create_envelope))
        .route("/api/envelopes/{id}", get(get_envelope))
        .route("/api/envelopes/{id}/signers", post(add_signer))
}
