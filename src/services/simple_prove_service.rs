use std::sync::{Arc, atomic::AtomicBool};

use crate::queue::{message_queue::Queue, sqs_message_queue::SqsMessageQueue};
use eyre::Result;
use serde::Deserialize;
use tokio::task;
use tracing::warn;

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
        while !self.terminator.load(std::sync::atomic::Ordering::Relaxed) {
            let messages = self.queue.receive_messages().await;

            match messages {
                Ok(messages) => {
                    for message in messages {
                        let job: Job = serde_json::from_str(&message.body)?;

                        task::spawn(async move {
                            println!("Received & processing job: {:?}", job);
                            // TODO: Implement the logic to process the job
                        });
                    }
                }
                Err(e) => {
                    warn!("Error receiving messages from queue: {}", e);
                }
            }
        }
        Ok(())
    }
}
