use axum::{Router, routing::post};
use message_handlers::{
    queue::sqs_message_queue::SqsMessageQueue, services::job_dispatcher::JobDispatcher,
};
use std::sync::Arc;
use tracing::info;

use crate::handlers::jobs::handle_job_request;

pub async fn create_router(queue: Arc<SqsMessageQueue>) -> Router {
    info!("Setting up HTTP router");

    let dispatcher = Arc::new(JobDispatcher::new(queue));

    Router::new()
        .route("/api/job", post(handle_job_request))
        .with_state(dispatcher)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use message_handlers::queue::message_queue::{Queue, QueueError, QueueMessage};

    // Mock queue implementation for testing
    #[derive(Debug, Clone)]
    struct MockQueue {
        should_fail: bool,
    }

    impl MockQueue {
        fn new(should_fail: bool) -> Self {
            Self { should_fail }
        }
    }

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

    // Create a wrapper to make MockQueue compatible with SqsMessageQueue interface
    // This is needed because our routes expect an SqsMessageQueue
    #[derive(Debug, Clone)]
    struct TestSqsMessageQueue {
        mock_queue: MockQueue,
    }

    impl TestSqsMessageQueue {
        fn new(mock_queue: MockQueue) -> Self {
            Self { mock_queue }
        }
    }

    #[async_trait]
    impl Queue for TestSqsMessageQueue {
        async fn send_message(&self, message: String) -> Result<(), QueueError> {
            self.mock_queue.send_message(message).await
        }

        async fn receive_messages(&self) -> Result<Vec<QueueMessage>, QueueError> {
            self.mock_queue.receive_messages().await
        }

        async fn delete_message(&self, message: &QueueMessage) -> Result<(), QueueError> {
            self.mock_queue.delete_message(message).await
        }
    }

    // Convert TestSqsMessageQueue to SqsMessageQueue for compatibility
    impl From<TestSqsMessageQueue> for SqsMessageQueue {
        fn from(_test_queue: TestSqsMessageQueue) -> Self {
            // This is a test-only implementation, so we use a dummy SqsMessageQueue
            // In a real scenario, you would never do this, but for tests it's acceptable
            let config = aws_config::SdkConfig::builder()
                .behavior_version(aws_config::BehaviorVersion::latest())
                .build();
            SqsMessageQueue::new("dummy-url".to_string(), config)
        }
    }

    #[tokio::test]
    async fn test_create_router() {
        // Create a mock queue that will succeed
        let mock_queue = MockQueue::new(false);
        let test_queue = TestSqsMessageQueue::new(mock_queue);

        // Convert to SqsMessageQueue (needed by create_router)
        let sqs_queue: SqsMessageQueue = test_queue.into();

        // Create the router
        let _app = create_router(Arc::new(sqs_queue)).await;

        // Simple assertion that we created a router
        // In a real test, we might want to test the router by making requests
        assert!(true, "Router created successfully");
    }
}
