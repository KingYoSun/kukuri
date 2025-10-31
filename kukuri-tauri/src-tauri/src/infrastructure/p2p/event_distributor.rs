use super::{metrics, GossipService, NetworkService};
use crate::domain::entities::Event;
use crate::domain::p2p::distribution::{DistributionMetrics, DistributionStrategy};
use crate::shared::error::AppError;
use async_trait::async_trait;
use bincode::serde::encode_to_vec;
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

type DynError = Box<dyn std::error::Error + Send + Sync>;

#[async_trait]
pub trait EventDistributor: Send + Sync {
    async fn distribute(
        &self,
        event: &Event,
        strategy: DistributionStrategy,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn receive(&self) -> Result<Option<Event>, Box<dyn std::error::Error + Send + Sync>>;
    async fn set_strategy(&self, strategy: DistributionStrategy);
    async fn get_pending_events(
        &self,
    ) -> Result<Vec<Event>, Box<dyn std::error::Error + Send + Sync>>;
    async fn retry_failed(&self) -> Result<u32, Box<dyn std::error::Error + Send + Sync>>;
}

/// デフォルトのEventDistributor実装
pub struct DefaultEventDistributor {
    inner: Arc<RwLock<EventDistributorInner>>,
    metrics: Arc<dyn DistributionMetrics>,
    gossip_service: Arc<RwLock<Option<Arc<dyn GossipService>>>>,
    network_service: Arc<RwLock<Option<Arc<dyn NetworkService>>>>,
    default_topics: Arc<RwLock<Vec<String>>>,
}

struct EventDistributorInner {
    strategy: DistributionStrategy,
    pending_events: VecDeque<Event>,
    failed_events: Vec<(Event, DistributionStrategy)>,
    max_retries: u32,
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
            inner: Arc::new(RwLock::new(EventDistributorInner {
                strategy,
                pending_events: VecDeque::new(),
                failed_events: Vec::new(),
                max_retries: 3,
            })),
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
                warn!("Failed to broadcast event {} on topic {} via gossip: {}", event.id, topic, err);
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
                warn!(
                    "Failed to join DHT topic {topic} before broadcast: {}",
                    err
                );
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
            warn!("Direct distribution requested with empty peer id; falling back to standard P2P broadcast");
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
                warn!("NetworkService not configured; direct distribution cannot target specific peer");
            }
        }

        let topics = self.resolve_topics(event).await;
        self.broadcast_p2p(event, &topics).await
    }

    /// 実際の配信処理（プライベートメソッド）
    async fn distribute_internal(
        &self,
        event: &Event,
        strategy: &DistributionStrategy,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
                // NostrとP2Pの両方で配信
                Box::pin(self.distribute_internal(event, &DistributionStrategy::Nostr)).await?;
                Box::pin(self.distribute_internal(event, &DistributionStrategy::P2P)).await?;
                Ok(())
            }
            DistributionStrategy::Nostr => {
                debug!("Distributing event {} via Nostr relays", event.id);
                // EventGateway 経由で処理されるため、ここでは成功扱いとする
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
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!(
            "Distributing event {} with strategy {:?}",
            event.id, strategy
        );
        self.metrics.record_attempt(&strategy);

        // 配信を試行
        match self.distribute_internal(event, &strategy).await {
            Ok(()) => {
                info!("Event {} distributed successfully", event.id);
                self.metrics.record_success(&strategy);
                Ok(())
            }
            Err(e) => {
                error!("Failed to distribute event {}: {}", event.id, e);
                self.metrics.record_failure(&strategy);

                // 失敗したイベントを記録
                let mut inner = self.inner.write().await;
                inner.failed_events.push((event.clone(), strategy));

                Err(e)
            }
        }
    }

    async fn receive(&self) -> Result<Option<Event>, Box<dyn std::error::Error + Send + Sync>> {
        let mut inner = self.inner.write().await;
        Ok(inner.pending_events.pop_front())
    }

    async fn set_strategy(&self, strategy: DistributionStrategy) {
        let mut inner = self.inner.write().await;
        debug!("Setting distribution strategy to {:?}", strategy);
        inner.strategy = strategy;
    }

    async fn get_pending_events(
        &self,
    ) -> Result<Vec<Event>, Box<dyn std::error::Error + Send + Sync>> {
        let inner = self.inner.read().await;
        Ok(inner.pending_events.iter().cloned().collect())
    }

    async fn retry_failed(&self) -> Result<u32, Box<dyn std::error::Error + Send + Sync>> {
        let mut inner = self.inner.write().await;
        let failed_events = std::mem::take(&mut inner.failed_events);
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

        // まだ失敗しているイベントを戻す
        inner.failed_events = still_failed;

        Ok(retry_count)
    }
}

