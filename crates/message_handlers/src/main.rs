use aws_config::{BehaviorVersion, defaults};
use eyre::Result;
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
    let _ = tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| eyre::eyre!("setting default subscriber failed: {}", e));

    info!("Starting Fossil Prover Message Handler Service");

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
    let processor = ExampleJobProcessor::new(queue.clone(), terminator.clone());

    // Start the job processor in a separate task
    let processor_handle = tokio::spawn(async move {
        // Run once - the receive_job method has its own loop
        if let Err(e) = processor.receive_job().await {
            debug!("Job processor exited with error: {:?}", e);
        }
    });

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
