use bson::doc;
use chrono::Utc;
use futures::TryStreamExt;
use mongodb::Database;

use crate::error::ServiceError;
use crate::models::{Envelope, EnvelopeStatus, Signer};

pub struct EnvelopeRepo;

impl EnvelopeRepo {
    pub async fn insert(db: &Database, env: &Envelope) -> Result<(), ServiceError> {
        db.collection::<Envelope>("envelopes").insert_one(env).await?;
        Ok(())
    }

    pub async fn find_by_id(db: &Database, id: &str) -> Result<Envelope, ServiceError> {
        db.collection::<Envelope>("envelopes")
            .find_one(doc! { "_id": id })
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("envelope {id}")))
    }

    pub async fn update_status(
        db: &Database,
        id: &str,
        status: &EnvelopeStatus,
    ) -> Result<(), ServiceError> {
        let s = serde_json::to_value(status)
            .map_err(|e| ServiceError::Internal(e.to_string()))?;
        db.collection::<Envelope>("envelopes")
            .update_one(
                doc! { "_id": id },
                doc! { "$set": {
                    "status": s.as_str().unwrap_or("draft"),
                    "updated_at": bson::DateTime::from_chrono(Utc::now()),
                }},
            )
            .await?;
        Ok(())
    }

    pub async fn set_anchored(
        db: &Database,
        id: &str,
        uid: &str,
        root: &str,
        proof: &str,
    ) -> Result<(), ServiceError> {
        db.collection::<Envelope>("envelopes")
            .update_one(
                doc! { "_id": id },
                doc! { "$set": {
                    "status": "anchored",
                    "attestation_uid": uid,
                    "merkle_root": root,
                    "merkle_proof": proof,
                    "updated_at": bson::DateTime::from_chrono(Utc::now()),
                }},
            )
            .await?;
        Ok(())
    }

    pub async fn find_completed(db: &Database) -> Result<Vec<Envelope>, ServiceError> {
        let cur = db
            .collection::<Envelope>("envelopes")
            .find(doc! { "status": "completed" })
            .await?;
        Ok(cur.try_collect().await?)
    }
}

pub struct SignerRepo;

impl SignerRepo {
    pub async fn insert(db: &Database, s: &Signer) -> Result<(), ServiceError> {
        db.collection::<Signer>("signers").insert_one(s).await?;
        Ok(())
    }

    pub async fn find_by_envelope(
        db: &Database,
        eid: &str,
    ) -> Result<Vec<Signer>, ServiceError> {
        let cur = db
            .collection::<Signer>("signers")
            .find(doc! { "envelope_id": eid })
            .await?;
        Ok(cur.try_collect().await?)
    }

    pub async fn find_by_id(db: &Database, id: &str) -> Result<Signer, ServiceError> {
        db.collection::<Signer>("signers")
            .find_one(doc! { "_id": id })
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("signer {id}")))
    }

    pub async fn update_signed(
        db: &Database,
        id: &str,
        sig: &str,
    ) -> Result<(), ServiceError> {
        db.collection::<Signer>("signers")
            .update_one(
                doc! { "_id": id },
                doc! { "$set": {
                    "status": "signed",
                    "signed_at": bson::DateTime::from_chrono(Utc::now()),
                    "signature_data": sig,
                }},
            )
            .await?;
        Ok(())
    }
}
