use crate::domain::p2p::events::P2PEvent;
use crate::infrastructure::p2p::{
    DiscoveryOptions, GossipService, NetworkService, iroh_gossip_service::IrohGossipService,
    iroh_network_service::IrohNetworkService, metrics,
};
use crate::shared::config::NetworkConfig as AppNetworkConfig;
use crate::shared::error::AppError;
use async_trait::async_trait;
use iroh::SecretKey;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};

/// P2P繝阪ャ繝医Ρ繝ｼ繧ｯ縺ｮ繧ｹ繝・・繧ｿ繧ｹ諠・ｱ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PStatus {
    pub connected: bool,
    pub endpoint_id: String,
    pub active_topics: Vec<TopicInfo>,
    pub peer_count: usize,
    pub metrics_summary: GossipMetricsSummary,
}

/// Gossip繝｡繝医Μ繧ｯ繧ｹ縺ｮ繧ｵ繝槭Μ繝ｼ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipMetricsSummary {
    pub joins: u64,
    pub leaves: u64,
    pub broadcasts_sent: u64,
    pub messages_received: u64,
}

/// 繝医ヴ繝・け諠・ｱ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicInfo {
    pub id: String,
    pub peer_count: usize,
    pub message_count: usize,
    pub last_activity: i64,
}

/// P2P繧ｵ繝ｼ繝薙せ縺ｮ繝医Ξ繧､繝・
#[async_trait]
pub trait P2PServiceTrait: Send + Sync {
    /// P2P繝阪ャ繝医Ρ繝ｼ繧ｯ繧貞・譛溷喧
    async fn initialize(&self) -> Result<(), AppError>;

    /// 繝医ヴ繝・け縺ｫ蜿ょ刈
    async fn join_topic(&self, topic_id: &str, initial_peers: Vec<String>) -> Result<(), AppError>;

    /// 繝医ヴ繝・け縺九ｉ髮｢閼ｱ
    async fn leave_topic(&self, topic_id: &str) -> Result<(), AppError>;

    /// 繝｡繝・そ繝ｼ繧ｸ繧偵ヶ繝ｭ繝ｼ繝峨く繝｣繧ｹ繝・
    async fn broadcast_message(&self, topic_id: &str, content: &str) -> Result<(), AppError>;

    /// P2P繧ｹ繝・・繧ｿ繧ｹ繧貞叙蠕・
    async fn get_status(&self) -> Result<P2PStatus, AppError>;

    /// 繝弱・繝峨い繝峨Ξ繧ｹ繧貞叙蠕・
    async fn get_node_addresses(&self) -> Result<Vec<String>, AppError>;

    /// 繝医ヴ繝・けID繧堤函謌・
    fn generate_topic_id(&self, topic_name: &str) -> String;
}

/// P2P繧ｵ繝ｼ繝薙せ縺ｮ螳溯｣・
pub struct P2PService {
    network_service: Arc<dyn NetworkService>,
    gossip_service: Arc<dyn GossipService>,
    discovery_options: Arc<RwLock<DiscoveryOptions>>,
}

impl P2PService {
    pub fn new(
        network_service: Arc<dyn NetworkService>,
        gossip_service: Arc<dyn GossipService>,
    ) -> Self {
        Self::with_discovery(network_service, gossip_service, DiscoveryOptions::default())
    }

    pub fn with_discovery(
        network_service: Arc<dyn NetworkService>,
        gossip_service: Arc<dyn GossipService>,
        discovery: DiscoveryOptions,
    ) -> Self {
        Self {
            network_service,
            gossip_service,
            discovery_options: Arc::new(RwLock::new(discovery)),
        }
    }

    pub async fn discovery_options(&self) -> DiscoveryOptions {
        *self.discovery_options.read().await
    }

    pub async fn set_mainline_enabled(&self, enabled: bool) {
        let mut options = self.discovery_options.write().await;
        *options = options.with_mainline(enabled);
    }

    async fn mainline_enabled(&self) -> bool {
        self.discovery_options.read().await.enable_mainline()
    }

    async fn ensure_topic_joined(&self, topic_id: &str) -> Result<(), AppError> {
        let joined_topics = self
            .gossip_service
            .get_joined_topics()
            .await
            .map_err(|e| AppError::P2PError(e.to_string()))?;

        if !joined_topics.iter().any(|topic| topic == topic_id) {
            self.gossip_service
                .join_topic(topic_id, Vec::new())
                .await
                .map_err(|e| AppError::P2PError(e.to_string()))?;
        }

        if self.mainline_enabled().await {
            self.network_service.join_dht_topic(topic_id).await?;
        }

        Ok(())
    }

    pub fn builder(secret_key: SecretKey, network_config: AppNetworkConfig) -> P2PServiceBuilder {
        let discovery_options = DiscoveryOptions::from(&network_config);
        P2PServiceBuilder::new(secret_key, network_config, discovery_options)
    }
}

