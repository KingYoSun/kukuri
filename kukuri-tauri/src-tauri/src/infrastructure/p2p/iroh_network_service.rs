use super::{
    DiscoveryOptions, NetworkService, NetworkStats, Peer,
    dht_bootstrap::{DhtGossip, secret},
    utils::parse_node_addr,
};
use crate::domain::p2p::{P2PEvent, generate_topic_id, topic_id_bytes};
use crate::shared::config::{BootstrapSource, NetworkConfig as AppNetworkConfig};
use crate::shared::error::AppError;
use async_trait::async_trait;
use iroh::{Endpoint, RelayMode, address_lookup::MemoryLookup, protocol::Router};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use tracing;

pub struct IrohNetworkService {
    endpoint: Arc<Endpoint>,
    router: Arc<Router>,
    static_discovery: Arc<MemoryLookup>,
    connected: Arc<RwLock<bool>>,
    peers: Arc<RwLock<Vec<Peer>>>,
    stats: Arc<RwLock<NetworkStats>>,
    dht_gossip: Option<Arc<DhtGossip>>,
    discovery_options: Arc<RwLock<DiscoveryOptions>>,
    network_config: Arc<RwLock<AppNetworkConfig>>,
    bootstrap_peers: Arc<RwLock<Vec<String>>>,
    bootstrap_source: Arc<RwLock<BootstrapSource>>,
    p2p_event_tx: Option<broadcast::Sender<P2PEvent>>,
}

impl IrohNetworkService {
    pub async fn new(
        secret_key: iroh::SecretKey,
        net_cfg: AppNetworkConfig,
        discovery_options: DiscoveryOptions,
        event_tx: Option<broadcast::Sender<P2PEvent>>,
    ) -> Result<Self, AppError> {
        // Endpointの作成（設定に応じてディスカバリーを有効化）
        let static_discovery = Arc::new(MemoryLookup::new());
        let builder = Endpoint::empty_builder(RelayMode::Default).secret_key(secret_key);
        let builder = discovery_options.apply_to_builder(builder);
        let builder = builder.address_lookup(static_discovery.clone());
        let endpoint = builder
            .bind()
            .await
            .map_err(|e| AppError::P2PError(format!("Failed to bind endpoint: {e:?}")))?;

        // Routerの作成（Gossipプロトコルは別で設定）
        let router = Router::builder(endpoint.clone()).spawn();

        // ブートストラップ設定の検証（警告/件数ログのみ）
        if let Err(e) = super::bootstrap_config::validate_bootstrap_config() {
            tracing::warn!("bootstrap_nodes.json validation failed: {:?}", e);
        }

        // DhtGossipの初期化
        let dht_gossip = match DhtGossip::new(Arc::new(endpoint.clone())).await {
            Ok(service) => Some(Arc::new(service)),
            Err(e) => {
                tracing::warn!("Failed to initialize DhtGossip: {:?}", e);
                None
            }
        };

        let network_config = Arc::new(RwLock::new(net_cfg.clone()));
        let endpoint = Arc::new(endpoint);
        let service = Self {
            endpoint: Arc::clone(&endpoint),
            router: Arc::new(router),
            static_discovery,
            connected: Arc::new(RwLock::new(false)),
            peers: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(NetworkStats {
                connected_peers: 0,
                total_messages_sent: 0,
                total_messages_received: 0,
                bandwidth_up: 0,
                bandwidth_down: 0,
            })),
            dht_gossip,
            discovery_options: Arc::new(RwLock::new(discovery_options)),
            network_config: Arc::clone(&network_config),
            bootstrap_peers: Arc::new(RwLock::new(net_cfg.bootstrap_peers.clone())),
            bootstrap_source: Arc::new(RwLock::new(net_cfg.bootstrap_source)),
            p2p_event_tx: event_tx,
        };

        service.apply_bootstrap_peers_from_config().await;

