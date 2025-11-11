use super::core::{P2PService, P2PServiceTrait};
use crate::application::services::p2p_service::status::ConnectionStatus;
use crate::domain::p2p::TopicStats;
use crate::infrastructure::p2p::network_service::Peer;
use crate::infrastructure::p2p::{GossipService, NetworkService, metrics};
use crate::shared::{AppError, config::BootstrapSource};
use async_trait::async_trait;
use chrono::Utc;
use mockall::{mock, predicate::*};
use std::sync::{Arc, Mutex};

pub struct MockNetworkServ {
    node_id: Mutex<Option<String>>,
    addresses: Mutex<Option<Vec<String>>>,
    join_dht: Mutex<Vec<String>>,
    leave_dht: Mutex<Vec<String>>,
    broadcast_dht: Mutex<Vec<(String, Vec<u8>)>>,
    connected: Mutex<bool>,
    peers: Mutex<Vec<Peer>>,
    applied_bootstrap_nodes: Mutex<Vec<String>>,
    applied_bootstrap_source: Mutex<Option<BootstrapSource>>,
}

impl MockNetworkServ {
    pub fn new() -> Self {
        Self {
            node_id: Mutex::new(None),
            addresses: Mutex::new(None),
            join_dht: Mutex::new(Vec::new()),
            leave_dht: Mutex::new(Vec::new()),
            broadcast_dht: Mutex::new(Vec::new()),
            connected: Mutex::new(true),
            peers: Mutex::new(Vec::new()),
            applied_bootstrap_nodes: Mutex::new(Vec::new()),
            applied_bootstrap_source: Mutex::new(None),
        }
    }

    pub fn expect_get_node_id(&mut self) -> &mut Self {
        self
    }

    pub fn returning<F>(&mut self, f: F) -> &mut Self
    where
        F: FnOnce() -> Result<String, AppError> + 'static,
    {
        if let Ok(value) = f() {
            *self.node_id.lock().unwrap() = Some(value);
        }
        self
    }

    pub fn expect_get_addresses(&mut self) -> &mut Self {
        self
    }

    pub fn returning_addresses<F>(&mut self, f: F) -> &mut Self
    where
        F: FnOnce() -> Result<Vec<String>, AppError> + 'static,
    {
        if let Ok(value) = f() {
            *self.addresses.lock().unwrap() = Some(value);
        }
        self
    }

    pub fn join_dht_calls(&self) -> Vec<String> {
        self.join_dht.lock().unwrap().clone()
    }

    pub fn leave_dht_calls(&self) -> Vec<String> {
        self.leave_dht.lock().unwrap().clone()
    }

    pub fn broadcast_dht_calls(&self) -> Vec<(String, Vec<u8>)> {
        self.broadcast_dht.lock().unwrap().clone()
    }

    pub fn set_connected(&self, connected: bool) {
        *self.connected.lock().unwrap() = connected;
    }

    pub fn set_peers(&self, peers: Vec<Peer>) {
        *self.peers.lock().unwrap() = peers;
    }

    pub fn applied_bootstrap_nodes(&self) -> Vec<String> {
        self.applied_bootstrap_nodes.lock().unwrap().clone()
    }

    pub fn applied_bootstrap_source(&self) -> Option<BootstrapSource> {
        *self.applied_bootstrap_source.lock().unwrap()
    }
}

#[async_trait]
impl NetworkService for MockNetworkServ {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn connect(&self) -> Result<(), AppError> {
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), AppError> {
        Ok(())
    }

    async fn get_peers(
        &self,
    ) -> Result<Vec<crate::infrastructure::p2p::network_service::Peer>, AppError> {
        Ok(self.peers.lock().unwrap().clone())
    }

