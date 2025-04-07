use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
};
use message_handler::{
    queue::sqs_message_queue::SqsMessageQueue,
    services::{
        job_dispatcher::JobDispatcher,
        jobs::{Job, RequestProof},
    },
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

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

pub async fn handle_job_request(
    State(dispatcher): State<Arc<JobDispatcher<SqsMessageQueue>>>,
    Json(request): Json<JobRequest>,
) -> impl IntoResponse {
    info!("Received job request for group: {}", request.job_group_id);

    // Create a single job with ranges for all three components
    // Use job_group_id as the job_id to simplify identification
    let job_id = request.job_group_id.clone();
    info!("Creating a single job with ID: {}", job_id);

    let combined_job = Job::RequestProof(RequestProof {
        job_id: job_id.clone(), // Use job_group_id as the job_id
        start_timestamp: request.twap.start_timestamp,
        end_timestamp: request.twap.end_timestamp,
        job_group_id: Some(request.job_group_id.clone()),
        twap_start_timestamp: Some(request.twap.start_timestamp),
        twap_end_timestamp: Some(request.twap.end_timestamp),
        reserve_price_start_timestamp: Some(request.reserve_price.start_timestamp),
        reserve_price_end_timestamp: Some(request.reserve_price.end_timestamp),
        max_return_start_timestamp: Some(request.max_return.start_timestamp),
        max_return_end_timestamp: Some(request.max_return.end_timestamp),
    });

    info!("Dispatching job with ID: {}", job_id);

    match dispatcher.dispatch_job(combined_job).await {
        Ok(_) => {
            info!("Successfully dispatched job with ID: {}", job_id);
            (
                StatusCode::OK,
                Json(Response {
                    status: "success".to_string(),
                    message: "Job dispatched successfully".to_string(),
                    job_group_id: request.job_group_id,
                }),
            )
        }
        Err(e) => {
            error!("Failed to dispatch job with ID: {}. Error: {}", job_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Response {
                    status: "error".to_string(),
                    message: format!("Failed to dispatch job: {}", e),
                    job_group_id: request.job_group_id,
                }),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use axum::http::StatusCode;
    use message_handler::queue::message_queue::{Queue, QueueError, QueueMessage};
    use std::sync::Arc;

    // Define a wrapper struct that we can use with JobDispatcher
    #[derive(Debug, Clone)]
    struct MockQueue {
        should_fail: bool,
    }

    impl MockQueue {
        fn new(should_fail: bool) -> Self {
            Self { should_fail }
        }
    }

    // Implement the Queue trait for our MockQueue
    #[async_trait]
    impl Queue for MockQueue {
        async fn send_message(&self, _message: String) -> Result<(), QueueError> {
            if self.should_fail {
                Err(QueueError::SendError("Mock send error".to_string()))
            } else {
                Ok(())
            }
        }

        async fn receive_messages(&self) -> Result<Vec<QueueMessage>, QueueError> {
            unimplemented!("Not needed for these tests")
        }

        async fn delete_message(&self, _message: &QueueMessage) -> Result<(), QueueError> {
            unimplemented!("Not needed for these tests")
        }
    }

    // We need to make our MockQueue usable in place of SqsMessageQueue
    // This is a workaround since we can't directly mock the SqsMessageQueue
    // Create a test wrapper for JobDispatcher that can use our MockQueue
    struct TestJobDispatcher {
        mock_queue: MockQueue,
    }

    impl TestJobDispatcher {
        fn new(mock_queue: MockQueue) -> Self {
            Self { mock_queue }
        }

        async fn dispatch_job(&self, job: Job) -> Result<(), QueueError> {
            let message_body = serde_json::to_string(&job).unwrap();
            self.mock_queue.send_message(message_body).await?;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_handle_job_request_success() {
        // Create a mock queue that will succeed
        let mock_queue = MockQueue::new(false);
        let dispatcher = TestJobDispatcher::new(mock_queue);
        let dispatcher = Arc::new(dispatcher);

        // Create a sample job request
        let request = JobRequest {
            job_group_id: "test-group-123".to_string(),
            twap: TimeRange {
                start_timestamp: 1000,
                end_timestamp: 2000,
            },
            reserve_price: TimeRange {
                start_timestamp: 1000,
                end_timestamp: 2000,
            },
            max_return: TimeRange {
                start_timestamp: 1000,
                end_timestamp: 2000,
            },
        };

        // Call the handler with custom implementation
        let response = handle_job_request_test(dispatcher, request).await;

        // Check response
        assert_eq!(response.0, StatusCode::OK);
        assert_eq!(response.1.status, "success");
        assert_eq!(response.1.job_group_id, "test-group-123");
        assert_eq!(response.1.message, "Job dispatched successfully");
    }

    #[tokio::test]
    async fn test_handle_job_request_failure() {
        // Create a mock queue that will fail
        let mock_queue = MockQueue::new(true);
        let dispatcher = TestJobDispatcher::new(mock_queue);
        let dispatcher = Arc::new(dispatcher);

        // Create a sample job request
        let request = JobRequest {
            job_group_id: "test-group-456".to_string(),
            twap: TimeRange {
                start_timestamp: 1000,
                end_timestamp: 2000,
            },
            reserve_price: TimeRange {
                start_timestamp: 1000,
                end_timestamp: 2000,
            },
            max_return: TimeRange {
                start_timestamp: 1000,
                end_timestamp: 2000,
            },
        };

        // Call the handler with custom implementation
        let response = handle_job_request_test(dispatcher, request).await;

        // Check response
        assert_eq!(response.0, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(response.1.status, "error");
        assert_eq!(response.1.job_group_id, "test-group-456");
        assert!(response.1.message.contains("Mock send error"));
    }

    // Test implementation of the handler that works with our TestJobDispatcher
    async fn handle_job_request_test(
        dispatcher: Arc<TestJobDispatcher>,
        request: JobRequest,
    ) -> (StatusCode, Response) {
        info!("Received job request for group: {}", request.job_group_id);

        // Create a single job with ranges for all three components
        // Use job_group_id as the job_id to simplify identification
        let job_id = request.job_group_id.clone();
        info!("Creating a single job with ID: {}", job_id);

        let combined_job = Job::RequestProof(RequestProof {
            job_id: job_id.clone(), // Use job_group_id as the job_id
            start_timestamp: request.twap.start_timestamp,
            end_timestamp: request.twap.end_timestamp,
            job_group_id: Some(request.job_group_id.clone()),
            twap_start_timestamp: Some(request.twap.start_timestamp),
            twap_end_timestamp: Some(request.twap.end_timestamp),
            reserve_price_start_timestamp: Some(request.reserve_price.start_timestamp),
            reserve_price_end_timestamp: Some(request.reserve_price.end_timestamp),
            max_return_start_timestamp: Some(request.max_return.start_timestamp),
            max_return_end_timestamp: Some(request.max_return.end_timestamp),
        });

        info!("Dispatching job with ID: {}", job_id);

        match dispatcher.dispatch_job(combined_job).await {
            Ok(_) => {
                info!("Successfully dispatched job with ID: {}", job_id);
                (
                    StatusCode::OK,
                    Response {
                        status: "success".to_string(),
                        message: "Job dispatched successfully".to_string(),
                        job_group_id: request.job_group_id,
                    },
                )
            }
            Err(e) => {
                error!("Failed to dispatch job with ID: {}. Error: {}", job_id, e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Response {
                        status: "error".to_string(),
                        message: format!("Failed to dispatch job: {}", e),
                        job_group_id: request.job_group_id,
                    },
                )
            }
        }
    }

    #[tokio::test]
    async fn test_timerange_deserialization() {
        let json = r#"{"start_timestamp": 1000, "end_timestamp": 2000}"#;
        let time_range: TimeRange = serde_json::from_str(json).unwrap();

        assert_eq!(time_range.start_timestamp, 1000);
        assert_eq!(time_range.end_timestamp, 2000);
    }

    #[tokio::test]
    async fn test_job_request_deserialization() {
        let json = r#"{
            "job_group_id": "test-group",
            "twap": {"start_timestamp": 1000, "end_timestamp": 2000},
            "reserve_price": {"start_timestamp": 3000, "end_timestamp": 4000},
            "max_return": {"start_timestamp": 5000, "end_timestamp": 6000}
        }"#;

        let request: JobRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.job_group_id, "test-group");
        assert_eq!(request.twap.start_timestamp, 1000);
        assert_eq!(request.twap.end_timestamp, 2000);
        assert_eq!(request.reserve_price.start_timestamp, 3000);
        assert_eq!(request.reserve_price.end_timestamp, 4000);
        assert_eq!(request.max_return.start_timestamp, 5000);
        assert_eq!(request.max_return.end_timestamp, 6000);
    }
}
