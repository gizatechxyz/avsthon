use alloy::signers::local::PrivateKeySigner;
use dirs::home_dir;
use dotenv::dotenv;
use std::env;

/// `OperatorConfig` represents the configuration for the operator service.
///
/// This struct holds the following configuration:
/// - `docker_sock_path`: The path to the Docker socket file (docker.sock).
/// - `ecdsa_signer`: The ECDSA signer for cryptographic operations.
///
/// The configuration is loaded from environment variables, with defaults based
/// on the operating system (macOS/Linux). It optionally loads values from a
/// `.env` file if one exists.
#[derive(Debug)]
pub struct OperatorConfig {
    /// The path to the Docker socket file (docker.sock).
    /// - Defaults to `$HOME/.colima/docker.sock` on macOS.
    /// - Defaults to `/var/run/docker.sock` on Linux.
    /// - Can be overridden by the `DOCKER_SOCK_PATH` environment variable.
    pub docker_sock_path: String,

    /// The URL of the aggregator.
    pub aggregator_url: String,

    /// The ECDSA signer used for cryptographic operations.
    /// - Currently initialized with a hardcoded private key.
    /// - In production, this should be securely loaded from an environment variable
    ///   or a secure key management system.
    pub ecdsa_signer: PrivateKeySigner,
}

impl OperatorConfig {
    /// Constructs a new `OperatorConfig` by loading environment variables.
    ///
    /// This function will:
    /// - Load environment variables from a `.env` file if it exists.
    /// - Set the Docker socket path based on the `DOCKER_SOCK_PATH` environment
    ///   variable or use the platform-specific default path.
    ///
    /// # Example
    ///
    /// ```rust
    /// let config = OperatorConfig::from_env();
    /// println!("Docker socket path: {}", config.docker_sock_path);
    /// ```
    pub(super) fn from_env() -> Self {
        // Load environment variables from .env file if present
        dotenv().ok();

        let docker_sock_path = Self::get_docker_sock_path();

        let ecdsa_signer: PrivateKeySigner =
            "2a7f875389f0ce57b6d3200fb88e9a95e864a2ff589e8b1b11e56faff32a1fc5"
                .parse()
                .expect("Failed to parse ECDSA private key");

        let aggregator_url = "http://0.0.0.0:8080".to_string();

        Self {
            docker_sock_path,
            aggregator_url,
            ecdsa_signer,
        }
    }

    /// Determines the Docker socket path, using the following logic:
    /// - If `DOCKER_SOCK_PATH` is set in the environment, it is used.
    /// - Otherwise, the default path is chosen based on the operating system.
    ///   - macOS: `$HOME/.colima/docker.sock`
    ///   - Linux: `/var/run/docker.sock`
    ///
    /// # Returns
    /// A `String` representing the Docker socket path.
    fn get_docker_sock_path() -> String {
        let default_path = if cfg!(target_os = "macos") {
            let home_dir = Self::get_home_dir();
            String::from(format!("{}/.colima/docker.sock", home_dir))
        } else {
            String::from("/var/run/docker.sock")
        };

        // Override with DOCKER_SOCK_PATH if available
        env::var("DOCKER_SOCK_PATH").unwrap_or(default_path)
    }

    /// Retrieves the user's home directory.
    ///
    /// This function first attempts to get the home directory using the `dirs`
    /// crate's `home_dir()` function. If that fails, it falls back to the `HOME`
    /// environment variable. If both methods fail, it returns `"."` as a fallback.
    ///
    /// # Returns
    /// A `String` representing the home directory or `"."` if it cannot be determined.
    fn get_home_dir() -> String {
        home_dir()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| env::var("HOME").unwrap_or(".".to_string()))
    }
}
