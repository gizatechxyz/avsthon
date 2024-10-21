use alloy::{
    providers::{IpcConnect, ProviderBuilder, RootProvider},
    pubsub::PubSubFrontend,
};
use contract_bindings::{
    TaskRegistry::{self, TaskRegistryInstance},
    TASK_REGISTRY_ADDRESS,
};
use eyre::{Result, WrapErr};
use futures::StreamExt;
use std::sync::Arc;
use tokio::{
    self,
    sync::mpsc::{self, Receiver, Sender},
    task::JoinHandle,
};
use tracing::{error, info, warn};

// Adjust this based on your expected load and system resources
const QUEUE_CAPACITY: usize = 100;

#[derive(Clone)]
pub struct Operator {
    task_registry: Arc<TaskRegistryInstance<PubSubFrontend, RootProvider<PubSubFrontend>>>,
}

impl Operator {
    pub async fn new() -> Result<Self> {
        // TODO(chalex-eth): Will have to make this WS and RPC Url configurable
        let ipc_path = "/tmp/anvil.ipc";
        let ipc = IpcConnect::new(ipc_path.to_string());
        let provider = ProviderBuilder::new()
            .on_ipc(ipc)
            .await
            .wrap_err("Failed to create provider")?;

        let task_registry = Arc::new(TaskRegistry::new(TASK_REGISTRY_ADDRESS, provider));

        Ok(Self { task_registry })
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting operator...");

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

    async fn listen_for_events(self, tx: Sender<TaskRegistry::TaskRequested>) -> Result<()> {
        // Create a stream of events from the the TaskRequested filter, will block until there is an incoming event
        let mut stream = self
            .task_registry
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