        Ok(service)
    }

    pub fn endpoint(&self) -> &Arc<Endpoint> {
        &self.endpoint
    }

    pub fn static_discovery(&self) -> Arc<MemoryLookup> {
        Arc::clone(&self.static_discovery)
    }

    pub fn router(&self) -> &Arc<Router> {
        &self.router
    }

    fn emit_event(&self, event: P2PEvent) {
        if let Some(tx) = &self.p2p_event_tx {
            let _ = tx.send(event);
        }
    }

    async fn apply_bootstrap_peers_from_config(&self) {
        let peers = { self.bootstrap_peers.read().await.clone() };
        if peers.is_empty() {
            return;
        }
        let source = *self.bootstrap_source.read().await;
        let success_count = self.connect_bootstrap_nodes(&peers).await;
        if success_count > 0 {
            super::metrics::record_bootstrap_source(source);
        }
    }

    async fn connect_bootstrap_nodes(&self, nodes: &[String]) -> usize {
        let mut success_count = 0usize;
        for peer in nodes {
            let trimmed = peer.trim();
            if trimmed.is_empty() {
                continue;
            }

            match self.add_peer(trimmed).await {
                Ok(_) => {
                    success_count += 1;
                    tracing::info!("Connected to bootstrap peer from config: {}", trimmed);
                }
                Err(err) => {
                    tracing::warn!("Failed to connect to bootstrap peer '{}': {}", trimmed, err);
                }
            }
        }
        success_count
    }

    fn bootstrap_node_id(candidate: &str) -> Option<String> {
        let trimmed = candidate.trim();
        let (node_id, _) = trimmed.split_once('@')?;
        let node_id = node_id.trim();
        if node_id.is_empty() {
            return None;
        }
        Some(node_id.to_string())
    }

    async fn prune_stale_bootstrap_peers(&self, stale_peer_ids: &HashSet<String>) -> usize {
        let mut peers = self.peers.write().await;
        let before = peers.len();

        peers.retain(|peer| {
            if peer.address.ends_with("@fallback") {
                return false;
            }
            !stale_peer_ids.contains(&peer.id)
        });

        let removed = before.saturating_sub(peers.len());
        if removed > 0 {
            let mut stats = self.stats.write().await;
            stats.connected_peers = peers.len();
            super::metrics::set_mainline_connected_peers(stats.connected_peers as u64);
        }

        removed
    }

    pub fn node_id(&self) -> String {
        self.endpoint.id().to_string()
    }

    pub async fn discovery_options(&self) -> DiscoveryOptions {
        *self.discovery_options.read().await
    }

    pub async fn node_addr(&self) -> Result<Vec<String>, AppError> {
        // 直接アドレスを解決し、`node_id@ip:port` 形式で返却
        self.endpoint.online().await;
        let node_addr = self.endpoint.addr();
        let node_id = node_addr.id.to_string();
        let mut out = Vec::new();
        for addr in node_addr.ip_addrs() {
            out.push(format!("{node_id}@{addr}"));
        }
        if out.is_empty() {
            out.push(node_id);
        }
        Ok(out)
    }

    /// DHT????????????
    pub async fn join_dht_topic(&self, topic_name: &str) -> Result<(), AppError> {
        let canonical = generate_topic_id(topic_name);
        let topic_bytes = topic_id_bytes(&canonical);
        if let Some(ref dht_gossip) = self.dht_gossip {
            dht_gossip.join_topic(&topic_bytes, vec![]).await?;
            tracing::info!(
                "Joined DHT topic: {} (requested: {})",
                canonical,
                topic_name
            );
        } else {
            tracing::warn!("DHT service not available, using fallback");
            // ?????????????
            self.connect_fallback().await?;
        }
        Ok(())
    }

    /// DHT?????????????
    pub async fn leave_dht_topic(&self, topic_name: &str) -> Result<(), AppError> {
        let canonical = generate_topic_id(topic_name);
        let topic_bytes = topic_id_bytes(&canonical);
        if let Some(ref dht_gossip) = self.dht_gossip {
            dht_gossip.leave_topic(&topic_bytes).await?;
            tracing::info!("Left DHT topic: {} (requested: {})", canonical, topic_name);
        }
        Ok(())
    }
    /// DHT???????????????????
    pub async fn broadcast_dht(&self, topic_name: &str, message: Vec<u8>) -> Result<(), AppError> {
        let canonical = generate_topic_id(topic_name);
        let topic_bytes = topic_id_bytes(&canonical);
        if let Some(ref dht_gossip) = self.dht_gossip {
            dht_gossip.broadcast(&topic_bytes, message).await?;
        } else {
            return Err(AppError::P2PError("DHT service not available".to_string()));
        }
        Ok(())
    }
    /// フォールバックモードでピアに接続
    async fn connect_fallback(&self) -> Result<(), AppError> {
        // 1) 設定ファイルからのブートストラップ接続を優先
        let fallback_peers =
            match super::dht_bootstrap::fallback::connect_from_config(&self.endpoint).await {
                Ok(peers) => peers,
                Err(_) => {
                    // 2) ハードコードされたフォールバックに接続（なければ失敗）
                    match super::dht_bootstrap::fallback::connect_to_fallback(&self.endpoint).await
                    {
                        Ok(peers) => peers,
                        Err(err) => {
                            super::metrics::record_mainline_reconnect_failure();
                            return Err(err);
                        }
                    }
                }
            };

        super::metrics::record_mainline_reconnect_success();

        // フォールバックピアをピアリストに追加
        let mut peers = self.peers.write().await;
        let now = chrono::Utc::now().timestamp();

        for node_addr in fallback_peers {
            peers.push(Peer {
                id: node_addr.id.to_string(),
                address: format!("{}@fallback", node_addr.id),
                connected_at: now,
                last_seen: now,
            });
            self.static_discovery.add_endpoint_info(node_addr);
        }

        // 統計を更新
        let mut stats = self.stats.write().await;
        stats.connected_peers = peers.len();
        super::metrics::set_mainline_connected_peers(stats.connected_peers as u64);

        Ok(())
    }

    /// 共有シークレットをローテーション
    pub async fn rotate_dht_secret(&self) -> Result<(), AppError> {
        secret::rotate_secret()
            .await
            .map_err(|e| AppError::P2PError(format!("Failed to rotate secret: {e:?}")))?;
        tracing::info!("DHT shared secret rotated");
        Ok(())
    }
}

