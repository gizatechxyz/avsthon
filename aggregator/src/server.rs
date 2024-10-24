use std::{collections::HashMap, sync::Arc};

use alloy_primitives::{Address, FixedBytes, Signature};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use contract_bindings::TaskStatus;
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
use tokio::{net::TcpListener, sync::RwLock};
use tracing::{error, info};

// Custom error type for server-related errors
#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Invalid operator")]
    InvalidOperator,
    #[error("Task does not exist")]
    TaskDoesNotExist,
    #[error("Task already completed")]
    TaskAlreadyCompleted,
    #[error("Internal server error: {0}")]
    InternalError(String),
}

// Implement IntoResponse for ServerError to handle error responses
impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ServerError::InvalidSignature => {
                (StatusCode::BAD_REQUEST, "Invalid signature".to_string())
            }
            ServerError::InvalidOperator => (StatusCode::FORBIDDEN, "Invalid operator".to_string()),
            ServerError::TaskDoesNotExist => {
                (StatusCode::NOT_FOUND, "Task does not exist".to_string())
            }
            ServerError::TaskAlreadyCompleted => {
                (StatusCode::CONFLICT, "Task already completed".to_string())
            }
            ServerError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

// Struct to represent an operator's response to a task
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct OperatorResponse {
    task_id: FixedBytes<32>,
    result: String,
    signature: Signature,
}

// Application state shared across request handlers
#[derive(Clone)]
pub struct AppState {
    pub operator_list: Arc<RwLock<Vec<Address>>>,
    pub tasks: Arc<RwLock<HashMap<FixedBytes<32>, TaskStatus>>>,
    pub sender: tokio::sync::mpsc::Sender<OperatorResponse>,
}

// Main function to run the server
pub async fn run_server(app_state: AppState) -> Result<(), ServerError> {
    let app = Router::new()
        .route("/task_status/:task_id", get(handle_task_status))
        .route("/submit_task", post(handle_submit_task))
        .with_state(Arc::new(app_state));

    let listener = TcpListener::bind("0.0.0.0:8080")
        .await
        .map_err(|e| ServerError::InternalError(format!("Failed to bind to address: {}", e)))?;

    info!("Server listening on 0.0.0.0:8080");
    axum::serve(listener, app.into_make_service())
        .await
        .map_err(|e| ServerError::InternalError(format!("Server error: {}", e)))
}

// Handler for GET /task_status/:task_id endpoint
async fn handle_task_status(
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<FixedBytes<32>>,
) -> Result<Json<TaskStatus>, ServerError> {
    let task_status = {
        let tasks_read = state.tasks.read().await;
        tasks_read
            .get(&task_id)
            .cloned()
            .unwrap_or(TaskStatus::EMPTY)
    };

    info!("Served task status for {:?}", task_id);
    Ok(Json(task_status))
}

// Handler for POST /submit_task endpoint
async fn handle_submit_task(
    State(state): State<Arc<AppState>>,
    Json(operator_response): Json<OperatorResponse>,
) -> Result<StatusCode, ServerError> {
    // Verify the signature and check if it came from a valid operator
    let recover_address = operator_response
        .signature
        .recover_address_from_msg(&operator_response.result)
        .map_err(|_| ServerError::InvalidSignature)?;

    {
        let operator_list = state.operator_list.read().await;
        if !operator_list.contains(&recover_address) {
            return Err(ServerError::InvalidOperator);
        }
    }

    let task_status = {
        let tasks = state.tasks.read().await;
        tasks.get(&operator_response.task_id).cloned()
    };

    match task_status {
        Some(status) if status == TaskStatus::EMPTY => {
            return Err(ServerError::TaskDoesNotExist);
        }
        Some(status) if status == TaskStatus::COMPLETED || status == TaskStatus::FAILED => {
            return Err(ServerError::TaskAlreadyCompleted);
        }
        None => {
            return Err(ServerError::TaskDoesNotExist);
        }
        _ => {}
    }

    state.sender.send(operator_response).await.map_err(|e| {
        ServerError::InternalError(format!("Failed to send operator response: {}", e))
    })?;

    Ok(StatusCode::OK)
}
