//! sigra-service — Core backend for the Sigra e-signature platform.

pub mod config;
pub mod db;
pub mod error;
pub mod models;
pub mod repo;
pub mod routes;
pub mod services;
pub mod state;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

use std::sync::Arc;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

/// Build the application router (used by `main` and integration tests).
pub async fn app() -> Result<Router, Box<dyn std::error::Error>> {
    let config = config::AppConfig::from_env();
    let db = db::connect(&config).await?;
    db::ensure_indexes(&db).await?;

    let s3_config = if config.s3_endpoint.is_some() {
        antarez_s3_storage::S3Config::minio(
            config.s3_bucket.clone(),
            config.s3_endpoint.clone().unwrap(),
        )
    } else {
        antarez_s3_storage::S3Config::aws(
            config.s3_bucket.clone(),
            config.s3_region.clone(),
        )
    };
    let s3 = Arc::new(antarez_s3_storage::S3Client::new(s3_config).await?);

    let st = state::AppState { db, s3, config };

    services::anchoring::spawn_anchor_loop(
        st.db.clone(),
        Arc::new(st.config.clone()),
        st.config.anchor_interval_secs,
    );

    Ok(routes::router()
        .with_state(st)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any)))
}
