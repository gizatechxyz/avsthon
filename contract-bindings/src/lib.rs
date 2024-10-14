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

use alloy::sol;
use alloy_primitives::{address, Address};

pub const TASK_REGISTRY_ADDRESS: Address = address!("6Da3D07a6BF01F02fB41c02984a49B5d9Aa6ea92");
pub const CLIENT_APP_REGISTRY_ADDRESS: Address =
    address!("a8d297D643a11cE83b432e87eEBce6bee0fd2bAb");
pub const AVS_DIRECTORY_ADDRESS: Address = address!("055733000064333CaDDbC92763c58BF0192fFeBf");
pub const GIZA_AVS_ADDRESS: Address = address!("68d2Ecd85bDEbfFd075Fb6D87fFD829AD025DD5C");
pub const OPERATOR_UJI_ADDRESS: Address = address!("37893031A8484066232AcBE6bFe7E2a7A4411a7d");

sol!(
    #[sol(rpc)]
    TaskRegistry,
    "../contracts/out/TaskRegistry.sol/TaskRegistry.json"
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

sol!(
    #[sol(rpc)]
    ClientAppRegistry,
    "../contracts/out/ClientAppRegistry.sol/ClientAppRegistry.json"
);

sol!(
    #[sol(rpc)]
    GizaAVS,
    "../contracts/out/GizaAVS.sol/GizaAVS.json"
);

sol! {
    #[sol(rpc)]
    interface AVSDirectory {
    function calculateOperatorAVSRegistrationDigestHash(address operator, address avs, bytes32 salt, uint256 expiry) external view returns (bytes32);
}}

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
