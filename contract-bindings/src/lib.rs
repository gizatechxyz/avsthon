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

pub const TASK_REGISTRY_ADDRESS: Address = address!("e7f1725E7734CE288F8367e1Bb143E90bb3F0512");
pub const CLIENT_APP_REGISTRY_ADDRESS: Address =
    address!("5FbDB2315678afecb367f032d93F642f64180aa3");

// TODO(chalex-eth): For now we provide the path to the compiled contract, but once the contract is
// "frozen" we can provide static ABI

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

sol!(
    #[sol(rpc)]
    ClientAppRegistryContract,
    "../contracts/out/ClientAppRegistry.sol/ClientAppRegistry.json"
);

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
