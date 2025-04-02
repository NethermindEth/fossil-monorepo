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
                let mut queue_messages = Vec::new();
                for message in messages {
                    queue_messages.push(QueueMessage {
                        body: message.body().unwrap_or("").to_string(),
                        id: message.message_id.clone(),
                    });

                    if let Some(receipt_handle) = message.receipt_handle() {
                        // Delete the message from the queue after processing
                        if let Err(e) = self
                            .client
                            .delete_message()
                            .queue_url(self.queue_url.clone())
                            .receipt_handle(receipt_handle)
                            .send()
                            .await
                        {
                            warn!("Error deleting message from SQS: {}", e);
                        }
                    }
                    debug!(
                        "Processed message: {}",
                        message
                            .message_id
                            .clone()
                            .unwrap_or_else(|| "no message id".to_string())
                    );
                }

                let queue_messages = messages
                    .iter()
                    .map(|m| QueueMessage {
                        body: m.body().unwrap_or("").to_string(),
                        id: m.message_id.clone(),
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_instance() {
        // Create a minimal configuration for testing
        let config = aws_config::SdkConfig::builder()
            .behavior_version(aws_config::BehaviorVersion::latest())
            .build();

        let queue_url = "https://test-queue-url".to_string();
        let queue = SqsMessageQueue::new(queue_url.clone(), config);

        assert_eq!(queue.queue_url, queue_url);
        // We can't easily test the client as it doesn't implement PartialEq
    }
}
