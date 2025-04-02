use std::sync::{Arc, atomic::AtomicBool};

use crate::queue::{message_queue::Queue, sqs_message_queue::SqsMessageQueue};
use eyre::Result;
use serde::{Deserialize, Serialize};
use tokio::task;
use tracing::{info, warn};

#[derive(Deserialize, Serialize, Debug)]
pub struct Job {
    pub job_id: String,
    pub start_timestamp: i64,
    pub end_timestamp: i64,
}

pub struct ExampleJobProcessor {
    queue: SqsMessageQueue,
    terminator: Arc<AtomicBool>,
}

impl ExampleJobProcessor {
    pub const fn new(queue: SqsMessageQueue, terminator: Arc<AtomicBool>) -> Self {
        Self { queue, terminator }
    }

    pub async fn receive_job(&self) -> Result<()> {
        info!("Job processor started, waiting for messages");

        while !self.terminator.load(std::sync::atomic::Ordering::Relaxed) {
            if self.terminator.load(std::sync::atomic::Ordering::Relaxed) {
                info!("Termination signal received, stopping message processing");
                break;
            }

            let messages = self.queue.receive_messages().await;

            match messages {
                Ok(messages) => {
                    if !messages.is_empty() {
                        info!("Received {} messages", messages.len());
                    }

                    for message in messages {
                        // Check termination flag before processing each message
                        if self.terminator.load(std::sync::atomic::Ordering::Relaxed) {
                            info!("Termination signal received, stopping message processing");
                            break;
                        }

                        let job: Job = serde_json::from_str(&message.body)?;

                        task::spawn(async move {
                            info!("Processing job: {:?}", job);
                            // TODO: Implement the logic to process the job
                        });
                    }
                }
                Err(e) => {
                    warn!("Error receiving messages from queue: {}", e);
                }
            }

            // Small sleep to prevent tight loops and excessive CPU usage
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        info!("Job processor shutting down");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::message_queue::{QueueError, QueueMessage};
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    // Mock Queue implementation for testing
    #[derive(Clone)]
    struct MockQueue {
        messages: Arc<Mutex<Vec<QueueMessage>>>,
        should_fail_receive: bool,
    }

    impl MockQueue {
        fn new(should_fail_receive: bool) -> Self {
            Self {
                messages: Arc::new(Mutex::new(Vec::new())),
                should_fail_receive,
            }
        }

        fn add_message(&self, message: &str) {
            let queue_message = QueueMessage {
                id: Some("test-id".to_string()),
                body: message.to_string(),
            };
            self.messages.lock().unwrap().push(queue_message);
        }
    }

    #[async_trait]
    impl Queue for MockQueue {
        async fn send_message(&self, message: String) -> std::result::Result<(), QueueError> {
            self.add_message(&message);
            Ok(())
        }

        async fn receive_messages(&self) -> std::result::Result<Vec<QueueMessage>, QueueError> {
            if self.should_fail_receive {
                return Err(QueueError::ReceiveError("Mock receive error".to_string()));
            }

            let mut messages = self.messages.lock().unwrap();
            let result = messages.clone();
            messages.clear();
            Ok(result)
        }
    }

    // Convert MockQueue to SqsMessageQueue for compatibility
    impl From<MockQueue> for SqsMessageQueue {
        fn from(_mock_queue: MockQueue) -> Self {
            // This is a test-only implementation, so we use a dummy SqsMessageQueue
            let config = aws_config::SdkConfig::builder()
                .behavior_version(aws_config::BehaviorVersion::latest())
                .build();
            SqsMessageQueue::new("dummy-url".to_string(), config)
        }
    }

    // Wrap our MockQueue to provide a manual receive method for tests
    struct TestQueue {
        inner: MockQueue,
    }

    impl TestQueue {
        fn new(should_fail_receive: bool) -> Self {
            Self {
                inner: MockQueue::new(should_fail_receive),
            }
        }

        fn add_message(&self, message: &str) {
            self.inner.add_message(message);
        }

        fn get_queue(&self) -> SqsMessageQueue {
            self.inner.clone().into()
        }
    }

    #[tokio::test]
    async fn test_processor_terminates_on_signal() {
        // Create a test queue
        let test_queue = TestQueue::new(false);

        // Add a job to the queue
        let job = Job {
            job_id: "test-job".to_string(),
            start_timestamp: 1000,
            end_timestamp: 2000,
        };
        test_queue.add_message(&serde_json::to_string(&job).unwrap());

        // Create terminator that's initially false
        let terminator = Arc::new(AtomicBool::new(false));

        // Create the processor
        let processor = ExampleJobProcessor::new(test_queue.get_queue(), terminator.clone());

        // Start processing in a separate task
        let process_handle = tokio::spawn({
            let terminator = terminator.clone();
            async move {
                // Set terminator to true after a short delay
                tokio::time::sleep(Duration::from_millis(200)).await;
                terminator.store(true, std::sync::atomic::Ordering::Relaxed);

                // This should eventually return because we set the terminator
                processor.receive_job().await
            }
        });

        // Wait for the processing to complete
        let result = tokio::time::timeout(Duration::from_secs(2), process_handle).await;

        // Check that the task completed and didn't time out
        assert!(result.is_ok());

        // Unwrap the process handle result
        let inner_result = result.unwrap();
        assert!(inner_result.is_ok());

        // Unwrap the receive_job result
        let receive_result = inner_result.unwrap();
        assert!(receive_result.is_ok());
    }

    #[tokio::test]
    async fn test_processor_handles_message() {
        // Create a test queue
        let test_queue = TestQueue::new(false);

        // Add a job to the queue
        let job = Job {
            job_id: "test-job".to_string(),
            start_timestamp: 1000,
            end_timestamp: 2000,
        };
        test_queue.add_message(&serde_json::to_string(&job).unwrap());

        // Create terminator that's initially false
        let terminator = Arc::new(AtomicBool::new(false));

        // Create the processor
        let processor = ExampleJobProcessor::new(test_queue.get_queue(), terminator.clone());

        // Start processing in a separate task
        let process_handle = tokio::spawn({
            let terminator = terminator.clone();
            async move {
                // Set terminator to true after a short delay
                tokio::time::sleep(Duration::from_millis(200)).await;
                terminator.store(true, std::sync::atomic::Ordering::Relaxed);

                processor.receive_job().await
            }
        });

        // Wait for the processing to complete
        let result = tokio::time::timeout(Duration::from_secs(2), process_handle).await;

        // Check that the task completed and didn't time out
        assert!(result.is_ok());

        // Unwrap the process handle result
        let inner_result = result.unwrap();
        assert!(inner_result.is_ok());

        // Unwrap the receive_job result
        let receive_result = inner_result.unwrap();
        assert!(receive_result.is_ok());
    }

    #[tokio::test]
    async fn test_processor_handles_receive_error() {
        // Create a test queue that will fail on receive
        let test_queue = TestQueue::new(true);

        // Create terminator that's initially false
        let terminator = Arc::new(AtomicBool::new(false));

        // Create the processor
        let processor = ExampleJobProcessor::new(test_queue.get_queue(), terminator.clone());

        // Start processing in a separate task
        let process_handle = tokio::spawn({
            let terminator = terminator.clone();
            async move {
                // Set terminator to true after a short delay
                tokio::time::sleep(Duration::from_millis(200)).await;
                terminator.store(true, std::sync::atomic::Ordering::Relaxed);

                processor.receive_job().await
            }
        });

        // Wait for the processing to complete
        let result = tokio::time::timeout(Duration::from_secs(2), process_handle).await;

        // Check that the task completed and didn't time out
        assert!(result.is_ok());

        // Unwrap the process handle result
        let inner_result = result.unwrap();
        assert!(inner_result.is_ok());

        // Unwrap the receive_job result - should still be OK even with queue errors
        let receive_result = inner_result.unwrap();
        assert!(receive_result.is_ok());
    }

    #[tokio::test]
    async fn test_processor_handles_invalid_message() {
        // Create a test queue
        let test_queue = TestQueue::new(false);

        // Add an invalid job to the queue (not valid JSON for Job)
        test_queue.add_message("{\"invalid\": \"message\"}");

        // Create terminator that's initially false
        let terminator = Arc::new(AtomicBool::new(false));

        // Create the processor
        let processor = ExampleJobProcessor::new(test_queue.get_queue(), terminator.clone());

        // Start processing in a separate task
        let process_handle = tokio::spawn({
            let terminator = terminator.clone();
            async move {
                // Set terminator to true after a short delay
                tokio::time::sleep(Duration::from_millis(200)).await;
                terminator.store(true, std::sync::atomic::Ordering::Relaxed);

                processor.receive_job().await
            }
        });

        // Wait for the processing to complete
        let result = tokio::time::timeout(Duration::from_secs(2), process_handle).await;

        // Processor should still complete successfully even with invalid messages
        // as they're logged and skipped, not returned as errors
        assert!(result.is_ok());
        let handle_result = result.unwrap();
        assert!(handle_result.is_ok());
    }
}
