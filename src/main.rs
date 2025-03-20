use aws_config::{BehaviorVersion, Region};
use eyre::Result;
use prover_service::queue::sqs_message_queue::SqsMessageQueue;
use prover_service::services::job_dispatcher::Job;
use prover_service::services::{
    job_dispatcher::JobDispatcher, simple_prove_service::ExampleJobProcessor,
};
use std::sync::{Arc, atomic::AtomicBool};
use tracing::debug;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize AWS SQS client
    let config = aws_config::load_defaults(BehaviorVersion::latest())
        .await
        .into_builder()
        .region(Region::new("us-east-1"))
        .build();

    // Create queue URL - replace with your actual queue URL
    let queue_url = "your-sqs-queue-url".to_string();

    let queue = SqsMessageQueue::new(queue_url, config);

    let terminator = Arc::new(AtomicBool::new(false));
    let terminator_clone = terminator.clone();

    let dispatcher = JobDispatcher::new(queue.clone());

    let processor = ExampleJobProcessor::new(queue.clone(), terminator.clone());

    tokio::spawn(async move {
        loop {
            let result = processor.receive_job().await;
            debug!("Job err?: {:?}", result);
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });

    // Handle Ctrl+C for graceful shutdown
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        println!("Received Ctrl+C, initiating shutdown...");
        terminator_clone.store(true, std::sync::atomic::Ordering::Relaxed);
    });

    // Dispatching a job
    dispatcher
        .dispatch_job(Job {
            job_id: "1".to_string(),
            start_timestamp: 1,
            end_timestamp: 2,
        })
        .await?;
    dispatcher
        .dispatch_job(Job {
            job_id: "2".to_string(),
            start_timestamp: 2,
            end_timestamp: 3,
        })
        .await?;

    Ok(())
}
