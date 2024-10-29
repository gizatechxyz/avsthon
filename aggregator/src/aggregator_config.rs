use alloy::signers::local::PrivateKeySigner;
use dotenv::dotenv;

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
            "6e7912cf57b1cd9df1b05712e92a082c8c06511f62432abdaad503060822bc72"
                .parse()
                .expect("Failed to parse ECDSA private key");

        Self { ecdsa_signer }
    }
}