#[async_trait]
impl NetworkService for IrohNetworkService {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn connect(&self) -> Result<(), AppError> {
        let mut connected = self.connected.write().await;
        let was_connected = *connected;
        *connected = true;
        drop(connected);
        if !was_connected {
            let node_id = self.endpoint.id().to_string();
            let addresses = match self.node_addr().await {
                Ok(addresses) => addresses,
                Err(err) => {
                    tracing::warn!("Failed to resolve node addresses on connect: {}", err);
                    Vec::new()
                }
            };
            self.emit_event(P2PEvent::NetworkConnected { node_id, addresses });
        }
        tracing::info!("Network service connected");
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), AppError> {
        let mut connected = self.connected.write().await;
        let was_connected = *connected;
        *connected = false;
        drop(connected);

        // ピアリストをクリア
        let mut peers = self.peers.write().await;
        peers.clear();
        super::metrics::set_mainline_connected_peers(0);

        tracing::info!("Network service disconnected");
        if was_connected {
            let node_id = self.endpoint.id().to_string();
            self.emit_event(P2PEvent::NetworkDisconnected { node_id });
        }
        Ok(())
    }

    async fn get_peers(&self) -> Result<Vec<Peer>, AppError> {
        let peers = self.peers.read().await;
        Ok(peers.clone())
    }

