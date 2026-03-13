use async_trait::async_trait;

use crate::domain::entities::Event;
use crate::domain::p2p::distribution::DistributionStrategy;

mod default;
mod metrics;
mod state;
mod strategy;

pub use default::DefaultEventDistributor;
pub use metrics::P2PDistributionMetrics;
pub use strategy::{NostrEventDistributor, P2PEventDistributor};

pub type DynError = Box<dyn std::error::Error + Send + Sync>;

#[async_trait]
pub trait EventDistributor: Send + Sync {
    async fn distribute(
        &self,
        event: &Event,
        strategy: DistributionStrategy,
    ) -> Result<(), DynError>;
    async fn receive(&self) -> Result<Option<Event>, DynError>;
    async fn set_strategy(&self, strategy: DistributionStrategy);
    async fn get_pending_events(&self) -> Result<Vec<Event>, DynError>;
    async fn retry_failed(&self) -> Result<u32, DynError>;
}

#[cfg(test)]
mod tests;