struct P2PDistributionMetrics;

impl DistributionMetrics for P2PDistributionMetrics {
    fn record_success(&self, strategy: &DistributionStrategy) {
        match strategy {
            DistributionStrategy::Nostr => metrics::record_broadcast_success(),
            DistributionStrategy::P2P
            | DistributionStrategy::Broadcast
            | DistributionStrategy::Gossip
            | DistributionStrategy::Direct(_)
            | DistributionStrategy::Hybrid => metrics::record_broadcast_success(),
        }
    }

    fn record_failure(&self, strategy: &DistributionStrategy) {
        match strategy {
            DistributionStrategy::Nostr => metrics::record_broadcast_failure(),
            DistributionStrategy::P2P
            | DistributionStrategy::Broadcast
            | DistributionStrategy::Gossip
            | DistributionStrategy::Direct(_)
            | DistributionStrategy::Hybrid => metrics::record_broadcast_failure(),
        }
    }
}

/// P2P配信専用実装
pub struct P2PEventDistributor {
    distributor: DefaultEventDistributor,
}

impl P2PEventDistributor {
    pub fn new() -> Self {
        Self {
            distributor: DefaultEventDistributor::with_strategy(DistributionStrategy::P2P),
        }
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
        _strategy: DistributionStrategy, // P2Pのみ使用
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.distributor
            .distribute(event, DistributionStrategy::P2P)
            .await
    }

    async fn receive(&self) -> Result<Option<Event>, Box<dyn std::error::Error + Send + Sync>> {
        self.distributor.receive().await
    }

    async fn set_strategy(&self, _strategy: DistributionStrategy) {
        // P2P専用なので変更しない
    }

    async fn get_pending_events(
        &self,
    ) -> Result<Vec<Event>, Box<dyn std::error::Error + Send + Sync>> {
        self.distributor.get_pending_events().await
    }

    async fn retry_failed(&self) -> Result<u32, Box<dyn std::error::Error + Send + Sync>> {
        self.distributor.retry_failed().await
    }
}

/// Nostr配信専用実装
pub struct NostrEventDistributor {
    distributor: DefaultEventDistributor,
}

impl NostrEventDistributor {
    pub fn new() -> Self {
        Self {
            distributor: DefaultEventDistributor::with_strategy(DistributionStrategy::Nostr),
        }
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
        _strategy: DistributionStrategy, // Nostrのみ使用
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.distributor
            .distribute(event, DistributionStrategy::Nostr)
            .await
    }

    async fn receive(&self) -> Result<Option<Event>, Box<dyn std::error::Error + Send + Sync>> {
        self.distributor.receive().await
    }

    async fn set_strategy(&self, _strategy: DistributionStrategy) {
        // Nostr専用なので変更しない
    }

    async fn get_pending_events(
        &self,
    ) -> Result<Vec<Event>, Box<dyn std::error::Error + Send + Sync>> {
        self.distributor.get_pending_events().await
    }

