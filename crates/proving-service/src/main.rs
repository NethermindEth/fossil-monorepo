use aws_config::{BehaviorVersion, defaults};
use eyre::Result;
use message_handler::queue::sqs_message_queue::SqsMessageQueue;
use proving_service::create_router;
use std::env;
use tokio::signal;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing with INFO level default
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!("Starting Fossil Prover HTTP Service");

    // Load .env file
    dotenv::dotenv().ok();

    // Get the queue URL from environment variable
    let queue_url = env::var("SQS_QUEUE_URL")
        .unwrap_or_else(|_| "http://localhost:4566/000000000000/fossilQueue".to_string());
    info!("Using SQS Queue URL: {}", queue_url);

    // Load AWS SDK config from environment variables
    let config = defaults(BehaviorVersion::latest()).load().await;
    info!("AWS configuration loaded");

    let queue = SqsMessageQueue::new(queue_url, config);

    // Create and start the HTTP server
    let app = create_router(queue).await;
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("Starting HTTP server on {}", addr);

    let server = axum::serve(tokio::net::TcpListener::bind(addr).await?, app);

    // Handle Ctrl+C for graceful shutdown
    let handle = tokio::spawn(async move { server.await });

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
    #[test]
    fn test_main_builds() {
        // This test simply verifies that the main.rs file compiles
        assert!(true);
    }
}
