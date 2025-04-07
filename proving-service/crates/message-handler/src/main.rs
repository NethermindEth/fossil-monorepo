use aws_config::{BehaviorVersion, defaults};
use eyre::Result;
#[cfg(feature = "mock-proof")]
use message_handler::proof_composition::BonsaiProofProvider;
use message_handler::proof_composition::ProofProvider;
use message_handler::queue::sqs_message_queue::SqsMessageQueue;
use message_handler::services::proof_job_handler::ProofJobHandler;
use std::sync::{Arc, atomic::AtomicBool};
use tokio::signal;
use tracing::{Level, debug, info};
use tracing_subscriber::FmtSubscriber;

// Create a no-op proof provider that implements the ProofProvider trait
mod no_op {
    use async_trait::async_trait;
    use eyre::{Result, eyre};
    use message_handler::proof_composition::{ProofProvider, ProofTimestampRanges};
    use risc0_zkvm::Receipt;

    #[derive(Debug, Clone)]
    pub struct NoOpProofProvider;

    impl NoOpProofProvider {
        pub const fn new() -> Self {
            Self
        }
    }

    #[async_trait]
    impl ProofProvider for NoOpProofProvider {
        async fn generate_proofs_from_data(
            &self,
            _timestamp_ranges: ProofTimestampRanges,
        ) -> Result<Receipt> {
            Err(eyre!(
                "Proof functionality is disabled. Set ENABLE_PROOF=true and enable either the 'proof-composition' or 'mock-proof' feature to use this functionality."
            ))
        }

        fn is_disabled(&self) -> bool {
            true
        }
    }
}

#[cfg(any(feature = "proof-composition", feature = "mock-proof"))]
mod disabled_provider {
    use async_trait::async_trait;
    use eyre::{Result, eyre};
    use message_handler::proof_composition::{ProofProvider, ProofTimestampRanges};
    use risc0_zkvm::Receipt;

    #[derive(Debug, Clone)]
    pub struct NoOpProofProvider;

    impl NoOpProofProvider {
        // pub const fn new() -> Self {
        //     Self
        // }
    }

    #[async_trait]
    impl ProofProvider for NoOpProofProvider {
        async fn generate_proofs_from_data(
            &self,
            _timestamp_ranges: ProofTimestampRanges,
        ) -> Result<Receipt> {
            Err(eyre!(
                "Proof functionality is disabled. Set ENABLE_PROOF=true and enable either the 'proof-composition' or 'mock-proof' feature to use this functionality."
            ))
        }

        fn is_disabled(&self) -> bool {
            true
        }
    }
}

// Create a very simple mock proof provider that doesn't use any external libraries
// This is useful for testing when we want to avoid the complexity of the full mock-proof feature
mod simple_mock {
    use async_trait::async_trait;
    use eyre::Result;
    use message_handler::proof_composition::{ProofProvider, ProofTimestampRanges};
    use risc0_zkvm::{Digest, FakeReceipt, InnerReceipt, MaybePruned, Receipt};

    #[derive(Debug, Clone)]
    pub struct SimpleMockProofProvider;

    impl SimpleMockProofProvider {
        pub const fn new() -> Self {
            Self
        }
    }

    #[async_trait]
    impl ProofProvider for SimpleMockProofProvider {
        async fn generate_proofs_from_data(
            &self,
            _timestamp_ranges: ProofTimestampRanges,
        ) -> Result<Receipt> {
            // Just create a fake receipt for testing
            // This doesn't rely on any external crates that might create Tokio runtime issues
            use std::time::Duration;
            use tokio::time::sleep;

            // Add a small delay to simulate proof generation time
            sleep(Duration::from_millis(100)).await;

            // Create a dummy receipt
            let fake_receipt = FakeReceipt::new(MaybePruned::Pruned(Digest::ZERO));
            let receipt = Receipt::new(InnerReceipt::Fake(fake_receipt), vec![]);

            Ok(receipt)
        }

        fn is_disabled(&self) -> bool {
            false
        }
    }
}

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

    let terminator = Arc::new(AtomicBool::new(false));

    // Check if proof generation is enabled via environment variable
    let enable_proof = std::env::var("ENABLE_PROOF").unwrap_or_else(|_| "false".to_string());
    let enable_proof = enable_proof.to_lowercase() == "true";

    // Check if simple mock is requested
    let use_simple_mock = std::env::var("USE_SIMPLE_MOCK").unwrap_or_else(|_| "false".to_string());
    let use_simple_mock = use_simple_mock.to_lowercase() == "true";

    // Create the proper proof provider based on feature flags and environment variable
    let proof_provider: Arc<dyn ProofProvider + Send + Sync> = if enable_proof {
        if use_simple_mock {
            info!("Using simple mock proof provider (for testing)");
            Arc::new(simple_mock::SimpleMockProofProvider::new())
        } else {
            #[cfg(any(feature = "proof-composition", feature = "mock-proof"))]
            {
                info!("Proof generation is enabled");
                Arc::new(BonsaiProofProvider::new())
            }
            #[cfg(not(any(feature = "proof-composition", feature = "mock-proof")))]
            {
                info!("Proof generation is enabled but no proof features are compiled in");
                Arc::new(no_op::NoOpProofProvider::new())
            }
        }
    } else {
        info!("Proof generation is disabled via ENABLE_PROOF environment variable");
        Arc::new(no_op::NoOpProofProvider::new())
    };

    // Note: Configure SQS with a suitable visibility timeout (300s) to match the proof generation timeout
    // This helps prevent the same message from being processed multiple times
    let processor = ProofJobHandler::new(
        queue.clone(),
        terminator.clone(),
        proof_provider,
        std::time::Duration::from_secs(300), // 5 minutes timeout for proof generation
    );

    // Create an async closure to wrap the job processing logic
    let process_job_handle = tokio::spawn(async move {
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

    // Wait for the processor to finish with a timeout to avoid hanging
    info!("Waiting for processor to finish...");
    tokio::select! {
        _ = process_job_handle => {
            info!("Processor completed gracefully");
        }
        _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {
            info!("Processor shutdown timed out after 10 seconds, proceeding with shutdown");
            // We're not aborting the task, just proceeding with shutdown
        }
    }

    info!("Shutdown complete");
    Ok(())
}
