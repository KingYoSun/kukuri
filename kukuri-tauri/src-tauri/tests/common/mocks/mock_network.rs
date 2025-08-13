use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct MockNetworkService {
    is_connected: Arc<RwLock<bool>>,
    peers: Arc<RwLock<Vec<String>>>,
}

impl MockNetworkService {
    pub fn new() -> Self {
        Self {
            is_connected: Arc::new(RwLock::new(false)),
            peers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn connected() -> Self {
        Self {
            is_connected: Arc::new(RwLock::new(true)),
            peers: Arc::new(RwLock::new(vec!["peer1".to_string(), "peer2".to_string()])),
        }
    }

    pub async fn set_connected(&self, connected: bool) {
        *self.is_connected.write().await = connected;
    }

    pub async fn add_peer(&self, peer: String) {
        self.peers.write().await.push(peer);
    }

    pub async fn is_connected(&self) -> bool {
        *self.is_connected.read().await
    }

    pub async fn get_peers(&self) -> Vec<String> {
        self.peers.read().await.clone()
    }
}

#[derive(Debug, Clone)]
pub struct MockGossipService {
    topics: Arc<RwLock<Vec<String>>>,
    messages: Arc<RwLock<Vec<(String, String)>>>, // (topic, message)
}

impl MockGossipService {
    pub fn new() -> Self {
        Self {
            topics: Arc::new(RwLock::new(Vec::new())),
            messages: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn join_topic(&self, topic: String) {
        self.topics.write().await.push(topic);
    }

    pub async fn leave_topic(&self, topic: &str) {
        self.topics.write().await.retain(|t| t != topic);
    }

    pub async fn broadcast(&self, topic: String, message: String) {
        self.messages.write().await.push((topic, message));
    }

    pub async fn get_joined_topics(&self) -> Vec<String> {
        self.topics.read().await.clone()
    }

    pub async fn get_messages(&self) -> Vec<(String, String)> {
        self.messages.read().await.clone()
    }
}

#[derive(Debug, Clone)]
pub struct MockEventDistributor {
    distributed_events: Arc<RwLock<Vec<String>>>,
    pending_events: Arc<RwLock<Vec<String>>>,
}

impl MockEventDistributor {
    pub fn new() -> Self {
        Self {
            distributed_events: Arc::new(RwLock::new(Vec::new())),
            pending_events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn with_pending(events: Vec<String>) -> Self {
        Self {
            distributed_events: Arc::new(RwLock::new(Vec::new())),
            pending_events: Arc::new(RwLock::new(events)),
        }
    }

    pub async fn distribute(&self, event_id: String) {
        self.distributed_events.write().await.push(event_id.clone());
        self.pending_events.write().await.retain(|e| e != &event_id);
    }

    pub async fn get_distributed_events(&self) -> Vec<String> {
        self.distributed_events.read().await.clone()
    }

    pub async fn get_pending_events(&self) -> Vec<String> {
        self.pending_events.read().await.clone()
    }
}