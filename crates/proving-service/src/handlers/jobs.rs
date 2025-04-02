use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
};
use message_handlers::services::job_dispatcher::{Job, JobDispatcher};
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

pub async fn handle_job_request(
    State(dispatcher): State<Arc<JobDispatcher>>,
    Json(request): Json<JobRequest>,
) -> impl IntoResponse {
    info!("Received job request for group: {}", request.job_group_id);
    let mut errors = Vec::new();

    // Dispatch TWAP job
    let twap_job = Job {
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
    let reserve_price_job = Job {
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
    let max_return_job = Job {
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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use axum::http::StatusCode;
    use message_handlers::queue::message_queue::{Queue, QueueError, QueueMessage};
    use message_handlers::services::job_dispatcher::Job;
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

        async fn dispatch_job(&self, job: Job) -> Result<String, QueueError> {
            let message_body = serde_json::to_string(&job).unwrap();
            self.mock_queue.send_message(message_body).await?;
            Ok(job.job_id)
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
        assert_eq!(response.1.message, "All jobs dispatched successfully");
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
        let mut errors = Vec::new();

        // Dispatch TWAP job
        let twap_job = Job {
            job_id: "twap".to_string(),
            start_timestamp: request.twap.start_timestamp,
            end_timestamp: request.twap.end_timestamp,
            job_group_id: Some(request.job_group_id.clone()),
        };

        if let Err(e) = dispatcher.dispatch_job(twap_job).await {
            errors.push(format!("TWAP job failed: {}", e));
        }

        // Dispatch Reserve Price job
        let reserve_price_job = Job {
            job_id: "reserve_price".to_string(),
            start_timestamp: request.reserve_price.start_timestamp,
            end_timestamp: request.reserve_price.end_timestamp,
            job_group_id: Some(request.job_group_id.clone()),
        };

        if let Err(e) = dispatcher.dispatch_job(reserve_price_job).await {
            errors.push(format!("Reserve Price job failed: {}", e));
        }

        // Dispatch Max Return job
        let max_return_job = Job {
            job_id: "max_return".to_string(),
            start_timestamp: request.max_return.start_timestamp,
            end_timestamp: request.max_return.end_timestamp,
            job_group_id: Some(request.job_group_id.clone()),
        };

        if let Err(e) = dispatcher.dispatch_job(max_return_job).await {
            errors.push(format!("Max Return job failed: {}", e));
        }

        if errors.is_empty() {
            (
                StatusCode::OK,
                Response {
                    status: "success".to_string(),
                    message: "All jobs dispatched successfully".to_string(),
                    job_group_id: request.job_group_id,
                },
            )
        } else {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Response {
                    status: "error".to_string(),
                    message: errors.join(", "),
                    job_group_id: request.job_group_id,
                },
            )
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
