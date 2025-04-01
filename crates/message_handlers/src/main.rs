use aws_config::BehaviorVersion;
use aws_config::load_defaults;
use eyre::Result;
use message_handlers::http::create_router;
use message_handlers::queue::sqs_message_queue::SqsMessageQueue;
use message_handlers::services::simple_prove_service::ExampleJobProcessor;
use std::sync::{Arc, atomic::AtomicBool};
use tokio::signal;
use tracing::{debug, info};

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file
    dotenv::dotenv().ok();

    // Initialize AWS SQS client
    let config = load_defaults(BehaviorVersion::latest()).await;

    // Create queue URL - replace with your actual queue URL
    let queue_url = "https://sqs.us-east-1.amazonaws.com/654654236251/fossilQueue".to_string();

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
    println!("Starting HTTP server on {}", addr);

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
