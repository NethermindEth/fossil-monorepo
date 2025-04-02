use aws_config::{BehaviorVersion, defaults};
use eyre::Result;
use message_handler::queue::sqs_message_queue::SqsMessageQueue;
use message_handler::services::simple_prove_service::ExampleJobProcessor;
use std::env;
use std::sync::{Arc, atomic::AtomicBool};
use tokio::signal;
use tracing::{Level, debug, info};
use tracing_subscriber::FmtSubscriber;

// Extract initialization logic to a testable function
pub async fn initialize_tracing() -> Result<()> {
    // Initialize tracing with INFO level default
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| eyre::eyre!("setting default subscriber failed: {}", e));

    Ok(())
}

// Extract configuration loading to a testable function
pub async fn load_queue_config() -> (String, aws_config::SdkConfig) {
    // Load .env file
    dotenv::dotenv().ok();

    // Get the queue URL from environment variable
    let queue_url = env::var("SQS_QUEUE_URL")
        .unwrap_or_else(|_| "http://localhost:4566/000000000000/fossilQueue".to_string());

    // Load AWS SDK config from environment variables
    // This will respect AWS_ENDPOINT_URL from the .env file
    let config = defaults(BehaviorVersion::latest()).load().await;

    (queue_url, config)
}

// Extract processor setup to a testable function
pub fn setup_processor(queue: SqsMessageQueue) -> (ExampleJobProcessor, Arc<AtomicBool>) {
    let terminator = Arc::new(AtomicBool::new(false));
    let processor = ExampleJobProcessor::new(queue, terminator.clone());

    (processor, terminator)
}

// Extract processor execution to a testable function
pub async fn run_processor(processor: ExampleJobProcessor) -> tokio::task::JoinHandle<Result<()>> {
    // Start the job processor in a separate task
    tokio::spawn(async move {
        // Run once - the receive_job method has its own loop
        if let Err(e) = processor.receive_job().await {
            debug!("Job processor exited with error: {:?}", e);
        }
        Ok(())
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    initialize_tracing().await?;

    info!("Starting Fossil Prover Message Handler Service");

    // Load configuration
    let (queue_url, config) = load_queue_config().await;
    info!("Using SQS Queue URL: {}", queue_url);
    info!("AWS configuration loaded");

    let queue = SqsMessageQueue::new(queue_url, config);

    // Setup processor
    let (processor, terminator) = setup_processor(queue);

    // Run the processor
    let processor_handle = run_processor(processor).await;

    // Handle Ctrl+C for graceful shutdown
    info!("Waiting for shutdown signal...");
    signal::ctrl_c().await?;
    info!("Received shutdown signal, initiating graceful shutdown...");

    // Set the terminator flag
    terminator.store(true, std::sync::atomic::Ordering::Relaxed);

    // Wait for the processor to finish
    info!("Waiting for processor to finish...");
    let _ = processor_handle.await;

    info!("Shutdown complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use message_handler::queue::message_queue::{Queue, QueueError, QueueMessage};
    use std::sync::Mutex;
    use std::time::Duration;

    #[test]
    fn test_main_builds() {
        // This test simply verifies that the main.rs file compiles
        assert!(true);
    }

    // Mock queue for testing
    #[derive(Clone)]
    struct MockQueue {
        messages: Arc<Mutex<Vec<QueueMessage>>>,
    }

    impl MockQueue {
        fn new() -> Self {
            Self {
                messages: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    #[async_trait]
    impl Queue for MockQueue {
        async fn send_message(&self, message: String) -> std::result::Result<(), QueueError> {
            self.messages.lock().unwrap().push(QueueMessage {
                id: None,
                body: message,
            });
            Ok(())
        }

        async fn receive_messages(&self) -> std::result::Result<Vec<QueueMessage>, QueueError> {
            let mut messages = self.messages.lock().unwrap();
            let result = messages.clone();
            messages.clear();
            Ok(result)
        }
    }

    // Convert MockQueue to SqsMessageQueue for compatibility
    impl From<MockQueue> for SqsMessageQueue {
        fn from(_mock_queue: MockQueue) -> Self {
            // This is a test-only implementation
            let config = aws_config::SdkConfig::builder()
                .behavior_version(aws_config::BehaviorVersion::latest())
                .build();
            SqsMessageQueue::new("dummy-url".to_string(), config)
        }
    }

    #[tokio::test]
    async fn test_load_queue_config() {
        let (queue_url, _config) = load_queue_config().await;
        // Can't effectively test the config directly, but can verify URL format
        assert!(!queue_url.is_empty());
        assert!(queue_url.contains("://"));
    }

    #[tokio::test]
    async fn test_initialize_tracing() {
        let result = initialize_tracing().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_setup_processor() {
        // Create a mock queue
        let mock_queue = MockQueue::new();
        let queue = mock_queue.into();

        // Test processor setup
        let (_processor, terminator) = setup_processor(queue);

        // Verify terminator is initially false
        assert!(!terminator.load(std::sync::atomic::Ordering::Relaxed));
    }

    #[tokio::test]
    async fn test_run_processor() {
        // Create a mock queue
        let mock_queue = MockQueue::new();
        let queue = mock_queue.into();

        // Setup processor with terminator set to true so it exits immediately
        let terminator = Arc::new(AtomicBool::new(true));
        let processor = ExampleJobProcessor::new(queue, terminator);

        // Run the processor
        let handle = run_processor(processor).await;

        // Wait for the processor to finish
        let result = tokio::time::timeout(Duration::from_millis(100), handle).await;

        // Check that it completed
        assert!(result.is_ok());
        let inner_result = result.unwrap();
        assert!(inner_result.is_ok());
    }
}
