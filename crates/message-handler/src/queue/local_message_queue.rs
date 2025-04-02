use super::message_queue::{Queue, QueueError, QueueMessage};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::warn;
use uuid::Uuid;

pub struct LocalMessageQueue {
    messages: Arc<Mutex<Vec<QueueMessage>>>,
}

impl LocalMessageQueue {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Default for LocalMessageQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Queue for LocalMessageQueue {
    async fn send_message(&self, message: String) -> Result<(), QueueError> {
        let mut messages = self.messages.lock().await;
        messages.push(QueueMessage {
            id: Some(Uuid::new_v4().to_string()),
            body: message,
        });
        Ok(())
    }

    async fn receive_messages(&self) -> Result<Vec<QueueMessage>, QueueError> {
        let messages = self.messages.lock().await;
        let result = messages.clone();
        Ok(result)
    }

    async fn delete_message(&self, message: &QueueMessage) -> Result<(), QueueError> {
        let mut messages = self.messages.lock().await;
        let index = if let Some(index) = messages.iter().position(|m| m.id == message.id) {
            index
        } else {
            warn!("Message not found, skipping delete");
            return Ok(());
        };
        messages.remove(index);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new_queue_is_empty() {
        let queue = LocalMessageQueue::new();
        let messages = queue.receive_messages().await.unwrap();
        assert!(messages.is_empty());
    }

    #[tokio::test]
    async fn test_send_single_message() {
        let queue = LocalMessageQueue::new();
        let test_message = "test message".to_string();

        queue.send_message(test_message.clone()).await.unwrap();

        let messages = queue.receive_messages().await.unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].body, test_message);
    }

    #[tokio::test]
    async fn test_send_multiple_messages() {
        let queue = LocalMessageQueue::new();
        let messages = ["first", "second", "third"];

        for msg in messages.iter() {
            queue.send_message(msg.to_string()).await.unwrap();
        }

        let received = queue.receive_messages().await.unwrap();
        assert_eq!(received.len(), 3);
        for (i, msg) in messages.iter().enumerate() {
            assert_eq!(received[i].body, *msg);
        }
    }
}