    async fn retry_failed(&self) -> Result<u32, Box<dyn std::error::Error + Send + Sync>> {
        self.distributor.retry_failed().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::EventKind;
    use crate::shared::error::AppError;
    use async_trait::async_trait;
    use std::collections::HashSet;
    use std::sync::Arc;
    use tokio::sync::{mpsc, Mutex};

    #[derive(Clone, Default)]
    struct DummyGossipService {
        broadcasts: Arc<Mutex<Vec<(String, String)>>>,
        joined: Arc<Mutex<HashSet<String>>>,
    }

    impl DummyGossipService {
        fn new() -> Self {
            Self::default()
        }

        async fn broadcast_count(&self) -> usize {
            let guard = self.broadcasts.lock().await;
            guard.len()
        }
    }

    #[async_trait]
    impl GossipService for DummyGossipService {
        fn local_peer_hint(&self) -> Option<String> {
            None
        }

        async fn join_topic(&self, topic: &str, _initial_peers: Vec<String>) -> Result<(), AppError> {
            let mut guard = self.joined.lock().await;
            guard.insert(topic.to_string());
            Ok(())
        }

        async fn leave_topic(&self, topic: &str) -> Result<(), AppError> {
            let mut guard = self.joined.lock().await;
            guard.remove(topic);
            Ok(())
        }

        async fn broadcast(&self, topic: &str, event: &Event) -> Result<(), AppError> {
            let mut guard = self.broadcasts.lock().await;
            guard.push((topic.to_string(), event.id.clone()));
            Ok(())
        }

        async fn subscribe(
            &self,
            _topic: &str,
        ) -> Result<mpsc::Receiver<Event>, AppError> {
            let (_tx, rx) = mpsc::channel(1);
            Ok(rx)
        }

        async fn get_joined_topics(&self) -> Result<Vec<String>, AppError> {
            let guard = self.joined.lock().await;
            Ok(guard.iter().cloned().collect())
        }

        async fn get_topic_peers(&self, _topic: &str) -> Result<Vec<String>, AppError> {
            Ok(Vec::new())
        }

        async fn get_topic_stats(
            &self,
            topic: &str,
        ) -> Result<Option<crate::domain::p2p::TopicStats>, AppError> {
            let guard = self.joined.lock().await;
            if guard.contains(topic) {
                Ok(Some(crate::domain::p2p::TopicStats {
                    peer_count: 0,
                    message_count: 0,
                    last_activity: 0,
                }))
            } else {
                Ok(None)
            }
        }

        async fn broadcast_message(
            &self,
            topic: &str,
            _message: &[u8],
        ) -> Result<(), AppError> {
            let mut guard = self.broadcasts.lock().await;
            guard.push((topic.to_string(), "<raw>".into()));
            Ok(())
        }
    }

    fn create_test_event() -> Event {
        Event {
            id: "test_event_123".to_string(),
            pubkey: "test_pubkey".to_string(),
            created_at: chrono::Utc::now(),
            kind: EventKind::TextNote.into(),
            tags: vec![],
            content: "Test event content".to_string(),
            sig: "test_signature".to_string(),
        }
    }

    #[tokio::test]
    async fn test_default_distributor_creation() {
        let distributor = DefaultEventDistributor::new();
        let inner = distributor.inner.read().await;
        assert!(matches!(inner.strategy, DistributionStrategy::Hybrid));
    }

    #[tokio::test]
    async fn test_set_strategy() {
        let distributor = DefaultEventDistributor::new();
        distributor.set_strategy(DistributionStrategy::P2P).await;

        let inner = distributor.inner.read().await;
        assert!(matches!(inner.strategy, DistributionStrategy::P2P));
    }

    #[tokio::test]
    async fn test_distribute_event() {
        let distributor = DefaultEventDistributor::new();
        let gossip = DummyGossipService::new();
        distributor
            .set_gossip_service(Arc::new(gossip.clone()))
            .await;

        let event = create_test_event();

        let result = distributor
            .distribute(&event, DistributionStrategy::Broadcast)
            .await;
        assert!(result.is_ok());

        assert_eq!(gossip.broadcast_count().await, 1);
    }

    #[tokio::test]
    async fn test_hybrid_distribution() {
        let distributor = DefaultEventDistributor::new();
        let gossip = DummyGossipService::new();
        distributor
            .set_gossip_service(Arc::new(gossip.clone()))
            .await;

        let event = create_test_event();

        let result = distributor
            .distribute(&event, DistributionStrategy::Hybrid)
            .await;
        assert!(result.is_ok());
        assert_eq!(gossip.broadcast_count().await, 1);
    }

    #[tokio::test]
    async fn test_get_pending_events_empty() {
        let distributor = DefaultEventDistributor::new();
        let events = distributor.get_pending_events().await.unwrap();
        assert_eq!(events.len(), 0);
    }

    #[tokio::test]
    async fn test_p2p_distributor() {
        let distributor = P2PEventDistributor::new();
        let gossip = DummyGossipService::new();
        distributor
            .distributor
            .set_gossip_service(Arc::new(gossip.clone()))
            .await;

        let event = create_test_event();

        // P2P distributorは常にP2P戦略を使用
        let result = distributor
            .distribute(&event, DistributionStrategy::Broadcast)
            .await;
        assert!(result.is_ok());
        assert_eq!(gossip.broadcast_count().await, 1);
    }

    #[tokio::test]
    async fn test_nostr_distributor() {
        let distributor = NostrEventDistributor::new();
        let event = create_test_event();

        // Nostr distributorは常にNostr戦略を使用
        let result = distributor
            .distribute(&event, DistributionStrategy::P2P)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_retry_failed_with_no_failures() {
        let distributor = DefaultEventDistributor::new();
        let retry_count = distributor.retry_failed().await.unwrap();
        assert_eq!(retry_count, 0);
    }
}
