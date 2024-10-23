use std::{collections::HashMap, sync::Arc};

use alloy_primitives::FixedBytes;
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use contract_bindings::TaskStatus;
use eyre::Result;
use tokio::{net::TcpListener, sync::RwLock};
use tracing::info;

pub async fn run_server(tasks: Arc<RwLock<HashMap<FixedBytes<32>, TaskStatus>>>) -> Result<()> {
    let app = Router::new()
        .route("/task_status/:task_id", get(handle_task_status))
        //.route("/submit_task", post(submit_task))
        .with_state(tasks);

    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    axum::serve(listener, app.into_make_service())
        .await
        .map_err(|_| eyre::eyre!("Failed to start server"))
}

async fn handle_task_status(
    State(tasks): State<Arc<RwLock<HashMap<FixedBytes<32>, TaskStatus>>>>,
    Path(task_id): Path<FixedBytes<32>>,
) -> Json<TaskStatus> {
    let tasks_read = tasks.read().await;
    let task_status = tasks_read
        .get(&task_id)
        .cloned()
        .unwrap_or(TaskStatus::EMPTY);

    info!("Served task status for {:?}", task_id);
    Json(task_status)
}
