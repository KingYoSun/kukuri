use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use bincode::serde::encode_to_vec;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::domain::entities::Event;
use crate::domain::p2p::distribution::{DistributionMetrics, DistributionStrategy};
use crate::infrastructure::p2p::{GossipService, NetworkService};
use crate::shared::error::AppError;

use super::state::DistributorState;
use super::{DynError, EventDistributor, P2PDistributionMetrics};

/// デフォルトの EventDistributor 実装。
pub struct DefaultEventDistributor {
    state: Arc<RwLock<DistributorState>>,
    metrics: Arc<dyn DistributionMetrics>,
    gossip_service: Arc<RwLock<Option<Arc<dyn GossipService>>>>,
    network_service: Arc<RwLock<Option<Arc<dyn NetworkService>>>>,
    default_topics: Arc<RwLock<Vec<String>>>,
}

impl DefaultEventDistributor {
    pub fn new() -> Self {
        Self::with_strategy(DistributionStrategy::Hybrid)
    }

    pub fn with_strategy(strategy: DistributionStrategy) -> Self {
        Self::with_strategy_and_metrics(
            strategy,
            Arc::new(P2PDistributionMetrics) as Arc<dyn DistributionMetrics>,
        )
    }

    pub fn with_strategy_and_metrics(
        strategy: DistributionStrategy,
        metrics: Arc<dyn DistributionMetrics>,
    ) -> Self {
        Self {
            state: Arc::new(RwLock::new(DistributorState::new(strategy, 3))),
            metrics,
            gossip_service: Arc::new(RwLock::new(None)),
            network_service: Arc::new(RwLock::new(None)),
            default_topics: Arc::new(RwLock::new(vec!["public".to_string()])),
        }
    }

    pub async fn set_gossip_service(&self, gossip: Arc<dyn GossipService>) {
        let mut guard = self.gossip_service.write().await;
        *guard = Some(gossip);
    }

    pub async fn set_network_service(&self, network: Arc<dyn NetworkService>) {
        let mut guard = self.network_service.write().await;
        *guard = Some(network);
    }

    pub async fn set_default_topics(&self, topics: Vec<String>) {
        let mut guard = self.default_topics.write().await;
        guard.clear();
        guard.extend(
            topics
                .into_iter()
                .filter(|topic| !topic.trim().is_empty())
                .map(|topic| topic.trim().to_string()),
        );
        if guard.is_empty() {
            guard.push("public".to_string());
        }
    }

    async fn resolve_topics(&self, event: &Event) -> Vec<String> {
        let mut topics: HashSet<String> = event
            .tags
            .iter()
            .filter_map(|tag| {
                if tag.is_empty() {
                    return None;
                }
                match tag[0].as_str() {
                    "topic" | "t" => tag.get(1).cloned(),
                    _ => None,
                }
            })
            .filter(|topic| !topic.trim().is_empty())
            .map(|topic| topic.trim().to_string())
            .collect();

        if topics.is_empty() {
            let defaults = self.default_topics.read().await;
            topics.extend(defaults.iter().cloned());
        }

        let mut resolved: Vec<String> = topics.into_iter().collect();
        resolved.sort();
        resolved
    }

    async fn broadcast_via_gossip(&self, event: &Event, topics: &[String]) -> Result<(), DynError> {
        if topics.is_empty() {
            return Ok(());
        }

        let gossip = {
            let guard = self.gossip_service.read().await;
            guard.clone()
        };

        let Some(gossip) = gossip else {
            warn!("GossipService not configured; skipping gossip broadcast");
            return Ok(());
        };

        for topic in topics {
            if topic.is_empty() {
                continue;
            }

            if let Err(err) = gossip.join_topic(topic, Vec::new()).await {
                warn!("Joining topic {topic} failed before broadcast: {err}");
            }

            gossip.broadcast(topic, event).await.map_err(|err| {
                warn!(
                    "Failed to broadcast event {} on topic {} via gossip: {}",
                    event.id, topic, err
                );
                Box::new(err) as DynError
            })?;
        }

        Ok(())
    }

    async fn broadcast_via_network(
        &self,
        event: &Event,
        topics: &[String],
    ) -> Result<(), DynError> {
        if topics.is_empty() {
            return Ok(());
        }

        let network = {
            let guard = self.network_service.read().await;
            guard.clone()
        };

        let Some(network) = network else {
            return Ok(());
        };

        let payload = encode_to_vec(event, bincode::config::standard()).map_err(|err| {
            Box::new(AppError::SerializationError(format!(
                "Failed to serialize event for DHT broadcast: {err}"
            ))) as DynError
        })?;

        for topic in topics {
            if topic.is_empty() {
                continue;
            }

            if let Err(err) = network.join_dht_topic(topic).await {
                warn!("Failed to join DHT topic {topic} before broadcast: {}", err);
            }

            network
                .broadcast_dht(topic, payload.clone())
                .await
                .map_err(|err| Box::new(err) as DynError)?;
        }

        Ok(())
    }

    async fn broadcast_p2p(&self, event: &Event, topics: &[String]) -> Result<(), DynError> {
        self.broadcast_via_gossip(event, topics).await?;
        self.broadcast_via_network(event, topics).await?;
        Ok(())
    }

