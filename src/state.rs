//! Shared application state.

use std::sync::Arc;

use mongodb::Database;
use crate::config::AppConfig;

/// Passed to all Axum handlers via `State`.
#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub s3: Arc<antarez_s3_storage::S3Client>,
    pub config: AppConfig,
}
