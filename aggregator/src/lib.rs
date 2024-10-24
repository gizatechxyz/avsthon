use aggregator_config::AggregatorConfig;
use alloy::{
    network::{Ethereum, EthereumWallet},
    providers::{
        fillers::{
            BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller,
            WalletFiller,
        },
        Identity, IpcConnect, ProviderBuilder, RootProvider,
    },
    pubsub::PubSubFrontend,
    signers::local::PrivateKeySigner,
    transports::http::{Client, Http},
};
use alloy_primitives::{Address, FixedBytes, U256};
use contract_bindings::{
    AVSDirectory::AVSDirectoryInstance, GizaAVS::GizaAVSInstance,
    TaskRegistry::TaskRegistryInstance, TaskStatus, AVS_DIRECTORY_ADDRESS, GIZA_AVS_ADDRESS,
    TASK_REGISTRY_ADDRESS,
};
use eyre::Result;
use futures::StreamExt;
use server::{AppState, OperatorResponse};
use std::{collections::HashMap, sync::Arc};
use thiserror::Error;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info};

pub mod aggregator_config;
pub mod server;

// Define custom error types for better error handling and reporting
#[derive(Error, Debug)]
pub enum AggregatorError {
    #[error("Failed to initialize provider: {0}")]
    ProviderInitError(String),
    #[error("Failed to fetch operator list: {0}")]
    OperatorListFetchError(String),
    #[error("Failed to fetch task history: {0}")]
    TaskHistoryFetchError(String),
    #[error("Task listener error: {0}")]
    TaskListenerError(String),
    #[error("Server error: {0}")]
    ServerError(String),
}

// Type alias for the complex provider type to improve readability
pub type HttpProviderWithSigner = Arc<
    FillProvider<
        JoinFill<
            JoinFill<
                Identity,
                JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
            >,
            WalletFiller<EthereumWallet>,
        >,
        RootProvider<Http<Client>>,
        Http<Client>,
        Ethereum,
    >,
>;

// Main Aggregator struct representing the core functionality
pub struct Aggregator {
    aggregator_address: Address,
    operator_list: Arc<RwLock<Vec<Address>>>,
    tasks: Arc<RwLock<HashMap<FixedBytes<32>, TaskStatus>>>,
    http_provider: HttpProviderWithSigner,
    pubsub_provider: Arc<RootProvider<PubSubFrontend>>,
    ecdsa_signer: PrivateKeySigner,
}

impl Aggregator {
    // Initialize a new Aggregator instance
    pub async fn new() -> Result<Self, AggregatorError> {
        let config = AggregatorConfig::from_env();

        let ecdsa_signer = config.ecdsa_signer;
        let aggregator_address = ecdsa_signer.address();
        let wallet = EthereumWallet::from(ecdsa_signer.clone());

        // Create HttpProvider
        let rpc_url = "http://localhost:8545"
            .parse()
            .expect("Failed to parse RPC URL");

        let http_provider = Arc::new(
            ProviderBuilder::new()
                .with_recommended_fillers()
                .wallet(wallet)
                .on_http(rpc_url),
        );

        // Create PubSubProvider
        let ipc_path = "/tmp/anvil.ipc";
        let ipc = IpcConnect::new(ipc_path.to_string());
        let pubsub_provider: Arc<RootProvider<PubSubFrontend>> = Arc::new(
            ProviderBuilder::new()
                .on_ipc(ipc)
                .await
                .map_err(|e| AggregatorError::ProviderInitError(e.to_string()))?,
        );

        Ok(Self {
            aggregator_address,
            operator_list: Arc::new(RwLock::new(vec![])),
            tasks: Arc::new(RwLock::new(HashMap::new())),
            http_provider,
            pubsub_provider,
            ecdsa_signer,
        })
    }

