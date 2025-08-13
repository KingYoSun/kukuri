use crate::domain::entities::Event;
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

#[derive(Debug, Clone)]
pub enum DistributionStrategy {
    Broadcast,       // 全ピアに配信
    Gossip,         // Gossipプロトコルで配信
    Direct(String), // 特定のピアに直接配信
    Hybrid,         // NostrとP2Pの両方で配信
    Nostr,          // Nostrリレー経由のみ
    P2P,            // P2Pネットワークのみ
}

#[async_trait]
pub trait EventDistributor: Send + Sync {
    async fn distribute(&self, event: &Event, strategy: DistributionStrategy) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn receive(&self) -> Result<Option<Event>, Box<dyn std::error::Error + Send + Sync>>;
    async fn set_strategy(&self, strategy: DistributionStrategy);
    async fn get_pending_events(&self) -> Result<Vec<Event>, Box<dyn std::error::Error + Send + Sync>>;
    async fn retry_failed(&self) -> Result<u32, Box<dyn std::error::Error + Send + Sync>>;
}

/// デフォルトのEventDistributor実装
pub struct DefaultEventDistributor {
    inner: Arc<RwLock<EventDistributorInner>>,
}

struct EventDistributorInner {
    strategy: DistributionStrategy,
    pending_events: VecDeque<Event>,
    failed_events: Vec<(Event, DistributionStrategy)>,
    max_retries: u32,
}

impl DefaultEventDistributor {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(EventDistributorInner {
                strategy: DistributionStrategy::Hybrid,
                pending_events: VecDeque::new(),
                failed_events: Vec::new(),
                max_retries: 3,
            })),
        }
    }

    pub fn with_strategy(strategy: DistributionStrategy) -> Self {
        Self {
            inner: Arc::new(RwLock::new(EventDistributorInner {
                strategy,
                pending_events: VecDeque::new(),
                failed_events: Vec::new(),
                max_retries: 3,
            })),
        }
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
                // TODO: 実際のブロードキャスト実装
                // 現在はモック実装
                info!("Event {} broadcasted successfully", event.id);
                Ok(())
            }
            DistributionStrategy::Gossip => {
                debug!("Distributing event {} via Gossip protocol", event.id);
                // TODO: Gossipプロトコルでの配信実装
                // 現在はモック実装
                info!("Event {} distributed via Gossip", event.id);
                Ok(())
            }
            DistributionStrategy::Direct(peer_id) => {
                debug!("Sending event {} directly to peer {}", event.id, peer_id);
                // TODO: 特定ピアへの直接送信実装
                // 現在はモック実装
                info!("Event {} sent to peer {}", event.id, peer_id);
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
                // TODO: Nostrリレーへの配信実装
                // 現在はモック実装
                info!("Event {} sent to Nostr relays", event.id);
                Ok(())
            }
            DistributionStrategy::P2P => {
                debug!("Distributing event {} via P2P network", event.id);
                // TODO: P2Pネットワークへの配信実装
                // 現在はモック実装
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
        debug!("Distributing event {} with strategy {:?}", event.id, strategy);
        
        // 配信を試行
        match self.distribute_internal(event, &strategy).await {
            Ok(()) => {
                info!("Event {} distributed successfully", event.id);
                Ok(())
            }
            Err(e) => {
                error!("Failed to distribute event {}: {}", event.id, e);
                
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

    async fn get_pending_events(&self) -> Result<Vec<Event>, Box<dyn std::error::Error + Send + Sync>> {
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
                    retry_count += 1;
                }
                Err(e) => {
                    error!("Event {} failed again on retry: {}", event.id, e);
                    still_failed.push((event, strategy));
                }
            }
        }

        // まだ失敗しているイベントを戻す
        inner.failed_events = still_failed;

        Ok(retry_count)
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
        self.distributor.distribute(event, DistributionStrategy::P2P).await
    }

    async fn receive(&self) -> Result<Option<Event>, Box<dyn std::error::Error + Send + Sync>> {
        self.distributor.receive().await
    }

    async fn set_strategy(&self, _strategy: DistributionStrategy) {
        // P2P専用なので変更しない
    }

    async fn get_pending_events(&self) -> Result<Vec<Event>, Box<dyn std::error::Error + Send + Sync>> {
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
        self.distributor.distribute(event, DistributionStrategy::Nostr).await
    }

    async fn receive(&self) -> Result<Option<Event>, Box<dyn std::error::Error + Send + Sync>> {
        self.distributor.receive().await
    }

    async fn set_strategy(&self, _strategy: DistributionStrategy) {
        // Nostr専用なので変更しない
    }

    async fn get_pending_events(&self) -> Result<Vec<Event>, Box<dyn std::error::Error + Send + Sync>> {
        self.distributor.get_pending_events().await
    }

    async fn retry_failed(&self) -> Result<u32, Box<dyn std::error::Error + Send + Sync>> {
        self.distributor.retry_failed().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::EventKind;

    fn create_test_event() -> Event {
        Event {
            id: "test_event_123".to_string(),
            pubkey: "test_pubkey".to_string(),
            created_at: chrono::Utc::now(),
            kind: EventKind::TextNote,
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
        let event = create_test_event();
        
        let result = distributor.distribute(&event, DistributionStrategy::Broadcast).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_hybrid_distribution() {
        let distributor = DefaultEventDistributor::new();
        let event = create_test_event();
        
        let result = distributor.distribute(&event, DistributionStrategy::Hybrid).await;
        assert!(result.is_ok());
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
        let event = create_test_event();
        
        // P2P distributorは常にP2P戦略を使用
        let result = distributor.distribute(&event, DistributionStrategy::Broadcast).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_nostr_distributor() {
        let distributor = NostrEventDistributor::new();
        let event = create_test_event();
        
        // Nostr distributorは常にNostr戦略を使用
        let result = distributor.distribute(&event, DistributionStrategy::P2P).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_retry_failed_with_no_failures() {
        let distributor = DefaultEventDistributor::new();
        let retry_count = distributor.retry_failed().await.unwrap();
        assert_eq!(retry_count, 0);
    }
}