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
    transports::http::{Client, Http},
};
use alloy_primitives::{Address, FixedBytes, U256};
use contract_bindings::{
    AVSDirectory::AVSDirectoryInstance, Chain, GizaAVS::GizaAVSInstance,
    TaskRegistry::TaskRegistryInstance, TaskStatus, AVS_DIRECTORY_ADDRESS, GIZA_AVS_ADDRESS,
    TASK_REGISTRY_ADDRESS,
};
use dashmap::DashMap;
use eyre::Result;
use futures::StreamExt;
use rand::Rng;
use server::{AppState, OperatorResponse};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::time::sleep;
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
    #[error("Tx error: {0}")]
    TxError(String),
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

#[derive(Debug, Clone)]
struct AggregatedResponse {
    task_id: FixedBytes<32>,
    responses: DashMap<Address, OperatorResponse>,
}

#[derive(Debug, Clone)]
struct TaskResult {
    task_id: FixedBytes<32>,
    status: TaskStatus,
    result: U256,
}

// Main Aggregator struct representing the core functionality
pub struct Aggregator {
    operator_list: Arc<DashMap<Address, ()>>,
    tasks: Arc<DashMap<FixedBytes<32>, TaskStatus>>,
    operator_responses: Arc<OperatorResponsesByTaskId>,
    http_provider: HttpProviderWithSigner,
    pubsub_provider: Arc<RootProvider<PubSubFrontend>>,
}

impl Aggregator {
    // Initialize a new Aggregator instance
    pub async fn new(chain: Chain) -> Result<Self, AggregatorError> {
        let config = AggregatorConfig::from_env();

        let ecdsa_signer = config.ecdsa_signer;
        let wallet = EthereumWallet::from(ecdsa_signer.clone());

        // Create HttpProvider
        let rpc_url = chain.http_url();

        let http_provider = Arc::new(
            ProviderBuilder::new()
                .with_recommended_fillers()
                .wallet(wallet)
                .on_http(rpc_url),
        );

        let pubsub_provider: Arc<RootProvider<PubSubFrontend>> = match chain {
            Chain::Anvil => {
                let ipc = IpcConnect::new("/tmp/anvil.ipc".to_string());
                ProviderBuilder::new()
                    .on_ipc(ipc)
                    .await
                    .map_err(|e| AggregatorError::ProviderInitError(e.to_string()))?
            }
            Chain::Holesky => ProviderBuilder::new()
                .on_ws(alloy::providers::WsConnect {
                    url: chain.ws_url().to_string(),
                    auth: None,
                })
                .await
                .map_err(|e| AggregatorError::ProviderInitError(e.to_string()))?,
        }
        .into();

        Ok(Self {
            operator_list: Arc::new(DashMap::new()),
            tasks: Arc::new(DashMap::new()),
            operator_responses: Arc::new(DashMap::new()),
            http_provider,
            pubsub_provider,
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

        // Create channels for operator responses and task processing
        let (tx_response, rx_response) = mpsc::channel::<OperatorResponse>(100);
        let (tx_aggregated_response, rx_aggregated_response) =
            mpsc::channel::<AggregatedResponse>(100);
        let (tx_task_process, rx_task_process) = mpsc::channel::<TaskResult>(100);
        let operator_responses = self.operator_responses.clone();
        let operator_list = self.operator_list.clone();
        let tasks = self.tasks.clone();

        // Spawn the operator response queue processor
        tokio::spawn(Self::queue_operator_response(
            rx_response,
            operator_responses.clone(),
            tx_aggregated_response,
            operator_list.clone(),
        ));

        // Spawn the task processor
        tokio::spawn(Self::process_completed_tasks(
            rx_aggregated_response,
            tx_task_process,
            tasks,
        ));

        // Spawn the task result sender
        tokio::spawn(Self::send_task_result(
            rx_task_process,
            self.http_provider.clone(),
        ));

        // Start the server
        info!("Initialization complete. Starting server...");
        let app_state = AppState {
            operator_list: self.operator_list.clone(),
            tasks: self.tasks.clone(),
            sender: tx_response,
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
        tx_aggregated_response: mpsc::Sender<AggregatedResponse>,
        operator_list: Arc<DashMap<Address, ()>>,
    ) -> Result<(), AggregatorError> {
        while let Some(response) = rx.recv().await {
            let operator_address = response
                .signature
                .recover_address_from_msg(response.result.as_bytes())
                .map_err(|e| AggregatorError::SignatureError(e.to_string()))?;

            info!(
                "Aggregating response from operator: {:?} for task: {:?}",
                operator_address, response.task_id
            );

            operator_responses
                .entry(response.clone().task_id)
                .or_insert_with(DashMap::new)
                .insert(operator_address, response.clone());

            // For AVSthon we wait for full operator responses
            // Once hashmap is full we process the task
            let operator_length = operator_list.len();
            if operator_responses.get(&response.task_id).unwrap().len() == operator_length {
                let aggregated_response = AggregatedResponse {
                    task_id: response.task_id,
                    responses: operator_responses.get(&response.task_id).unwrap().clone(),
                };
                match tx_aggregated_response.send(aggregated_response).await {
                    Ok(_) => (),
                    Err(e) => error!("Error sending aggregated response: {:?}", e),
                }
            }
        }

        Ok(())
    }

    async fn process_completed_tasks(
        mut rx: mpsc::Receiver<AggregatedResponse>,
        tx_task_process: mpsc::Sender<TaskResult>,
        tasks: Arc<DashMap<FixedBytes<32>, TaskStatus>>,
    ) {
        while let Some(aggregated_response) = rx.recv().await {
            let task_id = aggregated_response.task_id;
            let extracted_result = aggregated_response
                .responses
                .iter()
                .map(|entry| entry.value().result.trim().to_string())
                .collect::<Vec<String>>()
                .iter()
                .map(|result| result.parse().unwrap())
                .collect::<Vec<U256>>();

            // Check if all values in the array are equal
            let (task_status, consensus_result) =
                if extracted_result.iter().all(|&x| x == extracted_result[0]) {
                    info!("Consensus reached for task: {:?}", task_id);
                    (TaskStatus::COMPLETED, extracted_result[0])
                } else {
                    info!("Consensus not reached for task: {:?}", task_id);
                    (TaskStatus::FAILED, U256::ZERO)
                };

            match tx_task_process
                .send(TaskResult {
                    task_id,
                    status: task_status.clone(),
                    result: consensus_result,
                })
                .await
            {
                Ok(_) => (),
                Err(e) => error!("Failed to send consensus result: {:?}", e),
            }

            tasks.insert(task_id, task_status);
        }
    }

    async fn send_task_result(
        mut rx: mpsc::Receiver<TaskResult>,
        http_provider: HttpProviderWithSigner,
    ) -> Result<(), AggregatorError> {
        let task_registry = TaskRegistryInstance::new(TASK_REGISTRY_ADDRESS, http_provider);

        while let Some(task_result) = rx.recv().await {
            let tx = task_registry
                .respondToTask(
                    task_result.task_id,
                    task_result.status.into(),
                    task_result.result,
                )
                .send()
                .await
                .map_err(|e| AggregatorError::TxError(e.to_string()))?
                .watch()
                .await
                .map_err(|e| AggregatorError::TxError(e.to_string()))?;

            info!("Task result sent tx hash: {:?}", tx);
        }

        Ok(())
    }
}
