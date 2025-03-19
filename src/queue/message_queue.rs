use async_trait::async_trait;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct QueueMessage {
    pub body: String,       // TODO: should we make this generic?
    pub id: Option<String>, // Mainly for debugging & logging
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum QueueError {
    SendError(String),
    ReceiveError(String),
}

#[allow(dead_code)]
#[async_trait]
pub trait Queue {
    async fn send_message(&self, message: String) -> Result<(), QueueError>;

    async fn receive_messages(&self) -> Result<Vec<QueueMessage>, QueueError>;
}
