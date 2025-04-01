use axum::{Router, routing::post};
use message_handler::queue::sqs_message_queue::SqsMessageQueue;
use message_handler::services::job_dispatcher::JobDispatcher;
use std::sync::Arc;
use tracing::info;

use crate::handlers::jobs::handle_job_request;

pub async fn create_router(queue: SqsMessageQueue) -> Router {
    info!("Setting up HTTP router");

    let dispatcher = Arc::new(JobDispatcher::new(queue));

    Router::new()
        .route("/api/job", post(handle_job_request))
        .with_state(dispatcher)
}
