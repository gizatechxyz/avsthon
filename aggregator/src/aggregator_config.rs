use alloy::signers::local::PrivateKeySigner;
use dotenv::dotenv;
use std::env;

#[derive(Debug)]
pub struct AggregatorConfig {
    /// The ECDSA signer used for cryptographic operations.
    /// - Currently initialized with a hardcoded private key.
    /// - In production, this should be securely loaded from an environment variable
    ///   or a secure key management system.
    pub ecdsa_signer: PrivateKeySigner,
}

impl AggregatorConfig {
    pub(super) fn from_env() -> Self {
        // Load environment variables from .env file if present
        dotenv().ok();

        let ecdsa_signer: PrivateKeySigner =
            "47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a"
                .parse()
                .expect("Failed to parse ECDSA private key");

        Self { ecdsa_signer }
    }
}
