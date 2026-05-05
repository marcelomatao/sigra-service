//! Anchoring service — builds a Merkle tree over completed envelopes and submits an EAS attestation.

use std::sync::Arc;

use antarez_eas_client::{chain, client::EasClient, config::EasConfig, types::AttestationRequest};
use mongodb::Database;
use sigrachain_crypto::{build_merkle_tree, generate_merkle_proof};
use tracing::{error, info};

use crate::{config::AppConfig, error::ServiceError, repo::EnvelopeRepo};

/// Anchor all completed, un-anchored envelopes in a single EAS attestation.
///
/// Each envelope's `document_id` is used as a merkle tree leaf. The resulting
/// root is attested on-chain via EAS, and each envelope is updated with its
/// individual proof and the attestation UID.
///
/// Returns the number of envelopes anchored (0 if none pending).
pub async fn anchor_batch(db: &Database, config: &AppConfig) -> Result<usize, ServiceError> {
    let envelopes = EnvelopeRepo::find_completed(db).await?;
    if envelopes.is_empty() {
        return Ok(0);
    }

    // Build merkle tree from document IDs.
    let leaves: Vec<String> = envelopes.iter().map(|e| e.document_id.clone()).collect();
    let tree = build_merkle_tree(leaves).map_err(|e| ServiceError::Crypto(e.to_string()))?;
    let root = tree.root().to_string();

    // Select the chain config based on the configured chain ID.
    let chain_cfg = match config.eas_chain_id {
        8453 => &chain::BASE,
        1 => &chain::ETHEREUM_MAINNET,
        42161 => &chain::ARBITRUM_ONE,
        10 => &chain::OPTIMISM,
        id => return Err(ServiceError::Eas(format!("unsupported EAS chain ID: {id}"))),
    };

    let eas_config = EasConfig::for_chain(chain_cfg, &config.eas_rpc_url);
    let client = EasClient::new(&eas_config, &config.eas_private_key)
        .await
        .map_err(|e| ServiceError::Eas(e.to_string()))?;

    // Attest the merkle root on-chain.
    let req = AttestationRequest::simple(&config.eas_schema_uid, root.as_bytes().to_vec());
    let attestation = client
        .create_attestation(&req)
        .await
        .map_err(|e| ServiceError::Eas(e.to_string()))?;

    info!(
        uid = %attestation.uid,
        tx = %attestation.transaction_hash,
        envelopes = envelopes.len(),
        "Merkle root anchored on-chain"
    );

    // Persist the attestation UID, merkle root, and per-envelope proof.
    let count = envelopes.len();
    for env in &envelopes {
        let proof = generate_merkle_proof(&env.document_id, &tree)
            .map_err(|e| ServiceError::Crypto(e.to_string()))?;
        let proof_json =
            serde_json::to_string(&proof).map_err(|e| ServiceError::Internal(e.to_string()))?;
        EnvelopeRepo::set_anchored(db, &env.id, &attestation.uid, &root, &proof_json).await?;
    }

    Ok(count)
}

/// Spawn a background task that calls [`anchor_batch`] every `interval_secs` seconds.
pub fn spawn_anchor_loop(db: Database, config: Arc<AppConfig>, interval_secs: u64) {
    tokio::spawn(async move {
        let interval = tokio::time::Duration::from_secs(interval_secs);
        loop {
            tokio::time::sleep(interval).await;
            match anchor_batch(&db, &config).await {
                Ok(0) => {}
                Ok(n) => info!(envelopes = n, "Anchor loop: batch anchored"),
                Err(e) => error!(error = %e, "Anchor loop: batch failed"),
            }
        }
    });
}