/// P2P繝ｬ繧､繝､繝ｼ縺ｮ讒狗ｯ臥ｵ先棡
pub struct P2PStack {
    pub network_service: Arc<dyn NetworkService>,
    pub gossip_service: Arc<dyn GossipService>,
    pub p2p_service: Arc<P2PService>,
}

pub struct P2PServiceBuilder {
    secret_key: SecretKey,
    network_config: AppNetworkConfig,
    discovery_options: DiscoveryOptions,
    event_sender: Option<broadcast::Sender<P2PEvent>>,
}

impl P2PServiceBuilder {
    fn new(
        secret_key: SecretKey,
        network_config: AppNetworkConfig,
        discovery_options: DiscoveryOptions,
    ) -> Self {
        Self {
            secret_key,
            network_config,
            discovery_options,
            event_sender: None,
        }
    }

    pub fn with_discovery_options(mut self, options: DiscoveryOptions) -> Self {
        self.discovery_options = options;
        self
    }

    pub fn enable_mainline(mut self, enabled: bool) -> Self {
        self.discovery_options = self.discovery_options.with_mainline(enabled);
        self
    }

    pub fn with_event_sender(mut self, sender: broadcast::Sender<P2PEvent>) -> Self {
        self.event_sender = Some(sender);
        self
    }

    pub fn discovery_options(&self) -> DiscoveryOptions {
        self.discovery_options
    }

    pub async fn build(self) -> Result<P2PStack, AppError> {
        let P2PServiceBuilder {
            secret_key,
            network_config,
            discovery_options,
            event_sender,
        } = self;

        let (network_event_sender, gossip_event_sender) = match event_sender {
            Some(sender) => (Some(sender.clone()), Some(sender)),
            None => (None, None),
        };

        let iroh_network = Arc::new(
            IrohNetworkService::new(
                secret_key,
                network_config,
                discovery_options,
                network_event_sender,
            )
            .await?,
        );
        let endpoint_arc = iroh_network.endpoint().clone();
        let mut gossip_inner = IrohGossipService::new(endpoint_arc)?;
        if let Some(tx) = gossip_event_sender {
            gossip_inner.set_event_sender(tx);
        }
        let iroh_gossip = Arc::new(gossip_inner);

        let network_service_dyn: Arc<dyn NetworkService> = iroh_network.clone();
        let gossip_service_dyn: Arc<dyn GossipService> = iroh_gossip.clone();
        let p2p_service = Arc::new(P2PService::with_discovery(
            Arc::clone(&network_service_dyn),
            Arc::clone(&gossip_service_dyn),
            discovery_options,
        ));

        Ok(P2PStack {
            network_service: network_service_dyn,
            gossip_service: gossip_service_dyn,
            p2p_service,
        })
    }
}

#[async_trait]
impl P2PServiceTrait for P2PService {
    async fn initialize(&self) -> Result<(), AppError> {
        // P2P繝阪ャ繝医Ρ繝ｼ繧ｯ縺ｮ蛻晄悄蛹門・逅・
        // 譌｢縺ｫstate.rs縺ｮinitialize_p2p縺ｧ蛻晄悄蛹悶＆繧後※縺・ｋ蝣ｴ蜷医・繝√ぉ繝・け縺ｮ縺ｿ
        Ok(())
    }

    async fn join_topic(&self, topic_id: &str, initial_peers: Vec<String>) -> Result<(), AppError> {
        self.gossip_service
            .join_topic(topic_id, initial_peers)
            .await
            .map_err(|e| AppError::P2PError(e.to_string()))?;

        if self.mainline_enabled().await {
            self.network_service.join_dht_topic(topic_id).await?;
        }

        Ok(())
    }

    async fn leave_topic(&self, topic_id: &str) -> Result<(), AppError> {
        self.gossip_service
            .leave_topic(topic_id)
            .await
            .map_err(|e| AppError::P2PError(e.to_string()))?;

        if self.mainline_enabled().await {
            self.network_service.leave_dht_topic(topic_id).await?;
        }

        Ok(())
    }

    async fn broadcast_message(&self, topic_id: &str, content: &str) -> Result<(), AppError> {
        self.ensure_topic_joined(topic_id).await?;

        self.gossip_service
            .broadcast_message(topic_id, content.as_bytes())
            .await
            .map_err(|e| AppError::P2PError(e.to_string()))?;

        if self.mainline_enabled().await {
            self.network_service
                .broadcast_dht(topic_id, content.as_bytes().to_vec())
                .await?;
        }

        Ok(())
    }

