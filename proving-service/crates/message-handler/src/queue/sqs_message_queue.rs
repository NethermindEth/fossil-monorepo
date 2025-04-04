use async_trait::async_trait;
use aws_sdk_sqs::Client;
use tracing::{debug, warn};

use super::message_queue::{Queue, QueueError, QueueMessage};

#[derive(Debug, Clone)]
pub struct SqsMessageQueue {
    queue_url: String,
    client: Client,
}

impl SqsMessageQueue {
    pub fn new(queue_url: String, aws_config: aws_config::SdkConfig) -> Self {
        let client = Client::new(&aws_config);
        Self { client, queue_url }
    }
}

#[async_trait]
impl Queue for SqsMessageQueue {
    async fn send_message(&self, message: String) -> Result<(), QueueError> {
        match self
            .client
            .send_message()
            .queue_url(self.queue_url.clone())
            .message_body(message)
            .send()
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                warn!("Error sending message to SQS: {}", e);
                Err(QueueError::SendError(e.to_string()))
            }
        }
    }

    async fn receive_messages(&self) -> Result<Vec<QueueMessage>, QueueError> {
        let receive_res = self
            .client
            .receive_message()
            .queue_url(self.queue_url.clone())
            .wait_time_seconds(20)
            .max_number_of_messages(10)
            .send()
            .await;

        match receive_res {
            Ok(receive_resp) => {
                let messages = receive_resp.messages();

                let queue_messages = messages
                    .iter()
                    .map(|m| QueueMessage {
                        body: m.body().unwrap_or("").to_string(),
                        id: m.receipt_handle.clone(),
                    })
                    .collect();

                Ok(queue_messages)
            }
            Err(e) => {
                warn!("Error receiving messages from SQS: {}", e);
                Err(QueueError::ReceiveError(e.to_string()))
            }
        }
    }

    async fn delete_message(&self, message: &QueueMessage) -> Result<(), QueueError> {
        if let Some(message_id) = &message.id {
            match self
                .client
                .delete_message()
                .queue_url(self.queue_url.clone())
                .receipt_handle(message_id.to_owned())
                .send()
                .await
            {
                Ok(_) => Ok(()),
                Err(e) => {
                    warn!("Error deleting message from SQS: {}", e);
                    Err(QueueError::DeleteError(e.to_string()))
                }
            }
        } else {
            debug!(
                "Processed message: {}",
                message
                    .id
                    .clone()
                    .unwrap_or_else(|| "no message id".to_string())
            );
            // Its okay if the message doesn't have an id, we can't delete it
            // but we also shouldn't error out.
            Ok(())
        }
    }
}
