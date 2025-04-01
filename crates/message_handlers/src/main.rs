use aws_config::{BehaviorVersion, defaults};
use eyre::Result;
use message_handlers::http::create_router;
use message_handlers::queue::sqs_message_queue::SqsMessageQueue;
use message_handlers::services::simple_prove_service::ExampleJobProcessor;
use std::env;
use std::sync::{Arc, atomic::AtomicBool};
use tokio::signal;
use tracing::{Level, debug, info};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing with INFO level default
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!("Starting Fossil Prover Service");

    // Load .env file
    dotenv::dotenv().ok();

    // Get the queue URL from environment variable
    let queue_url = env::var("SQS_QUEUE_URL")
        .unwrap_or_else(|_| "http://localhost:4566/000000000000/fossilQueue".to_string());
    info!("Using SQS Queue URL: {}", queue_url);

    // Load AWS SDK config from environment variables
    // This will respect AWS_ENDPOINT_URL from the .env file
    let config = defaults(BehaviorVersion::latest()).load().await;
    info!("AWS configuration loaded");

    let queue = SqsMessageQueue::new(queue_url, config);

    let terminator = Arc::new(AtomicBool::new(false));
    let terminator_clone = terminator.clone();

    let processor = ExampleJobProcessor::new(queue.clone(), terminator.clone());

    // Start the job processor in a separate task
    let processor_handle = tokio::spawn(async move {
        loop {
            if terminator_clone.load(std::sync::atomic::Ordering::Relaxed) {
                info!("Processor received shutdown signal, stopping...");
                break;
            }
            let result = processor.receive_job().await;
            debug!("Job err?: {:?}", result);
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });

    // Create and start the HTTP server
    let app = create_router(queue.clone()).await;
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("Starting HTTP server on {}", addr);

    let server = axum::serve(tokio::net::TcpListener::bind(addr).await?, app);
    let server_handle = tokio::spawn(async move { server.await });

    // Handle Ctrl+C for graceful shutdown
    info!("Waiting for shutdown signal...");
    signal::ctrl_c().await?;
    info!("Received shutdown signal, initiating graceful shutdown...");

    // Set the terminator flag
    terminator.store(true, std::sync::atomic::Ordering::Relaxed);

    // Wait for the processor to finish
    info!("Waiting for processor to finish...");
    processor_handle.await?;

    // Shutdown the HTTP server
    info!("Shutting down HTTP server...");
    server_handle.abort();

    info!("Shutdown complete");
    Ok(())
}
