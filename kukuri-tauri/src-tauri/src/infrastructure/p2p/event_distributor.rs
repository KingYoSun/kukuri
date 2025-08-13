use crate::domain::entities::Event;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub enum DistributionStrategy {
    Broadcast,
    Gossip,
    Direct(String),
    Hybrid,
}

#[async_trait]
pub trait EventDistributor: Send + Sync {
    async fn distribute(&self, event: &Event, strategy: DistributionStrategy) -> Result<(), Box<dyn std::error::Error>>;
    async fn receive(&self) -> Result<Option<Event>, Box<dyn std::error::Error>>;
    async fn set_strategy(&self, strategy: DistributionStrategy);
    async fn get_pending_events(&self) -> Result<Vec<Event>, Box<dyn std::error::Error>>;
    async fn retry_failed(&self) -> Result<u32, Box<dyn std::error::Error>>;
}