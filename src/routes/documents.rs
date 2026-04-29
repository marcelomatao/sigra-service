use axum::{
    extract::{Multipart, Path, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

use crate::error::ServiceError;
use crate::models::Document;
use crate::repo::DocumentRepo;
use crate::state::AppState;

/// Extract authenticated user UUID from the gateway header.
pub fn user_id(headers: &HeaderMap) -> Result<String, ServiceError> {
    headers
        .get("x-user-uuid")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .ok_or_else(|| ServiceError::Forbidden("missing X-User-UUID header".into()))
}

#[derive(Serialize)]
struct DocumentRes {
    id: String,
    filename: String,
    content_type: String,
    size_bytes: i64,
    hash: String,
    created_at: String,
}

impl From<Document> for DocumentRes {
    fn from(d: Document) -> Self {
        Self {
            id: d.id,
            filename: d.filename,
            content_type: d.content_type,
            size_bytes: d.size_bytes,
            hash: d.hash,
            created_at: d.created_at.to_rfc3339(),
        }
    }
}

#[derive(Serialize)]
struct DownloadRes {
    url: String,
    expires_in_secs: u64,
}

/// POST /api/documents — multipart upload.
async fn upload(
    State(st): State<AppState>,
    headers: HeaderMap,
    mut mp: Multipart,
) -> Result<Json<DocumentRes>, ServiceError> {
    let owner = user_id(&headers)?;
    let field = mp
        .next_field()
        .await
        .map_err(|e| ServiceError::BadRequest(e.to_string()))?
        .ok_or_else(|| ServiceError::BadRequest("no file field".into()))?;

    let filename = field.file_name().unwrap_or("unnamed").to_string();
    let ctype = field.content_type().unwrap_or("application/octet-stream").to_string();
    let bytes = field.bytes().await.map_err(|e| ServiceError::BadRequest(e.to_string()))?;

    let hash = sigrachain_crypto::hash_document(&bytes);
    if DocumentRepo::find_by_hash(&st.db, &hash).await?.is_some() {
        return Err(ServiceError::Conflict(format!("hash {hash} exists")));
    }

    let id = Uuid::new_v4().to_string();
    let s3_key = format!("documents/{id}/{filename}");
    st.s3.put_object(&s3_key, &bytes, Some(&ctype)).await?;

    let doc = Document {
        id,
        owner_id: owner,
        filename,
        content_type: ctype,
        size_bytes: bytes.len() as i64,
        hash,
        s3_key,
        created_at: Utc::now(),
    };
    DocumentRepo::insert(&st.db, &doc).await?;
    tracing::info!(id = %doc.id, hash = %doc.hash, "document uploaded");
    Ok(Json(doc.into()))
}

/// GET /api/documents/:id
async fn get_document(
    State(st): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<DocumentRes>, ServiceError> {
    let owner = user_id(&headers)?;
    let doc = DocumentRepo::find_by_id(&st.db, &id).await?;
    if doc.owner_id != owner {
        return Err(ServiceError::Forbidden("not your document".into()));
    }
    Ok(Json(doc.into()))
}

/// GET /api/documents/:id/download — presigned S3 URL.
async fn download(
    State(st): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<DownloadRes>, ServiceError> {
    let owner = user_id(&headers)?;
    let doc = DocumentRepo::find_by_id(&st.db, &id).await?;
    if doc.owner_id != owner {
        return Err(ServiceError::Forbidden("not your document".into()));
    }
    let exp = std::time::Duration::from_secs(st.config.presigned_url_expiry_secs);
    let pre = st.s3.presigned_download(&doc.s3_key, Some(exp)).await?;
    Ok(Json(DownloadRes {
        url: pre.url,
        expires_in_secs: st.config.presigned_url_expiry_secs,
    }))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/documents", post(upload))
        .route("/api/documents/{id}", get(get_document))
        .route("/api/documents/{id}/download", get(download))
}
