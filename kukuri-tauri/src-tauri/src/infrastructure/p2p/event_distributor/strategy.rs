use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::Event;
use crate::domain::p2p::distribution::DistributionStrategy;
use crate::infrastructure::p2p::{GossipService, NetworkService};

use super::default::DefaultEventDistributor;
use super::{DynError, EventDistributor};

/// P2P 配信専用のディストリビューター。
pub struct P2PEventDistributor {
    distributor: DefaultEventDistributor,
}

impl P2PEventDistributor {
    pub fn new() -> Self {
        Self {
            distributor: DefaultEventDistributor::with_strategy(DistributionStrategy::P2P),
        }
    }

    pub async fn set_gossip_service(&self, gossip: Arc<dyn GossipService>) {
        self.distributor.set_gossip_service(gossip).await;
    }

    pub async fn set_network_service(&self, network: Arc<dyn NetworkService>) {
        self.distributor.set_network_service(network).await;
    }

    pub async fn set_default_topics(&self, topics: Vec<String>) {
        self.distributor.set_default_topics(topics).await;
    }
}

impl Default for P2PEventDistributor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventDistributor for P2PEventDistributor {
    async fn distribute(
        &self,
        event: &Event,
        _strategy: DistributionStrategy,
    ) -> Result<(), DynError> {
        self.distributor
            .distribute(event, DistributionStrategy::P2P)
            .await
    }

    async fn receive(&self) -> Result<Option<Event>, DynError> {
        self.distributor.receive().await
    }

    async fn set_strategy(&self, _strategy: DistributionStrategy) {
        // 固定戦略のため何もしない
    }

    async fn get_pending_events(&self) -> Result<Vec<Event>, DynError> {
        self.distributor.get_pending_events().await
    }

    async fn retry_failed(&self) -> Result<u32, DynError> {
        self.distributor.retry_failed().await
    }
}

/// Nostr 配信専用のディストリビューター。
pub struct NostrEventDistributor {
    distributor: DefaultEventDistributor,
}

impl NostrEventDistributor {
    pub fn new() -> Self {
        Self {
            distributor: DefaultEventDistributor::with_strategy(DistributionStrategy::Nostr),
        }
    }

    pub async fn set_default_topics(&self, topics: Vec<String>) {
        self.distributor.set_default_topics(topics).await;
    }
}

impl Default for NostrEventDistributor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventDistributor for NostrEventDistributor {
    async fn distribute(
        &self,
        event: &Event,
        _strategy: DistributionStrategy,
    ) -> Result<(), DynError> {
        self.distributor
            .distribute(event, DistributionStrategy::Nostr)
            .await
    }

    async fn receive(&self) -> Result<Option<Event>, DynError> {
        self.distributor.receive().await
    }

    async fn set_strategy(&self, _strategy: DistributionStrategy) {
        // 固定戦略のため何もしない
    }

    async fn get_pending_events(&self) -> Result<Vec<Event>, DynError> {
        self.distributor.get_pending_events().await
    }

    async fn retry_failed(&self) -> Result<u32, DynError> {
        self.distributor.retry_failed().await
    }
}
