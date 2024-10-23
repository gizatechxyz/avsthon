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
    signers::{local::PrivateKeySigner, Signer},
    transports::http::{Client, Http},
};
use alloy_primitives::{Address, FixedBytes, U256};
use contract_bindings::{
    AVSDirectory::AVSDirectoryInstance,
    GizaAVS::GizaAVSInstance,
    TaskRegistry::{self, TaskRegistryInstance},
    TaskStatus, AVS_DIRECTORY_ADDRESS, GIZA_AVS_ADDRESS, TASK_REGISTRY_ADDRESS,
};
use eyre::{Result, WrapErr};
use futures::StreamExt;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tracing::{error, info};

pub mod aggregator_config;
pub mod server;

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

#[derive(Clone)]
pub struct Aggregator {
    aggregator_address: Address,
    operator_list: Vec<Address>,
    tasks: Arc<RwLock<HashMap<FixedBytes<32>, TaskStatus>>>,
    http_provider: HttpProviderWithSigner,
    pubsub_provider: Arc<RootProvider<PubSubFrontend>>,
    ecdsa_signer: PrivateKeySigner,
}

impl Aggregator {
    pub async fn new() -> Result<Self> {
        let config = AggregatorConfig::from_env();

        let ecdsa_signer = config.ecdsa_signer;
        let aggregator_address = ecdsa_signer.address();
        let wallet = EthereumWallet::from(ecdsa_signer.clone());

        //Create HttpProvider
        let rpc_url = "http://localhost:8545"
            .parse()
            .expect("Failed to parse URL");

        let http_provider = Arc::new(
            ProviderBuilder::new()
                .with_recommended_fillers()
                .wallet(wallet)
                .on_http(rpc_url),
        );

        //Create PubSubProvider
        let ipc_path = "/tmp/anvil.ipc";
        let ipc = IpcConnect::new(ipc_path.to_string());
        let pubsub_provider: Arc<RootProvider<PubSubFrontend>> = Arc::new(
            ProviderBuilder::new()
                .on_ipc(ipc)
                .await
                .expect("Failed to create provider"),
        );

        Ok(Self {
            aggregator_address,
            operator_list: vec![],
            tasks: Arc::new(RwLock::new(HashMap::new())),
            http_provider,
            pubsub_provider,
            ecdsa_signer,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        self.operator_list = self.fetch_operator_list().await?;
        *self.tasks.write().await = self.fetch_task_history().await?;

        // Spawn the task listener
        let tasks = self.tasks.clone();
        let pubsub_provider = self.pubsub_provider.clone();
        tokio::spawn(async move {
            if let Err(e) = Self::listen_for_task(tasks, pubsub_provider).await {
                error!("Task listener error: {:?}", e);
            }
        });

        // Start the server
        info!("Initialization complete. Starting server...");
        let server_handle = tokio::spawn(server::run_server(self.tasks.clone()));

        // Wait for the server to finish (this will run indefinitely)
        match server_handle.await {
            Ok(_) => info!("Server has stopped"),
            Err(e) => tracing::error!("Server error: {}", e),
        }

        Ok(())
    }

    async fn fetch_operator_list(&self) -> Result<Vec<Address>> {
        info!("Fetching operator list");
        let giza_avs = GizaAVSInstance::new(GIZA_AVS_ADDRESS, self.http_provider.clone());
        let avs_directory =
            AVSDirectoryInstance::new(AVS_DIRECTORY_ADDRESS, self.http_provider.clone());

        // We first fetch operators list into GizaAVS
        let mut operator_list = giza_avs
            .OperatorRegistered_filter()
            .from_block(2577255)
            .query()
            .await?
            .into_iter()
            .map(|(operator_address, _)| operator_address.operator)
            .collect::<Vec<_>>();

        // We then filter out the operators that are not registered in AVS Directory (double check to make sure operators is registered in AVS Directory)
        for operator in operator_list.clone() {
            let is_registered = avs_directory
                .avsOperatorStatus(GIZA_AVS_ADDRESS, operator)
                .call()
                .await?;

            if is_registered._0 == U256::ZERO {
                operator_list.retain(|&x| x != operator);
            }
        }

        Ok(operator_list)
    }

    async fn fetch_task_history(&self) -> Result<HashMap<FixedBytes<32>, TaskStatus>> {
        info!("Fetching task history");
        let task_registry =
            TaskRegistryInstance::new(TASK_REGISTRY_ADDRESS, self.http_provider.clone());

        // We get the history of tasks created in TaskRegistry
        let task_list = task_registry
            .TaskRequested_filter()
            .from_block(2577255)
            .query()
            .await?
            .into_iter()
            .map(|(task_id, _)| task_id.taskId)
            .collect::<Vec<_>>();

        // We update the status of each task in the task history
        let mut tasks = HashMap::new();
        for task in task_list {
            let task_status = task_registry.tasks(task).call().await?._0;
            tasks.insert(task, TaskStatus::from(task_status));
        }

        Ok(tasks)
    }

    async fn listen_for_task(
        tasks: Arc<RwLock<HashMap<FixedBytes<32>, TaskStatus>>>,
        pubsub_provider: Arc<RootProvider<PubSubFrontend>>,
    ) -> Result<()> {
        let task_registry = TaskRegistryInstance::new(TASK_REGISTRY_ADDRESS, pubsub_provider);

        let mut stream = task_registry
            .TaskRequested_filter()
            .subscribe()
            .await
            .wrap_err("Failed to subscribe to TaskRegistry events")?
            .into_stream();

        info!("Subscribed to TaskRegistry events. Waiting for events...");

        while let Some(log) = stream.next().await {
            match log {
                Ok(event) => {
                    // Send the task to the processing queue
                    // NOTE: If the channel is full, this will block until there's space.
                    // Consider using `try_send` or implementing a timeout mechanism
                    // to prevent indefinite blocking.
                    tasks
                        .write()
                        .await
                        .insert(event.0.taskId.clone(), TaskStatus::PENDING);
                    info!("New task received: {:?}", event.0.taskId);
                }
                Err(e) => error!("Error receiving event: {:?}", e),
            }
        }

        Ok(())
    }
}
