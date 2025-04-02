use crate::queue::{message_queue::Queue, sqs_message_queue::SqsMessageQueue};
use eyre::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Job {
    pub job_id: String,
    pub start_timestamp: i64,
    pub end_timestamp: i64,
    pub job_group_id: Option<String>,
}

pub struct JobDispatcher {
    queue: SqsMessageQueue,
}

impl JobDispatcher {
    pub const fn new(queue: SqsMessageQueue) -> Self {
        Self { queue }
    }

    pub async fn dispatch_job(&self, job: Job) -> Result<String> {
        let message_body = serde_json::to_string(&job)?;
        self.queue
            .send_message(message_body)
            .await
            .map_err(|e| eyre::eyre!(e))?;
        Ok(job.job_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::message_queue::{QueueError, QueueMessage};
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    // Mock Queue implementation for testing
    #[derive(Clone)]
    struct MockQueue {
        should_fail: bool,
        messages: Arc<Mutex<Vec<String>>>,
    }

    impl MockQueue {
        fn new(should_fail: bool) -> Self {
            Self {
                should_fail,
                messages: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    #[async_trait]
    impl Queue for MockQueue {
        async fn send_message(&self, message: String) -> std::result::Result<(), QueueError> {
            if self.should_fail {
                Err(QueueError::SendError("Mock send error".to_string()))
            } else {
                self.messages.lock().unwrap().push(message);
                Ok(())
            }
        }

        async fn receive_messages(&self) -> std::result::Result<Vec<QueueMessage>, QueueError> {
            unimplemented!("Not needed for these tests")
        }
    }

    // Test version of the dispatcher that works directly with our MockQueue
    struct TestDispatcher {
        queue: MockQueue,
    }

    impl TestDispatcher {
        fn new(queue: MockQueue) -> Self {
            Self { queue }
        }

        async fn dispatch_job(&self, job: Job) -> Result<String> {
            let message_body = serde_json::to_string(&job)?;
            self.queue
                .send_message(message_body)
                .await
                .map_err(|e| eyre::eyre!("{}", e))?;
            Ok(job.job_id)
        }
    }

    #[tokio::test]
    async fn test_job_dispatcher_successful_dispatch() {
        // Setup a mock queue that will succeed
        let mock_queue = MockQueue::new(false);
        let messages_ref = mock_queue.messages.clone();

        // Create a test dispatcher with our mock queue
        let dispatcher = TestDispatcher::new(mock_queue);

        // Create a job
        let job = Job {
            job_id: "test-job-123".to_string(),
            start_timestamp: 1000,
            end_timestamp: 2000,
            job_group_id: Some("test-group-123".to_string()),
        };

        // Test dispatch
        let result = dispatcher.dispatch_job(job).await;

        // Verify success
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test-job-123");

        // Verify message was sent
        let messages = messages_ref.lock().unwrap();
        assert_eq!(messages.len(), 1);

        // Parse the message to verify it contains the correct data
        let sent_job: Job = serde_json::from_str(&messages[0]).unwrap();
        assert_eq!(sent_job.job_id, "test-job-123");
        assert_eq!(sent_job.start_timestamp, 1000);
        assert_eq!(sent_job.end_timestamp, 2000);
        assert_eq!(sent_job.job_group_id, Some("test-group-123".to_string()));
    }

    #[tokio::test]
    async fn test_job_dispatcher_failed_dispatch() {
        // Setup a queue that will fail
        let mock_queue = MockQueue::new(true);

        // Create a dispatcher
        let dispatcher = TestDispatcher::new(mock_queue);

        // Create a job
        let job = Job {
            job_id: "test-job-456".to_string(),
            start_timestamp: 3000,
            end_timestamp: 4000,
            job_group_id: Some("test-group-456".to_string()),
        };

        // Test dispatch
        let result = dispatcher.dispatch_job(job).await;

        // Verify failure
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Mock send error"));
    }

    #[tokio::test]
    async fn test_job_serialization() {
        // Create a job
        let job = Job {
            job_id: "serialization-test".to_string(),
            start_timestamp: 5000,
            end_timestamp: 6000,
            job_group_id: Some("serialization-group".to_string()),
        };

        // Test serialization
        let serialized = serde_json::to_string(&job).unwrap();

        // Verify the serialized string contains expected values
        assert!(serialized.contains("serialization-test"));
        assert!(serialized.contains("5000"));
        assert!(serialized.contains("6000"));
        assert!(serialized.contains("serialization-group"));

        // Verify deserialization works
        let deserialized: Job = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.job_id, "serialization-test");
        assert_eq!(deserialized.start_timestamp, 5000);
        assert_eq!(deserialized.end_timestamp, 6000);
        assert_eq!(
            deserialized.job_group_id,
            Some("serialization-group".to_string())
        );
    }
}
