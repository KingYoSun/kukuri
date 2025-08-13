use crate::domain::entities::Event;
use crate::shared::error::AppError;
use async_trait::async_trait;

#[async_trait]
pub trait GossipService: Send + Sync {
    async fn join_topic(&self, topic: &str, initial_peers: Vec<String>) -> Result<(), AppError>;
    async fn leave_topic(&self, topic: &str) -> Result<(), AppError>;
    async fn broadcast(&self, topic: &str, event: &Event) -> Result<(), AppError>;
    async fn subscribe(&self, topic: &str) -> Result<tokio::sync::mpsc::Receiver<Event>, AppError>;
    async fn get_joined_topics(&self) -> Result<Vec<String>, AppError>;
    async fn get_topic_peers(&self, topic: &str) -> Result<Vec<String>, AppError>;
    async fn broadcast_message(&self, topic: &str, message: &[u8]) -> Result<(), AppError>;
}