    // Main run function to start the Aggregator
    pub async fn run(&self) -> Result<(), AggregatorError> {
        // Fetch and update operator list
        {
            let fetched_operators = self.fetch_operator_list().await?;
            let mut operator_list = self.operator_list.write().await;
            *operator_list = fetched_operators;
        } // operator_list write lock is released here

        // Fetch and update task history
        {
            let fetched_tasks = self.fetch_task_history().await?;
            let mut tasks = self.tasks.write().await;
            *tasks = fetched_tasks;
        } // tasks write lock is released here

        // Spawn the task listener
        let tasks = self.tasks.clone();
        let pubsub_provider = self.pubsub_provider.clone();
        tokio::spawn(async move {
            if let Err(e) = Self::listen_for_task(tasks, pubsub_provider).await {
                error!("Task listener error: {:?}", e);
            }
        });

        // Create a channel to process operator responses
        let (tx, rx) = mpsc::channel::<OperatorResponse>(100);
        tokio::spawn(Self::process_operator_response(rx));

        // Start the server
        info!("Initialization complete. Starting server...");
        let app_state = AppState {
            operator_list: self.operator_list.clone(),
            tasks: self.tasks.clone(),
            sender: tx,
        };

        server::run_server(app_state)
            .await
            .map_err(|e| AggregatorError::ServerError(e.to_string()))
    }

    // Fetch the list of registered operators
    async fn fetch_operator_list(&self) -> Result<Vec<Address>, AggregatorError> {
        info!("Fetching operator list");
        let giza_avs = GizaAVSInstance::new(GIZA_AVS_ADDRESS, self.http_provider.clone());
        let avs_directory =
            AVSDirectoryInstance::new(AVS_DIRECTORY_ADDRESS, self.http_provider.clone());

        // Fetch operators list from GizaAVS
        let operator_list = giza_avs
            .OperatorRegistered_filter()
            .from_block(2577255)
            .query()
            .await
            .map_err(|e| AggregatorError::OperatorListFetchError(e.to_string()))?
            .into_iter()
            .map(|(operator_address, _)| operator_address.operator)
            .collect::<Vec<_>>();

        // Filter out operators not registered in AVS Directory
        let mut registered_operators = Vec::new();
        for &operator in &operator_list {
            let is_registered = avs_directory
                .avsOperatorStatus(GIZA_AVS_ADDRESS, operator)
                .call()
                .await
                .map(|status| status._0 != U256::ZERO)
                .unwrap_or(false);
            if is_registered {
                registered_operators.push(operator);
            }
        }

        Ok(registered_operators)
    }

    // Fetch the history of tasks
    async fn fetch_task_history(
        &self,
    ) -> Result<HashMap<FixedBytes<32>, TaskStatus>, AggregatorError> {
        info!("Fetching task history");
        let task_registry =
            TaskRegistryInstance::new(TASK_REGISTRY_ADDRESS, self.http_provider.clone());

        let task_list = task_registry
            .TaskRequested_filter()
            .from_block(2577255)
            .query()
            .await
            .map_err(|e| AggregatorError::TaskHistoryFetchError(e.to_string()))?
            .into_iter()
            .map(|(task_id, _)| task_id.taskId)
            .collect::<Vec<_>>();

        let mut tasks = HashMap::new();
        for task in task_list {
            let task_status = task_registry
                .tasks(task)
                .call()
                .await
                .map_err(|e| AggregatorError::TaskHistoryFetchError(e.to_string()))?
                ._0;
            tasks.insert(task, TaskStatus::from(task_status));
        }

        Ok(tasks)
    }

    // Listen for new tasks and update the task list
    async fn listen_for_task(
        tasks: Arc<RwLock<HashMap<FixedBytes<32>, TaskStatus>>>,
        pubsub_provider: Arc<RootProvider<PubSubFrontend>>,
    ) -> Result<(), AggregatorError> {
        let task_registry = TaskRegistryInstance::new(TASK_REGISTRY_ADDRESS, pubsub_provider);

        let mut stream = task_registry
            .TaskRequested_filter()
            .subscribe()
            .await
            .map_err(|e| AggregatorError::TaskListenerError(e.to_string()))?
            .into_stream();

        info!("Subscribed to TaskRegistry events. Waiting for events...");

        while let Some(log) = stream.next().await {
            match log {
                Ok(event) => {
                    let mut tasks = tasks.write().await;
                    tasks.insert(event.0.taskId.clone(), TaskStatus::PENDING);
                    info!("New task received: {:?}", event.0.taskId);
                }
                Err(e) => error!("Error receiving event: {:?}", e),
            }
        }

        Ok(())
    }

    // Process operator responses
    async fn process_operator_response(mut rx: mpsc::Receiver<OperatorResponse>) {
        while let Some(response) = rx.recv().await {
            info!("Received response: {:?}", response);
            // TODO: Implement response processing logic
        }
    }
}
