use crate::domain::entities::Event;
use async_trait::async_trait;

#[async_trait]
pub trait GossipService: Send + Sync {
    async fn join_topic(&self, topic: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn leave_topic(&self, topic: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn broadcast(&self, topic: &str, event: &Event) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn subscribe(&self, topic: &str) -> Result<tokio::sync::mpsc::Receiver<Event>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_joined_topics(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_topic_peers(&self, topic: &str) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>>;
}