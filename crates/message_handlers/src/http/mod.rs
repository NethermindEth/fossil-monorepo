use crate::queue::sqs_message_queue::SqsMessageQueue;
use crate::services::job_dispatcher::JobDispatcher;
use axum::{
    Router,
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

#[derive(Debug, Deserialize)]
pub struct TimeRange {
    start_timestamp: i64,
    end_timestamp: i64,
}

#[derive(Debug, Deserialize)]
pub struct JobRequest {
    job_group_id: String,
    twap: TimeRange,
    reserve_price: TimeRange,
    max_return: TimeRange,
}

#[derive(Debug, Serialize)]
pub struct Response {
    status: String,
    message: String,
    job_group_id: String,
}

async fn handle_request(
    State(dispatcher): State<Arc<JobDispatcher>>,
    Json(request): Json<JobRequest>,
) -> impl IntoResponse {
    info!("Received job request for group: {}", request.job_group_id);
    let mut errors = Vec::new();

    // Dispatch TWAP job
    let twap_job = crate::services::job_dispatcher::Job {
        job_id: "twap".to_string(),
        start_timestamp: request.twap.start_timestamp,
        end_timestamp: request.twap.end_timestamp,
        job_group_id: Some(request.job_group_id.clone()),
    };
    info!("Dispatching TWAP job for group: {}", request.job_group_id);
    if let Err(e) = dispatcher.dispatch_job(twap_job).await {
        error!("Failed to dispatch TWAP job: {}", e);
        errors.push(format!("TWAP job failed: {}", e));
    }

    // Dispatch Reserve Price job
    let reserve_price_job = crate::services::job_dispatcher::Job {
        job_id: "reserve_price".to_string(),
        start_timestamp: request.reserve_price.start_timestamp,
        end_timestamp: request.reserve_price.end_timestamp,
        job_group_id: Some(request.job_group_id.clone()),
    };
    info!(
        "Dispatching Reserve Price job for group: {}",
        request.job_group_id
    );
    if let Err(e) = dispatcher.dispatch_job(reserve_price_job).await {
        error!("Failed to dispatch Reserve Price job: {}", e);
        errors.push(format!("Reserve Price job failed: {}", e));
    }

    // Dispatch Max Return job
    let max_return_job = crate::services::job_dispatcher::Job {
        job_id: "max_return".to_string(),
        start_timestamp: request.max_return.start_timestamp,
        end_timestamp: request.max_return.end_timestamp,
        job_group_id: Some(request.job_group_id.clone()),
    };
    info!(
        "Dispatching Max Return job for group: {}",
        request.job_group_id
    );
    if let Err(e) = dispatcher.dispatch_job(max_return_job).await {
        error!("Failed to dispatch Max Return job: {}", e);
        errors.push(format!("Max Return job failed: {}", e));
    }

    if errors.is_empty() {
        info!(
            "Successfully dispatched all jobs for group: {}",
            request.job_group_id
        );
        (
            StatusCode::OK,
            Json(Response {
                status: "success".to_string(),
                message: "All jobs dispatched successfully".to_string(),
                job_group_id: request.job_group_id,
            }),
        )
    } else {
        error!(
            "Failed to dispatch some jobs for group: {}. Errors: {}",
            request.job_group_id,
            errors.join(", ")
        );
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Response {
                status: "error".to_string(),
                message: errors.join(", "),
                job_group_id: request.job_group_id,
            }),
        )
    }
}

pub async fn create_router(queue: SqsMessageQueue) -> Router {
    let dispatcher = Arc::new(JobDispatcher::new(queue));

    Router::new()
        .route("/api/job", post(handle_request))
        .with_state(dispatcher)
}
