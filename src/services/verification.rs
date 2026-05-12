//! Verification service — check document hashes and EAS attestations.

use futures::TryStreamExt;
use mongodb::Database;
use serde::Serialize;

use crate::{
    error::ServiceError,
    models::{Document, Envelope},
};

#[derive(Serialize)]
pub struct HashResult {
    pub hash: String,
    pub found: bool,
    pub document_id: Option<String>,
    pub filename: Option<String>,
    pub anchored: bool,
    pub attestation_uid: Option<String>,
    pub merkle_root: Option<String>,
    pub chain_id: Option<u64>,
}

#[derive(Serialize)]
pub struct AttestResult {
    pub attestation_uid: String,
    pub found: bool,
    pub envelope_count: usize,
    pub merkle_root: Option<String>,
    pub envelopes: Vec<AnchoredEnv>,
    pub chain_id: Option<u64>,
}

#[derive(Serialize)]
pub struct AnchoredEnv {
    pub envelope_id: String,
    pub document_hash: String,
    pub title: String,
    pub proof_valid: bool,
}

pub async fn verify_by_hash(
    db: &Database,
    hash: &str,
    chain_id: u64,
) -> Result<HashResult, ServiceError> {
    let d = db
        .collection::<Document>("documents")
        .find_one(bson::doc! { "hash": hash })
        .await?;

    let Some(d) = d else {
        return Ok(HashResult {
            hash: hash.into(),
            found: false,
            document_id: None,
            filename: None,
            anchored: false,
            attestation_uid: None,
            merkle_root: None,
            chain_id: None,
        });
    };

    let env: Option<Envelope> = db
        .collection::<Envelope>("envelopes")
        .find_one(bson::doc! { "document_id": &d.id, "status": "anchored" })
        .await?;

    let (anchored, uid, root) = match &env {
        Some(e) => {
            let ok = verify_proof(&d.hash, e);
            (ok, e.attestation_uid.clone(), e.merkle_root.clone())
        }
        None => (false, None, None),
    };

    Ok(HashResult {
        hash: hash.into(),
        found: true,
        document_id: Some(d.id),
        filename: Some(d.filename),
        anchored,
        attestation_uid: uid,
        merkle_root: root,
        chain_id: if anchored { Some(chain_id) } else { None },
    })
}

pub async fn verify_by_attestation(
    db: &Database,
    uid: &str,
    chain_id: u64,
) -> Result<AttestResult, ServiceError> {
    let envs: Vec<Envelope> = db
        .collection::<Envelope>("envelopes")
        .find(bson::doc! { "attestation_uid": uid })
        .await?
        .try_collect()
        .await?;

    if envs.is_empty() {
        return Ok(AttestResult {
            attestation_uid: uid.into(),
            found: false,
            envelope_count: 0,
            merkle_root: None,
            envelopes: vec![],
            chain_id: None,
        });
    }

    let root = envs[0].merkle_root.clone();
    let mut out = Vec::with_capacity(envs.len());

    for e in &envs {
        let d = db
            .collection::<Document>("documents")
            .find_one(bson::doc! { "_id": &e.document_id })
            .await?;
        let hash = d.as_ref().map(|x| x.hash.clone()).unwrap_or_default();
        let pv = d.as_ref().map(|x| verify_proof(&x.hash, e)).unwrap_or(false);
        out.push(AnchoredEnv {
            envelope_id: e.id.clone(),
            document_hash: hash,
            title: e.title.clone(),
            proof_valid: pv,
        });
    }

    Ok(AttestResult {
        attestation_uid: uid.into(),
        found: true,
        envelope_count: out.len(),
        merkle_root: root,
        envelopes: out,
        chain_id: Some(chain_id),
    })
}

fn verify_proof(hash: &str, env: &Envelope) -> bool {
    match (&env.merkle_root, &env.merkle_proof) {
        (Some(root), Some(pj)) => {
            match serde_json::from_str::<sigrachain_crypto::MerkleProof>(pj) {
                Ok(proof) => {
                    sigrachain_crypto::verify_merkle_proof(hash, &proof, root).unwrap_or(false)
                }
                Err(_) => false,
            }
        }
        _ => false,
    }
}