    async fn get_status(&self) -> Result<P2PStatus, AppError> {
        // 繧ｹ繝・・繧ｿ繧ｹ諠・ｱ繧貞庶髮・
        let endpoint_id = self
            .network_service
            .get_node_id()
            .await
            .map_err(|e| AppError::P2PError(e.to_string()))?;

        // 螳滄圀縺ｮ繝医ヴ繝・け諠・ｱ繧貞叙蠕・
        let joined_topics = self
            .gossip_service
            .get_joined_topics()
            .await
            .map_err(|e| AppError::P2PError(e.to_string()))?;

        let mut active_topics = Vec::new();
        let mut total_peer_count = 0;

        for topic_id in joined_topics {
            let stats = self
                .gossip_service
                .get_topic_stats(&topic_id)
                .await
                .map_err(|e| AppError::P2PError(e.to_string()))?;

            let (peer_count, message_count, last_activity) = if let Some(stats) = stats {
                (stats.peer_count, stats.message_count, stats.last_activity)
            } else {
                let peers = self
                    .gossip_service
                    .get_topic_peers(&topic_id)
                    .await
                    .map_err(|e| AppError::P2PError(e.to_string()))?;
                (peers.len(), 0, chrono::Utc::now().timestamp())
            };

            total_peer_count += peer_count;

            active_topics.push(TopicInfo {
                id: topic_id,
                peer_count,
                message_count,
                last_activity,
            });
        }

        let metrics_snapshot = metrics::snapshot();
        let metrics_summary = GossipMetricsSummary {
            joins: metrics_snapshot.joins,
            leaves: metrics_snapshot.leaves,
            broadcasts_sent: metrics_snapshot.broadcasts_sent,
            messages_received: metrics_snapshot.messages_received,
        };

        Ok(P2PStatus {
            connected: true,
            endpoint_id,
            active_topics,
            peer_count: total_peer_count,
            metrics_summary,
        })
    }

    async fn get_node_addresses(&self) -> Result<Vec<String>, AppError> {
        self.network_service
            .get_addresses()
            .await
            .map_err(|e| AppError::P2PError(e.to_string()))
    }

    fn generate_topic_id(&self, topic_name: &str) -> String {
        // 繝医ヴ繝・け蜷阪°繧迂D繧堤函謌撰ｼ井ｾ具ｼ壹ワ繝・す繝･繧剃ｽｿ逕ｨ・・
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(topic_name.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::p2p::TopicStats;
    use crate::infrastructure::p2p::{GossipService, NetworkService, metrics};
    use async_trait::async_trait;
    use chrono::Utc;
    use mockall::{mock, predicate::*};
    use std::sync::{Arc, Mutex};

    // NetworkService縺ｮ繝｢繝・け - 謇句虚螳溯｣・
    pub struct MockNetworkServ {
        node_id: Mutex<Option<String>>,
        addresses: Mutex<Option<Vec<String>>>,
        join_dht: Mutex<Vec<String>>,
        leave_dht: Mutex<Vec<String>>,
        broadcast_dht: Mutex<Vec<(String, Vec<u8>)>>,
    }

    impl MockNetworkServ {
        pub fn new() -> Self {
            Self {
                node_id: Mutex::new(None),
                addresses: Mutex::new(None),
                join_dht: Mutex::new(Vec::new()),
                leave_dht: Mutex::new(Vec::new()),
                broadcast_dht: Mutex::new(Vec::new()),
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
            Ok(vec![])
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
            true
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
    }

    // GossipService縺ｮ繝｢繝・け
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
            .with(eq("test_topic"), eq(vec![]))
            .times(1)
            .returning(|_, _| Err(AppError::P2PError("Failed to join topic".to_string())));

        let service = P2PService::new(network.clone(), Arc::new(mock_gossip));

        let result = service.join_topic("test_topic", vec![]).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to join topic")
        );
        assert!(network.join_dht_calls().is_empty());
    }

    #[tokio::test]
    async fn test_leave_topic() {
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
        assert_eq!(status.active_topics.len(), 2);
        assert_eq!(status.peer_count, 8); // 5 + 3
        assert_eq!(status.metrics_summary.joins, 0);
        assert_eq!(status.metrics_summary.leaves, 0);
        assert_eq!(status.metrics_summary.broadcasts_sent, 0);
        assert_eq!(status.metrics_summary.messages_received, 0);
        assert_eq!(status.active_topics[0].message_count, 12);
        assert_eq!(status.active_topics[0].last_activity, 1_700_000_000);
        assert_eq!(status.active_topics[1].message_count, 4);
        assert_eq!(status.active_topics[1].last_activity, 1_700_000_100);
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

        // 蜷後§繝医ヴ繝・け蜷阪°繧牙酔縺露D縺檎函謌舌＆繧後ｋ
        assert_eq!(topic_id1, topic_id2);
        // 逡ｰ縺ｪ繧九ヨ繝斐ャ繧ｯ蜷阪°繧峨・逡ｰ縺ｪ繧紀D縺檎函謌舌＆繧後ｋ
        assert_ne!(topic_id1, topic_id3);
    }
}
