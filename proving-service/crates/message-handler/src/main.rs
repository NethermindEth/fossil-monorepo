use aws_config::{BehaviorVersion, defaults};
use db::DbConnection;
use eyre::Result;
use message_handler::proof_composition::BonsaiProofProvider;
use message_handler::queue::sqs_message_queue::SqsMessageQueue;
use message_handler::services::proof_job_handler::ProofJobHandler;
use std::sync::{Arc, atomic::AtomicBool};
use tokio::signal;
use tokio::time::{Duration, sleep};
use tracing::{Level, debug, error, info, warn};
use tracing_subscriber::FmtSubscriber;

const MAX_DB_RETRY_ATTEMPTS: u32 = 5;
const DB_RETRY_DELAY_MS: u64 = 2000;

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
    let queue_url = std::env::var("SQS_QUEUE_URL")
        .map_err(|e| eyre::eyre!("SQS_QUEUE_URL environment variable not set: {}", e))?;
    let database_url = std::env::var("DATABASE_URL")
        .map_err(|e| eyre::eyre!("DATABASE_URL environment variable not set: {}", e))?;
    info!("Using SQS Queue URL: {}", queue_url);
    info!("Using database URL: {}", database_url);

    // Load AWS SDK config from environment variables
    // This will respect AWS_ENDPOINT_URL from the .env file
    let config = defaults(BehaviorVersion::latest()).load().await;
    info!("AWS configuration loaded");
    let queue = Arc::new(SqsMessageQueue::new(queue_url, config));

    // Attempt database connection with retries
    let db = connect_to_database_with_retry(&database_url, MAX_DB_RETRY_ATTEMPTS).await?;

    let terminator = Arc::new(AtomicBool::new(false));

    let proof_provider = Arc::new(BonsaiProofProvider::new());

    let processor = ProofJobHandler::new(
        queue.clone(),
        terminator.clone(),
        db.clone(),
        proof_provider,
        std::time::Duration::from_secs(300), // 5 minutes timeout for proof generation
    );

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

/// Attempts to connect to the database with retry logic
async fn connect_to_database_with_retry(
    database_url: &str,
    max_attempts: u32,
) -> Result<Arc<DbConnection>> {
    let mut attempt = 1;

    loop {
        info!(
            "Attempting database connection (attempt {}/{})",
            attempt, max_attempts
        );

        match DbConnection::new(database_url).await {
            Ok(conn) => {
                info!("Successfully connected to the database");
                return Ok(conn);
            }
            Err(e) => {
                if attempt >= max_attempts {
                    error!(
                        "Failed to connect to database after {} attempts: {}",
                        max_attempts, e
                    );
                    return Err(e);
                }

                warn!("Database connection attempt {} failed: {}", attempt, e);
                warn!("Retrying in {} ms...", DB_RETRY_DELAY_MS);

                sleep(Duration::from_millis(DB_RETRY_DELAY_MS)).await;
                attempt += 1;
            }
        }
    }
}
