//! Application configuration loaded from environment variables.

use std::env;

/// Application configuration — all values from env vars.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Server listen port.
    pub port: u16,
    /// MongoDB connection URI.
    pub mongodb_uri: String,
    /// MongoDB database name.
    pub mongodb_database: String,
    /// S3 bucket name.
    pub s3_bucket: String,
    /// S3 region (e.g., "us-east-1").
    pub s3_region: String,
    /// S3 endpoint override (for MinIO). None = AWS default.
    pub s3_endpoint: Option<String>,
    /// EAS RPC URL for the target chain.
    pub eas_rpc_url: String,
    /// EAS attester private key (hex, no 0x prefix).
    pub eas_private_key: String,
    /// Target chain ID (default: 8453 = Base).
    pub eas_chain_id: u64,
    /// Presigned URL expiry in seconds.
    pub presigned_url_expiry_secs: u64,
}

impl AppConfig {
    /// Load configuration from environment variables.
    ///
    /// # Panics
    /// Panics if required env vars are missing.
    pub fn from_env() -> Self {
        Self {
            port: env_or("SERVER_PORT", "8080").parse().expect("invalid SERVER_PORT"),
            mongodb_uri: env_required("MONGODB_URI"),
            mongodb_database: env_or("MONGODB_DATABASE", "sigra"),
            s3_bucket: env_required("S3_BUCKET"),
            s3_region: env_or("S3_REGION", "us-east-1"),
            s3_endpoint: env::var("S3_ENDPOINT").ok(),
            eas_rpc_url: env_required("EAS_RPC_URL"),
            eas_private_key: env_required("EAS_PRIVATE_KEY"),
            eas_chain_id: env_or("EAS_CHAIN_ID", "8453").parse().expect("invalid EAS_CHAIN_ID"),
            presigned_url_expiry_secs: env_or("PRESIGNED_URL_EXPIRY_SECS", "3600")
                .parse()
                .expect("invalid PRESIGNED_URL_EXPIRY_SECS"),
        }
    }
}

fn env_required(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("missing required env var: {key}"))
}

fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_or_returns_default() {
        let val = env_or("SIGRA_TEST_NONEXISTENT_VAR", "fallback");
        assert_eq!(val, "fallback");
    }
}
