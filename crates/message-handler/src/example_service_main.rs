use aws_config::BehaviorVersion;
use aws_config::load_defaults;
use eyre::Result;
use message_handler::queue::sqs_message_queue::SqsMessageQueue;
use message_handler::services::example_message_handler::ExampleMessageHandler;
use message_handler::services::job_dispatcher::JobDispatcher;
use message_handler::services::jobs::{Job, RequestProof};
use std::sync::{Arc, atomic::AtomicBool};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::join;
use tracing::debug;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    // This example tests with real sqs, but you can replace this with a local queue.
    let queue_url = std::env::var("SQS_QUEUE_URL").expect("SQS_QUEUE_URL must be set");

    // Configure tracing
    tracing_subscriber::fmt().init();

    // Initialize AWS SQS client
    let config = load_defaults(BehaviorVersion::latest()).await;

    let queue = Arc::new(SqsMessageQueue::new(queue_url, config));

    let terminator = Arc::new(AtomicBool::new(false));

    let dispatcher = JobDispatcher::new(queue.clone());

    let processor = ExampleMessageHandler::new(queue.clone(), terminator.clone());

    let processor_handle = tokio::spawn(async move {
        let result = processor.receive_job().await;
        debug!("Handler err?: {:?}", result);
    });

    // Dispatching a job
    let terminator_clone = terminator.clone();
    let dispatcher_handle = tokio::spawn(async move {
        let mut i: u128 = 1;
        while !terminator_clone.load(std::sync::atomic::Ordering::Relaxed) {
            let result = dispatcher
                .dispatch_job(Job::RequestProof(RequestProof {
                    job_id: i.to_string(),
                    job_group_id: None,
                    start_timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64,
                    end_timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64,
                }))
                .await;
            println!("Job dispatched: {:?}", result);
            i += 1;
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });

    // Handle Ctrl+C for graceful shutdown
    let terminator_clone = terminator.clone();
    let terminator_handle = tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        debug!("Received Ctrl+C, initiating shutdown...");
        terminator_clone.store(true, std::sync::atomic::Ordering::Relaxed);
    });

    let result = join!(dispatcher_handle, processor_handle, terminator_handle);
    debug!("Result: {:?}", result);

    Ok(())
}
