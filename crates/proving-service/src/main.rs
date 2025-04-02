use aws_config::{BehaviorVersion, defaults};
use eyre::Result;
use message_handler::queue::sqs_message_queue::SqsMessageQueue;
use proving_service::create_router;
use std::env;
use tokio::signal;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

// Extract initialization logic to a testable function
pub async fn initialize_tracing() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| eyre::eyre!("setting default subscriber failed: {}", e))?;

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
    let config = defaults(BehaviorVersion::latest()).load().await;

    (queue_url, config)
}

// Extract server setup to a testable function
pub async fn setup_server(
    queue: SqsMessageQueue,
) -> Result<(
    tokio::task::JoinHandle<Result<(), std::io::Error>>,
    std::net::SocketAddr,
)> {
    // Create and start the HTTP server
    let app = create_router(queue).await;
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 3000));

    let server = axum::serve(tokio::net::TcpListener::bind(addr).await?, app);
    let handle = tokio::spawn(async move { server.await });

    Ok((handle, addr))
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    initialize_tracing().await?;

    info!("Starting Fossil Prover HTTP Service");

    // Load configuration
    let (queue_url, config) = load_queue_config().await;
    info!("Using SQS Queue URL: {}", queue_url);
    info!("AWS configuration loaded");

    let queue = SqsMessageQueue::new(queue_url, config);

    // Setup server
    let (handle, addr) = setup_server(queue).await?;
    info!("Starting HTTP server on {}", addr);

    // Handle Ctrl+C for graceful shutdown
    info!("Waiting for shutdown signal...");
    signal::ctrl_c().await?;
    info!("Received shutdown signal, initiating graceful shutdown...");

    // Shutdown the HTTP server
    info!("Shutting down HTTP server...");
    handle.abort();

    info!("Shutdown complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use message_handler::queue::message_queue::{Queue, QueueError, QueueMessage};
    use std::sync::{Arc, Mutex};

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
    async fn test_setup_server() {
        // Create a mock queue
        let mock_queue = MockQueue::new();
        let queue = mock_queue.into();

        // Test server setup
        let result = setup_server(queue).await;
        assert!(result.is_ok());

        // Extract the handle and abort it to clean up
        let (handle, addr) = result.unwrap();
        assert_eq!(addr.port(), 3000);
        handle.abort();
    }

    #[tokio::test]
    async fn test_initialize_tracing() {
        let result = initialize_tracing().await;
        assert!(result.is_ok());
    }
}
