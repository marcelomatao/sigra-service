use bson::doc;
use mongodb::Database;

use crate::error::ServiceError;
use crate::models::Document;

pub struct DocumentRepo;

impl DocumentRepo {
    pub async fn insert(db: &Database, doc: &Document) -> Result<(), ServiceError> {
        db.collection::<Document>("documents")
            .insert_one(doc)
            .await
            .map_err(|e| {
                if e.to_string().contains("duplicate key") {
                    ServiceError::Conflict(format!("hash {} already exists", doc.hash))
                } else {
                    ServiceError::Database(e.to_string())
                }
            })?;
        Ok(())
    }

    pub async fn find_by_id(db: &Database, id: &str) -> Result<Document, ServiceError> {
        db.collection::<Document>("documents")
            .find_one(doc! { "_id": id })
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("document {id}")))
    }

    pub async fn find_by_hash(db: &Database, hash: &str) -> Result<Option<Document>, ServiceError> {
        Ok(db
            .collection::<Document>("documents")
            .find_one(doc! { "hash": hash })
            .await?)
    }

    pub async fn list_by_owner(db: &Database, owner: &str) -> Result<Vec<Document>, ServiceError> {
        use futures::TryStreamExt;
        let cur = db
            .collection::<Document>("documents")
            .find(doc! { "owner_id": owner })
            .await?;
        Ok(cur.try_collect().await?)
    }
}
