use dirs::home_dir;
use dotenv::dotenv;
use std::env;

#[derive(Debug)]
pub struct OperatorConfig {
    // DOCKER_SOCK_PATH
    pub docker_sock_path: String,
}

impl OperatorConfig {
    pub(super) fn from_env() -> Self {
        // Load environment variables from .env file if present
        dotenv().ok();

        let docker_sock_path = Self::get_docker_sock_path();
        Self { docker_sock_path }
    }

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

    fn get_home_dir() -> String {
        return home_dir()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| env::var("HOME").unwrap_or(".".to_string()));
    }
}
