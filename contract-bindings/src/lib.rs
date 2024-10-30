//! This file provides Rust bindings for interacting with smart contracts.
//! Currently, it connects to the contracts through Anvil using an IPC connection.
//!
//! ## Prerequisites
//! - Start an Anvil instance using `anvil --ipc` in one terminal.
//! - Deploy the contract using `make contracts-deploy` in a different terminal.
//!
//! ## TODO(chalex-eth):
//! - Currently, only IPC connections are supported as we are working locally.
//! - Future improvements include adding support for WebSocket (Ws) and HTTP connections
//!   over a generic type of Provider and Transport. This requires further exploration
//!   with the new Alloy crate.

use alloy::{sol, transports::http::reqwest::Url};
use alloy_primitives::{address, Address};
use serde::Serialize;

pub const TASK_REGISTRY_ADDRESS: Address = address!("56421D6AEb393C5361a3f262e5b94626B7E88aD7");
pub const CLIENT_APP_REGISTRY_ADDRESS: Address =
    address!("0D6D127A718A2d1BBFBD809D75048058A2830B8b");
pub const AVS_DIRECTORY_ADDRESS: Address = address!("055733000064333CaDDbC92763c58BF0192fFeBf");
pub const GIZA_AVS_ADDRESS: Address = address!("9f06d855F70a99fcDcA4c9f26A6066499A93923d");
pub const OPERATOR_UJI_ADDRESS: Address = address!("37893031A8484066232AcBE6bFe7E2a7A4411a7d");

sol!(
    #[sol(rpc)]
    TaskRegistry,
    "src/abis/TaskRegistry.json"
);

impl std::fmt::Debug for TaskRegistry::TaskRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "app_id: {:?}", self.appId)
    }
}

impl std::fmt::Debug for TaskRegistry::TaskRequested {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "task_id: {:?}, task_request: {:?}",
            self.taskId, self.taskRequest
        )
    }
}

impl std::fmt::Debug for ClientAppRegistry::ClientAppMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "name: {:?}, description: {:?}, logo_url: {:?}, docker_url: {:?}",
            self.name, self.description, self.logoUrl, self.dockerUrl
        )
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum TaskStatus {
    EMPTY,
    PENDING,
    COMPLETED,
    FAILED,
}

impl From<u8> for TaskStatus {
    fn from(value: u8) -> Self {
        match value {
            0 => TaskStatus::EMPTY,
            1 => TaskStatus::PENDING,
            2 => TaskStatus::COMPLETED,
            3 => TaskStatus::FAILED,
            _ => TaskStatus::EMPTY,
        }
    }
}

impl From<TaskStatus> for u8 {
    fn from(status: TaskStatus) -> Self {
        match status {
            TaskStatus::EMPTY => 0,
            TaskStatus::PENDING => 1,
            TaskStatus::COMPLETED => 2,
            TaskStatus::FAILED => 3,
        }
    }
}

sol!(
    #[sol(rpc)]
    ClientAppRegistry,
    "src/abis/ClientAppRegistry.json"
);

sol!(
    #[sol(rpc)]
    GizaAVS,
    "src/abis/GizaAVS.json"
);

sol! {
    #[sol(rpc)]
    interface AVSDirectory {
    function calculateOperatorAVSRegistrationDigestHash(address operator, address avs, bytes32 salt, uint256 expiry) external view returns (bytes32);
    function avsOperatorStatus(address avs,address operator) external view returns (uint256);
}}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum Chain {
    Anvil,
    Holesky,
}

impl From<String> for Chain {
    fn from(value: String) -> Self {
        match value.as_str() {
            "anvil" => Chain::Anvil,
            "holesky" => Chain::Holesky,
            _ => Chain::Anvil,
        }
    }
}

impl Chain {
    pub fn http_url(&self) -> Url {
        match self {
            Chain::Anvil => Url::parse("http://localhost:8545").unwrap(),
            Chain::Holesky => {
                Url::parse("https://eth-holesky.g.alchemy.com/v2/8lbq3evplhjE7rP48rxeMXcpDNTGz0Hf")
                    .unwrap()
            }
        }
    }

    pub fn ws_url(&self) -> Url {
        match self {
            Chain::Anvil => Url::parse("/tmp/anvil.ipc").unwrap(),
            Chain::Holesky => {
                Url::parse("wss://holesky.infura.io/ws/v3/ee62fcbb87df4cc69d3643770d977603")
                    .unwrap()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::providers::{IpcConnect, ProviderBuilder};
    use eyre::Result;
    use tokio;

    #[tokio::test]
    async fn test_task_registry_interaction() -> Result<()> {
        // Ensure `anvil` is available in $PATH.
        let ipc_path = "/tmp/anvil.ipc";

        // Create the provider.
        let ipc = IpcConnect::new(ipc_path.to_string());
        let provider = ProviderBuilder::new().on_ipc(ipc).await?;

        // Create a contract instance
        let task_registry = TaskRegistry::new(TASK_REGISTRY_ADDRESS, provider.clone());
        let owner = task_registry.owner().call().await?._0;
        assert_eq!(owner, address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266"));

        Ok(())
    }
}
