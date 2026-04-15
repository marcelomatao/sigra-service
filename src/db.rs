//! MongoDB connection and index initialization.

use bson::doc;
use mongodb::{Client, Database, IndexModel, options::IndexOptions};

use crate::config::AppConfig;

/// Connect to MongoDB and return a `Database` handle.
pub async fn connect(config: &AppConfig) -> Result<Database, mongodb::error::Error> {
    let client = Client::with_uri_str(&config.mongodb_uri).await?;
    client
        .database("admin")
        .run_command(doc! { "ping": 1 })
        .await?;
    tracing::info!(db = %config.mongodb_database, "connected to MongoDB");
    Ok(client.database(&config.mongodb_database))
}

/// Create required indexes (idempotent — safe on every startup).
pub async fn ensure_indexes(db: &Database) -> Result<(), mongodb::error::Error> {
    // documents: unique hash for dedup + verification.
    db.collection::<bson::Document>("documents")
        .create_index(
            IndexModel::builder()
                .keys(doc! { "hash": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
        )
        .await?;

    // documents: owner lookup.
    db.collection::<bson::Document>("documents")
        .create_index(IndexModel::builder().keys(doc! { "owner_id": 1 }).build())
        .await?;

    // envelopes: owner + status for listing.
    db.collection::<bson::Document>("envelopes")
        .create_index(
            IndexModel::builder()
                .keys(doc! { "owner_id": 1, "status": 1 })
                .build(),
        )
        .await?;

    // envelopes: document_id lookup.
    db.collection::<bson::Document>("envelopes")
        .create_index(IndexModel::builder().keys(doc! { "document_id": 1 }).build())
        .await?;

    // signers: envelope_id for joins.
    db.collection::<bson::Document>("signers")
        .create_index(IndexModel::builder().keys(doc! { "envelope_id": 1 }).build())
        .await?;

    tracing::info!("database indexes ensured");
    Ok(())
}