    async fn add_peer(&self, _address: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn remove_peer(&self, _peer_id: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn get_stats(
        &self,
    ) -> Result<crate::infrastructure::p2p::network_service::NetworkStats, AppError> {
        Ok(crate::infrastructure::p2p::network_service::NetworkStats {
            connected_peers: 0,
            total_messages_sent: 0,
            total_messages_received: 0,
            bandwidth_up: 0,
            bandwidth_down: 0,
        })
    }

    async fn is_connected(&self) -> bool {
        *self.connected.lock().unwrap()
    }

    async fn get_node_id(&self) -> Result<String, AppError> {
        let node_id = self.node_id.lock().unwrap();
        Ok(node_id
            .clone()
            .unwrap_or_else(|| "default_node_id".to_string()))
    }

    async fn get_addresses(&self) -> Result<Vec<String>, AppError> {
        let addresses = self.addresses.lock().unwrap();
        Ok(addresses.clone().unwrap_or_else(std::vec::Vec::new))
    }

    async fn join_dht_topic(&self, topic: &str) -> Result<(), AppError> {
        self.join_dht.lock().unwrap().push(topic.to_string());
        Ok(())
    }

    async fn leave_dht_topic(&self, topic: &str) -> Result<(), AppError> {
        self.leave_dht.lock().unwrap().push(topic.to_string());
        Ok(())
    }

    async fn broadcast_dht(&self, topic: &str, message: Vec<u8>) -> Result<(), AppError> {
        self.broadcast_dht
            .lock()
            .unwrap()
            .push((topic.to_string(), message));
        Ok(())
    }

    async fn apply_bootstrap_nodes(
        &self,
        nodes: Vec<String>,
        source: BootstrapSource,
    ) -> Result<(), AppError> {
        *self.applied_bootstrap_nodes.lock().unwrap() = nodes;
        *self.applied_bootstrap_source.lock().unwrap() = Some(source);
        Ok(())
    }
}

mock! {
    pub GossipServ {}

    #[async_trait]
    impl GossipService for GossipServ {
        fn local_peer_hint(&self) -> Option<String>;
        async fn join_topic(&self, topic: &str, initial_peers: Vec<String>) -> Result<(), AppError>;
        async fn leave_topic(&self, topic: &str) -> Result<(), AppError>;
        async fn broadcast(&self, topic: &str, event: &crate::domain::entities::Event) -> Result<(), AppError>;
        async fn subscribe(&self, topic: &str) -> Result<tokio::sync::mpsc::Receiver<crate::domain::entities::Event>, AppError>;
        async fn get_joined_topics(&self) -> Result<Vec<String>, AppError>;
        async fn get_topic_peers(&self, topic: &str) -> Result<Vec<String>, AppError>;
        async fn get_topic_stats(&self, topic: &str) -> Result<Option<TopicStats>, AppError>;
        async fn broadcast_message(&self, topic: &str, message: &[u8]) -> Result<(), AppError>;
    }
}

#[tokio::test]
async fn test_initialize() {
    let network = Arc::new(MockNetworkServ::new());
    let gossip = Arc::new(MockGossipServ::new());

    let service = P2PService::new(network, gossip);

    let result = service.initialize().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_apply_bootstrap_nodes_forwards_to_network() {
    let network = Arc::new(MockNetworkServ::new());
    let gossip = Arc::new(MockGossipServ::new());
    let service = P2PService::new(
        Arc::clone(&network) as Arc<dyn NetworkService>,
        gossip as Arc<dyn GossipService>,
    );

    let nodes = vec![
        "node1@127.0.0.1:44001".to_string(),
        "node2@127.0.0.1:44002".to_string(),
    ];

    service
        .apply_bootstrap_nodes(nodes.clone(), BootstrapSource::User)
        .await
        .expect("apply bootstrap nodes");

    assert_eq!(network.applied_bootstrap_nodes(), nodes);
    assert_eq!(
        network.applied_bootstrap_source(),
        Some(BootstrapSource::User)
    );
}

#[tokio::test]
async fn test_join_topic_success() {
    let network = Arc::new(MockNetworkServ::new());
    let mut mock_gossip = MockGossipServ::new();

    mock_gossip
        .expect_join_topic()
        .with(
            eq("test_topic"),
            eq(vec!["peer1".to_string(), "peer2".to_string()]),
        )
        .times(1)
        .returning(|_, _| Ok(()));

    let service = P2PService::new(network.clone(), Arc::new(mock_gossip));

    let result = service
        .join_topic("test_topic", vec!["peer1".to_string(), "peer2".to_string()])
        .await;
    assert!(result.is_ok());
    assert_eq!(network.join_dht_calls(), vec!["test_topic".to_string()]);
}

#[tokio::test]
async fn test_join_topic_failure() {
    let network = Arc::new(MockNetworkServ::new());
    let mut mock_gossip = MockGossipServ::new();

    mock_gossip
        .expect_join_topic()
        .with(eq("fail_topic"), eq(Vec::<String>::new()))
        .times(1)
        .returning(|_, _| Err(AppError::P2PError("join failed".into())));

    let service = P2PService::new(network.clone(), Arc::new(mock_gossip));

    let result = service.join_topic("fail_topic", Vec::new()).await;
    assert!(result.is_err());
    assert!(network.join_dht_calls().is_empty());
}

#[tokio::test]
async fn test_leave_topic_success() {
    let network = Arc::new(MockNetworkServ::new());
    let mut mock_gossip = MockGossipServ::new();

    mock_gossip
        .expect_leave_topic()
        .with(eq("test_topic"))
        .times(1)
        .returning(|_| Ok(()));

    let service = P2PService::new(network.clone(), Arc::new(mock_gossip));

    let result = service.leave_topic("test_topic").await;
    assert!(result.is_ok());
    assert_eq!(network.leave_dht_calls(), vec!["test_topic".to_string()]);
}

#[tokio::test]
async fn test_leave_topic_failure() {
    let network = Arc::new(MockNetworkServ::new());
    let mut mock_gossip = MockGossipServ::new();

    mock_gossip
        .expect_leave_topic()
        .with(eq("fail_topic"))
        .times(1)
        .returning(|_| Err(AppError::P2PError("leave failed".into())));

    let service = P2PService::new(network.clone(), Arc::new(mock_gossip));

    let result = service.leave_topic("fail_topic").await;
    assert!(result.is_err());
    assert!(network.leave_dht_calls().is_empty());
}

#[tokio::test]
async fn test_broadcast_message() {
    let network = Arc::new(MockNetworkServ::new());
    let mut mock_gossip = MockGossipServ::new();

    mock_gossip
        .expect_get_joined_topics()
        .times(1)
        .returning(|| Ok(vec!["test_topic".to_string()]));

    let test_content = "Test message";
    mock_gossip
        .expect_broadcast_message()
        .with(eq("test_topic"), eq(test_content.as_bytes()))
        .times(1)
        .returning(|_, _| Ok(()));

    let service = P2PService::new(network.clone(), Arc::new(mock_gossip));

    let result = service.broadcast_message("test_topic", test_content).await;
    assert!(result.is_ok());
    assert_eq!(network.join_dht_calls(), vec!["test_topic".to_string()]);
    let broadcast_calls = network.broadcast_dht_calls();
    assert_eq!(broadcast_calls.len(), 1);
    assert_eq!(broadcast_calls[0].0, "test_topic".to_string());
    assert_eq!(String::from_utf8_lossy(&broadcast_calls[0].1), test_content);
}

#[tokio::test]
async fn test_broadcast_message_auto_join_when_not_joined() {
    let network = Arc::new(MockNetworkServ::new());
    let mut mock_gossip = MockGossipServ::new();

    mock_gossip
        .expect_get_joined_topics()
        .times(1)
        .returning(|| Ok(vec![]));

    mock_gossip
        .expect_join_topic()
        .with(eq("auto_topic"), eq(Vec::<String>::new()))
        .times(1)
        .returning(|_, _| Ok(()));

    mock_gossip
        .expect_broadcast_message()
        .with(eq("auto_topic"), eq("auto payload".as_bytes()))
        .times(1)
        .returning(|_, _| Ok(()));

    let service = P2PService::new(network.clone(), Arc::new(mock_gossip));
    let result = service
        .broadcast_message("auto_topic", "auto payload")
        .await;
    assert!(result.is_ok());

    assert_eq!(network.join_dht_calls(), vec!["auto_topic".to_string()]);
    let broadcast_calls = network.broadcast_dht_calls();
    assert_eq!(broadcast_calls.len(), 1);
    assert_eq!(broadcast_calls[0].0, "auto_topic".to_string());
    assert_eq!(
        String::from_utf8_lossy(&broadcast_calls[0].1),
        "auto payload"
    );
}

#[tokio::test]
async fn test_get_status() {
    metrics::reset_all();
    let mut mock_network = MockNetworkServ::new();
    mock_network
        .expect_get_node_id()
        .returning(|| Ok("node123".to_string()));
    let network = Arc::new(mock_network);
    network.set_connected(false);
    network.set_peers(Vec::new());
    assert!(!network.is_connected().await);

    network.set_connected(true);
    let now = Utc::now().timestamp();
    network.set_peers(vec![
        Peer {
            id: "peer-1".to_string(),
            address: "/ip4/127.0.0.1/tcp/4001".to_string(),
            connected_at: now,
            last_seen: now,
        },
        Peer {
            id: "peer-2".to_string(),
            address: "/ip4/127.0.0.1/tcp/4002".to_string(),
            connected_at: now,
            last_seen: now,
        },
    ]);

    let mut mock_gossip = MockGossipServ::new();
    mock_gossip
        .expect_get_joined_topics()
        .times(1)
        .returning(|| Ok(vec!["topic1".to_string(), "topic2".to_string()]));

    mock_gossip
        .expect_get_topic_stats()
        .with(eq("topic1"))
        .times(1)
        .returning(|_| {
            Ok(Some(TopicStats {
                peer_count: 5,
                message_count: 12,
                last_activity: 1_700_000_000,
            }))
        });

    mock_gossip
        .expect_get_topic_stats()
        .with(eq("topic2"))
        .times(1)
        .returning(|_| {
            Ok(Some(TopicStats {
                peer_count: 3,
                message_count: 4,
                last_activity: 1_700_000_100,
            }))
        });

    let service = P2PService::new(network, Arc::new(mock_gossip));

    let result = service.get_status().await;
    assert!(result.is_ok());

    let status = result.unwrap();
    assert_eq!(status.endpoint_id, "node123");
    assert!(status.connected);
    assert_eq!(status.connection_status, ConnectionStatus::Connected);
    assert_eq!(status.active_topics.len(), 2);
    assert_eq!(status.peer_count, 8);
    assert_eq!(status.metrics_summary.joins, 0);
    assert_eq!(status.metrics_summary.leaves, 0);
    assert_eq!(status.metrics_summary.broadcasts_sent, 0);
    assert_eq!(status.metrics_summary.messages_received, 0);
    assert_eq!(status.active_topics[0].message_count, 12);
    assert_eq!(status.active_topics[0].last_activity, 1_700_000_000);
    assert_eq!(status.active_topics[1].message_count, 4);
    assert_eq!(status.active_topics[1].last_activity, 1_700_000_100);
    assert_eq!(status.peers.len(), 2);
    assert_eq!(status.peers[0].node_id, "peer-1");
}

#[tokio::test]
async fn test_get_status_fallback_to_peers_when_stats_missing() {
    metrics::reset_all();
    let mut mock_network = MockNetworkServ::new();
    mock_network
        .expect_get_node_id()
        .returning(|| Ok("node123".to_string()));
    let network = Arc::new(mock_network);

    let mut mock_gossip = MockGossipServ::new();
    mock_gossip
        .expect_get_joined_topics()
        .times(1)
        .returning(|| Ok(vec!["topic1".to_string()]));

    mock_gossip
        .expect_get_topic_stats()
        .with(eq("topic1"))
        .times(1)
        .returning(|_| Ok(None));

    mock_gossip
        .expect_get_topic_peers()
        .with(eq("topic1"))
        .times(1)
        .returning(|_| Ok(vec!["peer1".to_string(), "peer2".to_string()]));

    let service = P2PService::new(network, Arc::new(mock_gossip));

    let before = Utc::now().timestamp();
    let status = service.get_status().await.unwrap();
    let after = Utc::now().timestamp();

    assert_eq!(status.active_topics.len(), 1);
    let topic = &status.active_topics[0];
    assert_eq!(topic.peer_count, 2);
    assert_eq!(topic.message_count, 0);
    assert!(topic.last_activity >= before);
    assert!(topic.last_activity <= after);
    assert_eq!(status.connection_status, ConnectionStatus::Disconnected);
}

#[tokio::test]
async fn test_get_node_addresses() {
    let mut mock_network = MockNetworkServ::new();
    mock_network.expect_get_addresses().returning_addresses(|| {
        Ok(vec![
            "/ip4/127.0.0.1/tcp/4001".to_string(),
            "/ip4/192.168.1.10/tcp/4001".to_string(),
        ])
    });

    let mock_gossip = MockGossipServ::new();

    let service = P2PService::new(Arc::new(mock_network), Arc::new(mock_gossip));

    let result = service.get_node_addresses().await;
    assert!(result.is_ok());

    let addresses = result.unwrap();
    assert_eq!(addresses.len(), 2);
    assert!(addresses.contains(&"/ip4/127.0.0.1/tcp/4001".to_string()));
}

#[tokio::test]
async fn test_generate_topic_id() {
    let mock_network = MockNetworkServ::new();
    let mock_gossip = MockGossipServ::new();

    let service = P2PService::new(Arc::new(mock_network), Arc::new(mock_gossip));

    let topic_id1 = service.generate_topic_id("test_topic");
    let topic_id2 = service.generate_topic_id("test_topic");
    let topic_id3 = service.generate_topic_id("different_topic");

    assert_eq!(topic_id1, topic_id2);
    assert_ne!(topic_id1, topic_id3);
}