    async fn add_peer(&self, address: &str) -> Result<(), AppError> {
        let node_addr = parse_node_addr(address).map_err(|e| {
            super::metrics::record_mainline_connection_failure();
            AppError::from(format!("Failed to parse peer address: {e}"))
        })?;
        let node_id = node_addr.id.to_string();

        self.static_discovery.add_endpoint_info(node_addr.clone());

        // ピアに接続
        self.endpoint
            .connect(node_addr.clone(), iroh_gossip::ALPN)
            .await
            .map_err(|e| {
                super::metrics::record_mainline_connection_failure();
                AppError::from(format!("Failed to connect to peer: {e}"))
            })?;

        // ピアリストに追加
        let mut peers = self.peers.write().await;
        let now = chrono::Utc::now().timestamp();
        if let Some(existing) = peers.iter_mut().find(|peer| peer.id == node_id) {
            existing.address = address.to_string();
            existing.last_seen = now;
        } else {
            peers.push(Peer {
                id: node_id,
                address: address.to_string(),
                connected_at: now,
                last_seen: now,
            });
        }

        // 統計を更新
        let mut stats = self.stats.write().await;
        stats.connected_peers = peers.len();
        super::metrics::record_mainline_connection_success();
        super::metrics::set_mainline_connected_peers(stats.connected_peers as u64);

        tracing::info!("Added peer: {}", address);
        Ok(())
    }

    async fn remove_peer(&self, peer_id: &str) -> Result<(), AppError> {
        let mut peers = self.peers.write().await;
        peers.retain(|p| p.id != peer_id);

        // 統計を更新
        let mut stats = self.stats.write().await;
        stats.connected_peers = peers.len();
        super::metrics::set_mainline_connected_peers(stats.connected_peers as u64);

        tracing::info!("Removed peer: {}", peer_id);
        Ok(())
    }

    async fn get_stats(&self) -> Result<NetworkStats, AppError> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    async fn is_connected(&self) -> bool {
        let connected = self.connected.read().await;
        *connected
    }

    async fn get_node_id(&self) -> Result<String, AppError> {
        Ok(self.endpoint.id().to_string())
    }

    async fn get_addresses(&self) -> Result<Vec<String>, AppError> {
        self.node_addr().await
    }

    async fn join_dht_topic(&self, topic: &str) -> Result<(), AppError> {
        IrohNetworkService::join_dht_topic(self, topic).await
    }

    async fn leave_dht_topic(&self, topic: &str) -> Result<(), AppError> {
        IrohNetworkService::leave_dht_topic(self, topic).await
    }

    async fn broadcast_dht(&self, topic: &str, message: Vec<u8>) -> Result<(), AppError> {
        IrohNetworkService::broadcast_dht(self, topic, message).await
    }

    async fn apply_bootstrap_nodes(
        &self,
        nodes: Vec<String>,
        source: BootstrapSource,
    ) -> Result<(), AppError> {
        let previous = { self.bootstrap_peers.read().await.clone() };
        let mut normalized: Vec<String> = nodes
            .into_iter()
            .map(|entry| entry.trim().to_string())
            .filter(|entry| !entry.is_empty())
            .collect();
        normalized.sort();
        normalized.dedup();

        let mut stale_peer_ids: HashSet<String> = previous
            .iter()
            .filter_map(|entry| Self::bootstrap_node_id(entry))
            .collect();
        stale_peer_ids.extend(
            normalized
                .iter()
                .filter_map(|entry| Self::bootstrap_node_id(entry)),
        );
        let removed = self.prune_stale_bootstrap_peers(&stale_peer_ids).await;
        if removed > 0 {
            tracing::info!(
                removed,
                "Removed stale bootstrap peer entries before applying updated bootstrap nodes"
            );
        }

        {
            let mut cfg = self.network_config.write().await;
            cfg.bootstrap_peers = normalized.clone();
            cfg.bootstrap_source = source;
        }
        {
            let mut peers = self.bootstrap_peers.write().await;
            *peers = normalized.clone();
        }
        {
            let mut stored_source = self.bootstrap_source.write().await;
            *stored_source = source;
        }

        if normalized.is_empty() {
            tracing::warn!("Bootstrap nodes list is empty; skipping connections");
            return Ok(());
        }

        let success = self.connect_bootstrap_nodes(&normalized).await;
        if success > 0 {
            super::metrics::record_bootstrap_source(source);
        }
        Ok(())
    }
}
