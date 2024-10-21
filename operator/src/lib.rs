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
use bollard::{
    container::Config, container::CreateContainerOptions, container::LogsOptions,
    container::StartContainerOptions, image::CreateImageOptions, Docker, API_DEFAULT_VERSION,
};
use contract_bindings::{
    AVSDirectory::AVSDirectoryInstance,
    ClientAppRegistry::{ClientAppMetadata, ClientAppRegistryInstance},
    GizaAVS::GizaAVSInstance,
    ISignatureUtils::SignatureWithSaltAndExpiry,
    TaskRegistry::{self, TaskRegistryInstance},
    AVS_DIRECTORY_ADDRESS, CLIENT_APP_REGISTRY_ADDRESS, GIZA_AVS_ADDRESS, TASK_REGISTRY_ADDRESS,
};
use dirs::home_dir;
use eyre::{Result, WrapErr};
use futures::StreamExt;
use regex::Regex;
use std::{str::FromStr, sync::Arc};
use tokio::{
    self,
    sync::mpsc::{self, Receiver, Sender},
    task::JoinHandle,
};
use tracing::{error, info, warn};

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
    pubsub_provider: Arc<RootProvider<PubSubFrontend>>,
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
        let pubsub_provider: Arc<RootProvider<PubSubFrontend>> = Arc::new(
            ProviderBuilder::new()
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

        self.register_operator_in_avs().await?;

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

    async fn register_operator_in_avs(&self) -> Result<()> {
        let giza_avs = GizaAVSInstance::new(GIZA_AVS_ADDRESS, self.http_provider.clone());
        let avs_directory =
            AVSDirectoryInstance::new(AVS_DIRECTORY_ADDRESS, self.http_provider.clone());

        // Register operator in GizaAVS
        let is_operator_registered = giza_avs
            .isOperatorRegistered(self.operator_address)
            .call()
            .await?
            .isRegistered;

        if is_operator_registered {
            info!("Operator already registered");
            return Ok(());
        }

        let salt = FixedBytes::<32>::from_str(
            "0x2ef06b8bbad022ca2dd29795902ceb588d06d1cfd10cb6e687db0dbb837865e9",
        )
        .unwrap();
        let expiry = U256::from(1779248899);

        // Eigenlayer provide a view function to calculate the digest hash that needs to be signed
        let digest_hash = avs_directory
            .calculateOperatorAVSRegistrationDigestHash(
                self.operator_address,
                GIZA_AVS_ADDRESS,
                salt,
                expiry,
            )
            .call()
            .await?
            ._0;

        // We signed the hash
        let signed_digest = self.signer.sign_hash(&digest_hash).await?;

        // Broadcast tx to register in EL contracts and GizaAVS contracts
        let tx = giza_avs
            .registerOperatorToAVS(
                self.operator_address,
                SignatureWithSaltAndExpiry {
                    signature: signed_digest.as_bytes().into(),
                    salt,
                    expiry,
                },
            )
            .send()
            .await?
            .watch()
            .await?;
        info!("GizaAVS registration submitted {:?}", tx);

        // Check if the operator is registered
        let is_operator_registered = giza_avs
            .isOperatorRegistered(self.operator_address)
            .call()
            .await?
            .isRegistered;

        match is_operator_registered {
            true => info!("Successfully registered operator in GizaAVS"),
            false => {
                return Err(eyre::eyre!("Operator registration failed"));
            }
        }

        // Register client app in GizaAVS
        let client_app_id: FixedBytes<32> = FixedBytes::<32>::from_str(
            "0xc86aab04e8ef18a63006f43fa41a2a0150bae3dbe276d581fa8b5cde0ccbc966",
        )
        .unwrap();

        let is_client_app_registered = giza_avs
            .operatorClientAppIdRegistrationStatus(self.operator_address, client_app_id.clone())
            .call()
            .await?
            .isRegistered;

        if is_client_app_registered {
            info!("Client app already registered");
            return Ok(());
        }

        let tx = giza_avs
            .optInClientAppId(client_app_id)
            .send()
            .await?
            .watch()
            .await?;
        info!("Operator successfully opted-in for Client app {:?}", tx);

        let is_client_app_registered = giza_avs
            .operatorClientAppIdRegistrationStatus(self.operator_address, client_app_id.clone())
            .call()
            .await?
            .isRegistered;

        match is_client_app_registered {
            true => info!("Successfully registered client app in GizaAVS"),
            false => {
                return Err(eyre::eyre!("Client app registration failed"));
            }
        }

        Ok(())
    }

    async fn fetch_client_app(&self) -> Result<()> {
        let client_app_registry =
            ClientAppRegistryInstance::new(CLIENT_APP_REGISTRY_ADDRESS, self.http_provider.clone());

        // Fetch all the client apps registered
        let clients_list = client_app_registry
            .ClientAppRegistered_filter()
            .from_block(2577255)
            .query()
            .await?
            .into_iter()
            .map(|(client_app_id, _)| client_app_id.clientAppId)
            .collect::<Vec<_>>();

        // TODO(eduponz): Handle panics here.
        // TODO(eduponz): Support other Docker configurations.
        let docker = Docker::connect_with_socket(
            &(Operator::get_home_dir() + "/.colima/docker.sock"),
            120,
            API_DEFAULT_VERSION,
        )
        .unwrap();

        for client_app_id in &clients_list {
            info!("Getting metadata from ClientApp: {:?}", client_app_id);

            let meta_data = client_app_registry
                .getClientAppMetadata(client_app_id.clone())
                .call()
                .await?;

            info!("Getting image from: {:?}", meta_data._0.dockerUrl);

            let re = Regex::new(r"/layers/([^/]+/[^/]+)/([^/]+)/.+/sha256:([a-f0-9]+)").unwrap();

            let repository: &str;
            let tag: &str;

            if let Some(caps) = re.captures(meta_data._0.dockerUrl.as_str()) {
                repository = &caps[1];
                tag = &caps[2];
                // TODO(eduponz): This is the manifest digest, can be used to check if the image we have corresponds
                //                to the one we want to download
                _ = &caps[3];

                info!("Downloading Docker image: {:?}:{:?}", repository, tag);

                // Download the image if we don't have it
                let options = CreateImageOptions {
                    from_image: repository.to_string(),
                    tag: tag.to_string(),
                    ..Default::default()
                };

                // Request the image
                let mut stream = docker.create_image(Some(options), None, None);

                // Process the stream
                while let Some(result) = stream.next().await {
                    match result {
                        Ok(build_info) => {
                            if let Some(error) = build_info.error {
                                error!("Error pulling image: {:?}", error);
                                continue;
                            }
                        }
                        Err(e) => {
                            error!("Error pulling image: {:?}", e);
                            continue;
                        }
                    }
                }

                info!("Image pulled successfully");
            } else {
                error!(
                    "No repository, tag, or digest found in URL: {:?}",
                    meta_data._0.dockerUrl
                );
                continue;
            }
        }

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
        let client_app_registry =
            ClientAppRegistryInstance::new(CLIENT_APP_REGISTRY_ADDRESS, self.http_provider.clone());

        while let Some(task) = rx.recv().await {
            info!("Processing task: {:?}", task);

            let client_app_id = task.taskRequest.appId;

            info!("Getting metadata from ClientApp: {:?}", client_app_id);

            let meta_data = client_app_registry
                .getClientAppMetadata(client_app_id)
                .call()
                .await
                .unwrap();

            info!("Getting image from: {:?}", meta_data._0.dockerUrl);

            let re = Regex::new(r"/layers/([^/]+/[^/]+)/([^/]+)/.+/sha256:([a-f0-9]+)").unwrap();

            if let Some(caps) = re.captures(meta_data._0.dockerUrl.as_str()) {
                let repository = &caps[1];

                let docker = Docker::connect_with_socket(
                    &(Operator::get_home_dir() + "/.colima/docker.sock"),
                    120,
                    API_DEFAULT_VERSION,
                )
                .unwrap();

                let container_opts = CreateContainerOptions {
                    name: "test",
                    ..Default::default()
                };

                let container_conf = Config {
                    tty: Some(true),
                    attach_stdin: Some(true),
                    image: Some(repository),
                    ..Default::default()
                };

                let container = docker
                    .create_container(Some(container_opts), container_conf)
                    .await
                    .unwrap();

                docker
                    .start_container(&container.id, None::<StartContainerOptions<String>>)
                    .await
                    .unwrap();

                // TODO(eduponz): Find a better way to check if the container is done
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

                let log_opts = LogsOptions::<String> {
                    stdout: true,
                    ..Default::default()
                };

                let mut logs = docker.logs(&container.id, Some(log_opts));

                while let Some(log) = logs.next().await {
                    info!("Container logs: {:?}", log);
                }

                docker.stop_container(&container.id, None).await.unwrap();

                docker.remove_container(&container.id, None).await.unwrap();
            }

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

    fn get_home_dir() -> String {
        return home_dir()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| ".".to_string());
    }
}
