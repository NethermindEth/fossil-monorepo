use std::sync::{Arc, atomic::AtomicBool};

use crate::queue::{message_queue::Queue, sqs_message_queue::SqsMessageQueue};
use eyre::Result;
use serde::Deserialize;
use tokio::task;
use tracing::{info, warn};

#[derive(Deserialize, Debug)]
pub struct Job {
    pub job_id: String,
    pub start_timestamp: i64,
    pub end_timestamp: i64,
}

pub struct ExampleJobProcessor {
    queue: SqsMessageQueue,
    terminator: Arc<AtomicBool>,
}

impl ExampleJobProcessor {
    pub fn new(queue: SqsMessageQueue, terminator: Arc<AtomicBool>) -> Self {
        Self { queue, terminator }
    }

    pub async fn receive_job(&self) -> Result<()> {
        info!("Job processor started, waiting for messages");

        while !self.terminator.load(std::sync::atomic::Ordering::Relaxed) {
            if self.terminator.load(std::sync::atomic::Ordering::Relaxed) {
                info!("Termination signal received, stopping message processing");
                break;
            }

            let messages = self.queue.receive_messages().await;

            match messages {
                Ok(messages) => {
                    if !messages.is_empty() {
                        info!("Received {} messages", messages.len());
                    }

                    for message in messages {
                        // Check termination flag before processing each message
                        if self.terminator.load(std::sync::atomic::Ordering::Relaxed) {
                            info!("Termination signal received, stopping message processing");
                            break;
                        }

                        let job: Job = serde_json::from_str(&message.body)?;

                        task::spawn(async move {
                            info!("Processing job: {:?}", job);
                            // TODO: Implement the logic to process the job
                        });
                    }
                }
                Err(e) => {
                    warn!("Error receiving messages from queue: {}", e);
                }
            }

            // Small sleep to prevent tight loops and excessive CPU usage
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        info!("Job processor shutting down");
        Ok(())
    }
}