    async fn broadcast_direct(&self, event: &Event, peer_id: &str) -> Result<(), DynError> {
        if peer_id.trim().is_empty() {
            warn!(
                "Direct distribution requested with empty peer id; falling back to standard P2P broadcast"
            );
        } else {
            let network = {
                let guard = self.network_service.read().await;
                guard.clone()
            };

            if let Some(network) = network {
                if peer_id.contains('@') {
                    if let Err(err) = network.add_peer(peer_id).await {
                        warn!("Failed to add peer {peer_id} for direct distribution: {err}");
                    }
                } else {
                    warn!(
                        "Peer id \"{peer_id}\" is not in node_id@address format; skipping add_peer"
                    );
                }
            } else {
                warn!(
                    "NetworkService not configured; direct distribution cannot target specific peer"
                );
            }
        }

        let topics = self.resolve_topics(event).await;
        self.broadcast_p2p(event, &topics).await
    }

    async fn distribute_internal(
        &self,
        event: &Event,
        strategy: &DistributionStrategy,
    ) -> Result<(), DynError> {
        match strategy {
            DistributionStrategy::Broadcast => {
                debug!("Broadcasting event {} to all peers", event.id);
                let topics = self.resolve_topics(event).await;
                self.broadcast_p2p(event, &topics).await?;
                info!("Event {} broadcasted via gossip/DHT", event.id);
                Ok(())
            }
            DistributionStrategy::Gossip => {
                debug!("Distributing event {} via Gossip protocol", event.id);
                let topics = self.resolve_topics(event).await;
                self.broadcast_via_gossip(event, &topics).await?;
                info!("Event {} distributed via gossip", event.id);
                Ok(())
            }
            DistributionStrategy::Direct(peer_id) => {
                debug!("Sending event {} directly to peer {}", event.id, peer_id);
                self.broadcast_direct(event, peer_id).await?;
                info!(
                    "Event {} distributed directly (peer {}) via fallback P2P path",
                    event.id, peer_id
                );
                Ok(())
            }
            DistributionStrategy::Hybrid => {
                debug!("Distributing event {} using hybrid strategy", event.id);
                Box::pin(self.distribute_internal(event, &DistributionStrategy::Nostr)).await?;
                Box::pin(self.distribute_internal(event, &DistributionStrategy::P2P)).await?;
                Ok(())
            }
            DistributionStrategy::Nostr => {
                debug!("Distributing event {} via Nostr relays", event.id);
                info!(
                    "Event {} marked for Nostr distribution (handled upstream)",
                    event.id
                );
                Ok(())
            }
            DistributionStrategy::P2P => {
                debug!("Distributing event {} via P2P network", event.id);
                let topics = self.resolve_topics(event).await;
                self.broadcast_p2p(event, &topics).await?;
                info!("Event {} distributed via P2P", event.id);
                Ok(())
            }
        }
    }

    #[cfg(test)]
    pub(crate) async fn current_strategy(&self) -> DistributionStrategy {
        self.state.read().await.strategy()
    }
}

impl Default for DefaultEventDistributor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventDistributor for DefaultEventDistributor {
    async fn distribute(
        &self,
        event: &Event,
        strategy: DistributionStrategy,
    ) -> Result<(), DynError> {
        debug!(
            "Distributing event {} with strategy {:?}",
            event.id, strategy
        );
        self.metrics.record_attempt(&strategy);

        match self.distribute_internal(event, &strategy).await {
            Ok(()) => {
                info!("Event {} distributed successfully", event.id);
                self.metrics.record_success(&strategy);
                Ok(())
            }
            Err(e) => {
                error!("Failed to distribute event {}: {}", event.id, e);
                self.metrics.record_failure(&strategy);

                let mut state = self.state.write().await;
                state.record_failure(event.clone(), strategy);

                Err(e)
            }
        }
    }

    async fn receive(&self) -> Result<Option<Event>, DynError> {
        let mut state = self.state.write().await;
        Ok(state.pop_pending())
    }

    async fn set_strategy(&self, strategy: DistributionStrategy) {
        let mut state = self.state.write().await;
        debug!("Setting distribution strategy to {:?}", strategy);
        state.set_strategy(strategy);
    }

    async fn get_pending_events(&self) -> Result<Vec<Event>, DynError> {
        let state = self.state.read().await;
        Ok(state.pending_events_snapshot())
    }

    async fn retry_failed(&self) -> Result<u32, DynError> {
        let mut state = self.state.write().await;
        let failed_events = state.drain_failures();
        let mut retry_count = 0;
        let mut still_failed = Vec::new();

        for (event, strategy) in failed_events {
            debug!("Retrying distribution for event {}", event.id);
            match self.distribute_internal(&event, &strategy).await {
                Ok(()) => {
                    info!("Event {} successfully distributed on retry", event.id);
                    self.metrics.record_success(&strategy);
                    retry_count += 1;
                }
                Err(e) => {
                    error!("Event {} failed again on retry: {}", event.id, e);
                    self.metrics.record_failure(&strategy);
                    still_failed.push((event, strategy));
                }
            }
        }

        state.restore_failures(still_failed);

        Ok(retry_count)
    }
}
