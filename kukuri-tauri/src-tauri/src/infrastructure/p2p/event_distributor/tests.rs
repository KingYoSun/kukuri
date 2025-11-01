use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{Mutex, mpsc};

use crate::domain::entities::{Event, EventKind};
use crate::domain::p2p::distribution::DistributionStrategy;
use crate::infrastructure::p2p::GossipService;
use crate::shared::error::AppError;

use super::EventDistributor;
use super::default::DefaultEventDistributor;
use super::strategy::{NostrEventDistributor, P2PEventDistributor};

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

    async fn subscribe(&self, _topic: &str) -> Result<mpsc::Receiver<Event>, AppError> {
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

    async fn broadcast_message(&self, topic: &str, _message: &[u8]) -> Result<(), AppError> {
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
    let strategy = distributor.current_strategy().await;
    assert!(matches!(strategy, DistributionStrategy::Hybrid));
}

#[tokio::test]
async fn test_set_strategy() {
    let distributor = DefaultEventDistributor::new();
    distributor.set_strategy(DistributionStrategy::P2P).await;

    let strategy = distributor.current_strategy().await;
    assert!(matches!(strategy, DistributionStrategy::P2P));
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
async fn test_nostr_distributor() {
    let distributor = NostrEventDistributor::new();
    let event = create_test_event();

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
