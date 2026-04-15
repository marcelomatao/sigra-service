//! Service error types and Axum error response handling.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// Top-level service error.
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error("database error: {0}")]
    Database(String),

    #[error("storage error: {0}")]
    Storage(#[from] antarez_s3_storage::S3Error),

    #[error("crypto error: {0}")]
    Crypto(String),

    #[error("eas error: {0}")]
    Eas(String),

    #[error("internal: {0}")]
    Internal(String),
}

/// JSON error response body.
#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
    message: String,
}

impl IntoResponse for ServiceError {
    fn into_response(self) -> Response {
        let (status, error_type) = match &self {
            ServiceError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
            ServiceError::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            ServiceError::Conflict(_) => (StatusCode::CONFLICT, "conflict"),
            ServiceError::Forbidden(_) => (StatusCode::FORBIDDEN, "forbidden"),
            ServiceError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "database_error"),
            ServiceError::Storage(_) => (StatusCode::INTERNAL_SERVER_ERROR, "storage_error"),
            ServiceError::Crypto(_) => (StatusCode::INTERNAL_SERVER_ERROR, "crypto_error"),
            ServiceError::Eas(_) => (StatusCode::INTERNAL_SERVER_ERROR, "eas_error"),
            ServiceError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        };

        let body = ErrorBody {
            error: error_type.to_string(),
            message: self.to_string(),
        };

        tracing::error!(status = %status, error = %body.message, "request failed");

        (status, axum::Json(body)).into_response()
    }
}

impl From<mongodb::error::Error> for ServiceError {
    fn from(err: mongodb::error::Error) -> Self {
        ServiceError::Database(err.to_string())
    }
}
