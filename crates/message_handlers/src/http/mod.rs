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

#[derive(Debug, Deserialize)]
pub struct TimeRange {
    start_timestamp: i64,
    end_timestamp: i64,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum RequestType {
    #[serde(rename = "twap")]
    Twap(TimeRange),
    #[serde(rename = "reserve-price")]
    ReservePrice(TimeRange),
    #[serde(rename = "max-return")]
    MaxReturn(TimeRange),
}

#[derive(Debug, Serialize)]
pub struct Response {
    status: String,
    message: String,
    job_id: String,
}

async fn handle_request(
    State(dispatcher): State<Arc<JobDispatcher>>,
    Json(request): Json<RequestType>,
) -> impl IntoResponse {
    let (job_id, time_range) = match request {
        RequestType::Twap(range) => ("twap", range),
        RequestType::ReservePrice(range) => ("reserve_price", range),
        RequestType::MaxReturn(range) => ("max_return", range),
    };

    let job = crate::services::job_dispatcher::Job {
        job_id: job_id.to_string(),
        start_timestamp: time_range.start_timestamp,
        end_timestamp: time_range.end_timestamp,
    };

    match dispatcher.dispatch_job(job).await {
        Ok(_) => (
            StatusCode::OK,
            Json(Response {
                status: "success".to_string(),
                message: "Job dispatched successfully".to_string(),
                job_id: job_id.to_string(),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Response {
                status: "error".to_string(),
                message: e.to_string(),
                job_id: job_id.to_string(),
            }),
        ),
    }
}

pub async fn create_router(queue: SqsMessageQueue) -> Router {
    let dispatcher = Arc::new(JobDispatcher::new(queue));

    Router::new()
        .route("/api/job", post(handle_request))
        .with_state(dispatcher)
}
