use alloy::{
    network::{Ethereum, EthereumWallet},
    providers::{
        fillers::{
            BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller,
            WalletFiller,
        },
        Identity, IpcConnect, Provider, ProviderBuilder, RootProvider,
    },
    pubsub::PubSubFrontend,
    signers::{local::PrivateKeySigner, Signer},
    transports::http::{Client, Http},
};
use alloy_primitives::{Address, Bytes, FixedBytes, U256};
use contract_bindings::{
    AVSDirectory::{self, AVSDirectoryInstance},
    ClientAppRegistry::{self, ClientAppMetadata, ClientAppRegistryInstance},
    GizaAVS::{self, GizaAVSInstance},
    ISignatureUtils::SignatureWithSaltAndExpiry,
    TaskRegistry::{self, TaskRegistryInstance},
    AVS_DIRECTORY_ADDRESS, CLIENT_APP_REGISTRY_ADDRESS, GIZA_AVS_ADDRESS, TASK_REGISTRY_ADDRESS,
};
use eyre::{Result, WrapErr};
use futures::StreamExt;
use std::{str::FromStr, sync::Arc};
use tokio::{
    self,
    sync::mpsc::{self, Receiver, Sender},
    task::JoinHandle,
};
use tracing::{error, info, warn};

pub type PubSubProviderWithSigner = Arc<
    FillProvider<
        JoinFill<
            JoinFill<
                Identity,
                JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
            >,
            WalletFiller<EthereumWallet>,
        >,
        RootProvider<PubSubFrontend>,
        PubSubFrontend,
        Ethereum,
    >,
>;

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

// Adjust this based on your expected load and system resources
const QUEUE_CAPACITY: usize = 100;

#[derive(Clone)]
pub struct Operator {
    operator_address: Address,
    pubsub_provider: PubSubProviderWithSigner,
    http_provider: HttpProviderWithSigner,
    signer: PrivateKeySigner,
}

impl Operator {
    pub async fn new() -> Result<Self> {
        // Init wallet, PK is for testing only
        let private_key: PrivateKeySigner =
            "2a7f875389f0ce57b6d3200fb88e9a95e864a2ff589e8b1b11e56faff32a1fc5"
                .parse()
                .unwrap();
        let operator_address = private_key.address();
        let wallet = EthereumWallet::from(private_key.clone());

        //Create PubSubProvider
        let ipc_path = "/tmp/anvil.ipc";
        let ipc = IpcConnect::new(ipc_path.to_string());
        let pubsub_provider = Arc::new(
            ProviderBuilder::new()
                .with_recommended_fillers()
                .wallet(wallet.clone())
                .on_ipc(ipc)
                .await
                .wrap_err("Failed to create provider")?,
        );

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

        Ok(Self {
            operator_address,
            pubsub_provider,
            http_provider,
            signer: private_key,
        })
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting operator...");

        //  self.register_operator().await?;

        self.fetch_client_app().await?;

        // Create a bounded channel for task communication
        // NOTE: Using a bounded channel helps with backpressure, preventing the event listener from overwhelming the task processor. However, if the
        // channel becomes full, it may block the event listener.
        let (tx, rx) = mpsc::channel::<TaskRegistry::TaskRequested>(QUEUE_CAPACITY);

        // Spawn the event listener task
        let event_listener = tokio::spawn(self.clone().listen_for_events(tx));

        // Spawn the task processor
        let task_processor = tokio::spawn(self.clone().process_tasks(rx));

        // Wait for both tasks to complete or handle errors
        self.handle_tasks(event_listener, task_processor).await?;

        Ok(())
    }

    async fn fetch_client_app(&self) -> Result<()> {
        let client_app_registry =
            ClientAppRegistryInstance::new(CLIENT_APP_REGISTRY_ADDRESS, self.http_provider.clone());

        // Fetch all the client apps registered
        let clients_list = client_app_registry
            .ClientAppRegistered_filter()
            .query()
            .await?
            .into_iter()
            .map(|(client_app_id, _)| client_app_id.clientAppId)
            .collect::<Vec<_>>();

        // Fetch the metadata for all the client apps
        let mut client_apps: Vec<ClientAppMetadata> = Vec::new();
        for client_app_id in clients_list {
            let client_app = client_app_registry
                .getClientAppMetadata(client_app_id)
                .call()
                .await?
                ._0;
            client_apps.push(client_app);
        }

        // For PoC let's assume operators opt for all the client apps
        // Check if image is already downloaded, if not download the image
        // Run a container for each client app

        Ok(())
    }

    async fn listen_for_events(self, tx: Sender<TaskRegistry::TaskRequested>) -> Result<()> {
        let task_registry =
            TaskRegistryInstance::new(TASK_REGISTRY_ADDRESS, self.pubsub_provider.clone());

        // Create a stream of events from the the TaskRequested filter, will block until there an incoming event
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
                    if let Err(e) = tx.send(event.0).await {
                        error!("Error sending task to queue: {:?}", e);
                        // TODO(eduponz): Implement retry logic or error handling strategy here
                    }
                }
                Err(e) => error!("Error receiving event: {:?}", e),
            }
        }

        Ok(())
    }

    async fn process_tasks(self, mut rx: Receiver<TaskRegistry::TaskRequested>) {
        while let Some(task) = rx.recv().await {
            info!("Processing task: {:?}", task);
            // Simulate work with a delay
            // In a real scenario, this is where your task processing logic would go
            // Consider implementing proper error handling and retries for task processing
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            info!("Processed task: {:?}", task);
        }
    }

    async fn handle_tasks(
        &self,
        event_listener: JoinHandle<Result<()>>,
        task_processor: JoinHandle<()>,
    ) -> Result<()> {
        tokio::select! {
            event_result = event_listener => {
                match event_result {
                    Ok(result) => result.wrap_err("Event listener task failed"),
                    Err(e) => Err(eyre::eyre!("Event listener task panicked: {:?}", e)),
                }
            }
            _ = task_processor => {
                warn!("Task processor exited unexpectedly");
                Err(eyre::eyre!("Task processor exited unexpectedly"))
            }
        }
    }
}
