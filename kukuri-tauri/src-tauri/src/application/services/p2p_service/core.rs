use super::bootstrap::P2PServiceBuilder;
use super::metrics::GossipMetricsSummary;
use super::status::{ConnectionStatus, P2PStatus, PeerStatus, TopicInfo};
use crate::infrastructure::p2p::{DiscoveryOptions, GossipService, NetworkService, metrics};
use crate::shared::config::NetworkConfig as AppNetworkConfig;
use crate::shared::error::AppError;
use async_trait::async_trait;
use iroh::SecretKey;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct P2PService {
    network_service: Arc<dyn NetworkService>,
    gossip_service: Arc<dyn GossipService>,
    discovery_options: Arc<RwLock<DiscoveryOptions>>,
}

#[async_trait]
pub trait P2PServiceTrait: Send + Sync {
    async fn initialize(&self) -> Result<(), AppError>;
    async fn join_topic(&self, topic_id: &str, initial_peers: Vec<String>) -> Result<(), AppError>;
    async fn leave_topic(&self, topic_id: &str) -> Result<(), AppError>;
    async fn broadcast_message(&self, topic_id: &str, content: &str) -> Result<(), AppError>;
    async fn get_status(&self) -> Result<P2PStatus, AppError>;
    async fn get_node_addresses(&self) -> Result<Vec<String>, AppError>;
    fn generate_topic_id(&self, topic_name: &str) -> String;
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

#[async_trait]
impl P2PServiceTrait for P2PService {
    async fn initialize(&self) -> Result<(), AppError> {
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
        let endpoint_id = self
            .network_service
            .get_node_id()
            .await
            .map_err(|e| AppError::P2PError(e.to_string()))?;

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

        let peers = self
            .network_service
            .get_peers()
            .await
            .map_err(|e| AppError::P2PError(e.to_string()))?;

        let network_connected = self.network_service.is_connected().await && !peers.is_empty();

        let peer_status: Vec<PeerStatus> = peers
            .into_iter()
            .map(|peer| PeerStatus {
                node_id: peer.id,
                address: peer.address,
                connected_at: peer.connected_at,
                last_seen: peer.last_seen,
            })
            .collect();

        let metrics_summary = GossipMetricsSummary::from_snapshot(&metrics::snapshot());

        Ok(P2PStatus {
            connected: network_connected,
            connection_status: if network_connected {
                ConnectionStatus::Connected
            } else {
                ConnectionStatus::Disconnected
            },
            endpoint_id,
            active_topics,
            peer_count: total_peer_count,
            peers: peer_status,
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
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(topic_name.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}
