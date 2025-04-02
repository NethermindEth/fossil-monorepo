use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::message_queue::{Queue, QueueError, QueueMessage};

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
            id: None,
            body: message,
        });
        Ok(())
    }

    async fn receive_messages(&self) -> Result<Vec<QueueMessage>, QueueError> {
        let mut messages = self.messages.lock().await;
        let result = messages.clone();
        messages.clear();
        Ok(result)
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

    #[tokio::test]
    async fn test_receive_clears_queue() {
        let queue = LocalMessageQueue::new();
        queue.send_message("test".to_string()).await.unwrap();

        // First receive should get the message
        let first_receive = queue.receive_messages().await.unwrap();
        assert_eq!(first_receive.len(), 1);

        // Second receive should be empty
        let second_receive = queue.receive_messages().await.unwrap();
        assert!(second_receive.is_empty());
    }
}
