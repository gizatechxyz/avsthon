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
use dashmap::DashMap;
use eyre::Result;
use futures::StreamExt;
use server::{AppState, OperatorResponse};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc;
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
    #[error("Signature error: {0}")]
    SignatureError(String),
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

type OperatorResponsesByTaskId = DashMap<FixedBytes<32>, DashMap<Address, OperatorResponse>>;

// Main Aggregator struct representing the core functionality
pub struct Aggregator {
    aggregator_address: Address,
    operator_list: Arc<DashMap<Address, ()>>,
    tasks: Arc<DashMap<FixedBytes<32>, TaskStatus>>,
    operator_responses: Arc<OperatorResponsesByTaskId>,
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
            operator_list: Arc::new(DashMap::new()),
            tasks: Arc::new(DashMap::new()),
            operator_responses: Arc::new(DashMap::new()),
            http_provider,
            pubsub_provider,
            ecdsa_signer,
        })
    }

    // Main run function to start the Aggregator
    pub async fn run(&mut self) -> Result<(), AggregatorError> {
        // Fetch and update operator list
        let fetched_operators = self.fetch_operator_list().await?;
        self.operator_list.clear();
        for operator in fetched_operators {
            self.operator_list.insert(operator, ());
        }

        // Fetch and update task history
        self.tasks = Arc::new(self.fetch_task_history().await?);

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
        let operator_responses = self.operator_responses.clone();
        tokio::spawn(Self::queue_operator_response(rx, operator_responses));

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
    ) -> Result<DashMap<FixedBytes<32>, TaskStatus>, AggregatorError> {
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

        let tasks = DashMap::new();
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
        tasks: Arc<DashMap<FixedBytes<32>, TaskStatus>>,
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
                    tasks.insert(event.0.taskId, TaskStatus::PENDING);
                    info!("New task detected: {:?}", event.0.taskId);
                }
                Err(e) => error!("Error receiving event: {:?}", e),
            }
        }

        Ok(())
    }

    // Process operator responses
    async fn queue_operator_response(
        mut rx: mpsc::Receiver<OperatorResponse>,
        operator_responses: Arc<OperatorResponsesByTaskId>,
    ) -> Result<(), AggregatorError> {
        while let Some(response) = rx.recv().await {
            let operator_address = response
                .signature
                .recover_address_from_msg(response.result.as_bytes())
                .map_err(|e| AggregatorError::SignatureError(e.to_string()))?;

            info!(
                "Received response from operator: {:?} for task: {:?}",
                operator_address, response.task_id
            );
            operator_responses
                .entry(response.task_id)
                .or_insert_with(DashMap::new)
                .insert(operator_address, response);
        }

        Ok(())
    }
}

// We want 100% consensus, so we need to wait for all the operators response
// Once tasks are coming we need to store them in a queue (maybe hashmap) and wait for all the operators to respond
// Once we get all the responses we can process the task and update the task status if there is consensus we update to completed if not we set to failed
// Figure out how to handle the data type and structure of the task queue and how to process them while waiting for getting all the responses
