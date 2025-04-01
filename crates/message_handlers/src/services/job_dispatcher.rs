use crate::queue::{message_queue::Queue, sqs_message_queue::SqsMessageQueue};
use eyre::Result;
use serde::Serialize;

#[derive(Serialize)]
pub struct Job {
    pub job_id: String,
    pub start_timestamp: i64,
    pub end_timestamp: i64,
    pub job_group_id: Option<String>,
}

pub struct JobDispatcher {
    queue: SqsMessageQueue,
}

impl JobDispatcher {
    pub fn new(queue: SqsMessageQueue) -> Self {
        Self { queue }
    }

    pub async fn dispatch_job(&self, job: Job) -> Result<String> {
        let message_body = serde_json::to_string(&job)?;
        self.queue
            .send_message(message_body)
            .await
            .map_err(|e| eyre::eyre!(e))?;
        Ok(job.job_id)
    }
}
