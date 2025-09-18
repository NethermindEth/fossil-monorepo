use std::sync::Arc;

use crate::queue::message_queue::Queue;
use eyre::Result;

use super::jobs::Job;

pub struct JobDispatcher<Q: Queue> {
    queue: Arc<Q>,
}

impl<Q: Queue> JobDispatcher<Q> {
    pub const fn new(queue: Arc<Q>) -> Self {
        Self { queue }
    }

    pub async fn dispatch_job(&self, job: Job) -> Result<()> {
        // Debug logging to see what's being serialized
        if let Job::RequestProof(ref request_proof) = job {
            tracing::info!("🔍 JobDispatcher - Serializing RequestProof: vault_address={:?}, vault_timestamp={:?}",
                request_proof.vault_address, request_proof.vault_timestamp);
        }

        let message_body = serde_json::to_string(&job)?;
        tracing::info!("🔍 JobDispatcher - Sending message to queue: {}", message_body);

        self.queue
            .send_message(message_body)
            .await
            .map_err(|e| eyre::eyre!(e))?;
        Ok(())
    }
}
