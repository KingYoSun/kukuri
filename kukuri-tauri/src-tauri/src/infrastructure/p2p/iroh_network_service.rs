use super::{
    DiscoveryOptions, NetworkService, NetworkStats, Peer,
    dht_bootstrap::{DhtGossip, secret},
    utils::{normalize_endpoint_addr, parse_peer_hint},
};
use crate::domain::p2p::{P2PEvent, generate_topic_id, topic_id_bytes};
use crate::shared::config::{BootstrapSource, NetworkConfig as AppNetworkConfig};
use crate::shared::error::AppError;
use async_trait::async_trait;
use iroh::{
    Endpoint, RelayMode, RelayUrl, address_lookup::MemoryLookup, endpoint::QuicTransportConfig,
};
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use tracing;

const KUKURI_IROH_RELAY_MODE_ENV: &str = "KUKURI_IROH_RELAY_MODE";
const KUKURI_IROH_RELAY_URLS_ENV: &str = "KUKURI_IROH_RELAY_URLS";
const KUKURI_IROH_TRANSPORT_PROFILE_ENV: &str = "KUKURI_IROH_TRANSPORT_PROFILE";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EndpointTransportProfile {
    Default,
    RelayOnly,
}

impl EndpointTransportProfile {
    fn from_env_value(raw: Option<&str>) -> Result<Self, AppError> {
        let Some(raw) = raw else {
            return Ok(Self::Default);
        };

        match raw.trim().to_ascii_lowercase().as_str() {
            "" | "default" => Ok(Self::Default),
            "relay-only" | "relay_only" => Ok(Self::RelayOnly),
            other => Err(AppError::ConfigurationError(format!(
                "Invalid {KUKURI_IROH_TRANSPORT_PROFILE_ENV} value `{other}`"
            ))),
        }
    }

    fn quic_transport_config(self) -> Option<QuicTransportConfig> {
        match self {
            Self::Default => None,
            Self::RelayOnly => Some(
                QuicTransportConfig::builder()
                    .enable_segmentation_offload(false)
                    .send_observed_address_reports(false)
                    .receive_observed_address_reports(false)
                    .build(),
            ),
        }
    }

    fn apply_to_builder(self, builder: iroh::endpoint::Builder) -> iroh::endpoint::Builder {
        match self {
            Self::Default => builder,
            Self::RelayOnly => builder.clear_ip_transports(),
        }
    }

    fn allows_direct_discovery(self) -> bool {
        matches!(self, Self::Default)
    }

    fn exposes_direct_addresses(self) -> bool {
        matches!(self, Self::Default)
    }

    fn supports_mainline(self, discovery_options: DiscoveryOptions) -> bool {
        matches!(self, Self::Default) && discovery_options.enable_mainline()
    }
}

pub struct IrohNetworkService {
    endpoint: Arc<Endpoint>,
    static_discovery: Arc<MemoryLookup>,
    connected: Arc<RwLock<bool>>,
    peers: Arc<RwLock<Vec<Peer>>>,
    stats: Arc<RwLock<NetworkStats>>,
    dht_gossip: Option<Arc<DhtGossip>>,
    mainline_enabled: bool,
    discovery_options: Arc<RwLock<DiscoveryOptions>>,
    network_config: Arc<RwLock<AppNetworkConfig>>,
    bootstrap_peers: Arc<RwLock<Vec<String>>>,
    bootstrap_source: Arc<RwLock<BootstrapSource>>,
    p2p_event_tx: Option<broadcast::Sender<P2PEvent>>,
    transport_profile: EndpointTransportProfile,
}

impl IrohNetworkService {
    pub async fn new(
        secret_key: iroh::SecretKey,
        net_cfg: AppNetworkConfig,
        discovery_options: DiscoveryOptions,
        event_tx: Option<broadcast::Sender<P2PEvent>>,
    ) -> Result<Self, AppError> {
        // Configure the endpoint and discovery lookups for the selected transport profile.
        let static_discovery = Arc::new(MemoryLookup::new());
        let relay_mode = resolve_endpoint_relay_mode()?;
        let transport_profile = resolve_endpoint_transport_profile()?;
        let builder = transport_profile
            .apply_to_builder(Endpoint::empty_builder(relay_mode))
            .secret_key(secret_key);
        let builder = if let Some(transport_config) = transport_profile.quic_transport_config() {
            builder.transport_config(transport_config)
        } else {
            builder
        };
        let builder = if transport_profile.allows_direct_discovery() {
            discovery_options.apply_to_builder(builder)
        } else {
            builder
        };
        let builder = builder.address_lookup(static_discovery.clone());
        let endpoint = builder
            .bind()
            .await
            .map_err(|e| AppError::P2PError(format!("Failed to bind endpoint: {e:?}")))?;

        // Validate persisted bootstrap configuration for diagnostics.
        if let Err(e) = super::bootstrap_config::validate_bootstrap_config() {
            tracing::warn!("bootstrap_nodes.json validation failed: {:?}", e);
        }

        let mainline_enabled = transport_profile.supports_mainline(discovery_options);

        // Initialize DHT gossip only when mainline transport is enabled.
        let dht_gossip = if mainline_enabled {
            match DhtGossip::new(Arc::new(endpoint.clone())).await {
                Ok(service) => Some(Arc::new(service)),
                Err(e) => {
                    tracing::warn!("Failed to initialize DhtGossip: {:?}", e);
                    None
                }
            }
        } else {
            None
        };

        let network_config = Arc::new(RwLock::new(net_cfg.clone()));
        let endpoint = Arc::new(endpoint);
        let service = Self {
            endpoint: Arc::clone(&endpoint),
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
            mainline_enabled,
            discovery_options: Arc::new(RwLock::new(discovery_options)),
            network_config: Arc::clone(&network_config),
            bootstrap_peers: Arc::new(RwLock::new(net_cfg.bootstrap_peers.clone())),
            bootstrap_source: Arc::new(RwLock::new(net_cfg.bootstrap_source)),
            p2p_event_tx: event_tx,
            transport_profile,
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

    pub fn exposes_direct_addresses(&self) -> bool {
        self.transport_profile.exposes_direct_addresses()
    }

    pub fn supports_mainline(&self) -> bool {
        self.mainline_enabled
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
                    tracing::info!("Registered bootstrap peer from config: {}", trimmed);
                }
                Err(err) => {
                    tracing::warn!("Failed to register bootstrap peer '{}': {}", trimmed, err);
                }
            }
        }
        success_count
    }

    async fn upsert_known_peer(&self, node_id: &str, address: &str) {
        let mut peers = self.peers.write().await;
        let now = chrono::Utc::now().timestamp();
        if let Some(existing) = peers.iter_mut().find(|peer| peer.id == node_id) {
            existing.address = address.to_string();
            existing.last_seen = now;
        } else {
            peers.push(Peer {
                id: node_id.to_string(),
                address: address.to_string(),
                connected_at: now,
                last_seen: now,
            });
        }
        let connected_count = peers.len();
        drop(peers);

        let mut stats = self.stats.write().await;
        stats.connected_peers = connected_count;
        super::metrics::set_mainline_connected_peers(stats.connected_peers as u64);
    }

    async fn register_peer_endpoint(
        &self,
        node_id: &str,
        address: &str,
        node_addr: iroh::EndpointAddr,
    ) -> Result<(), AppError> {
        self.static_discovery.add_endpoint_info(node_addr.clone());
        self.upsert_known_peer(node_id, address).await;
        tracing::debug!(
            node_id = %node_id,
            "Registered peer endpoint for future gossip joins"
        );
        Ok(())
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
        // Resolve local endpoint hints in `node_id@ip:port` and relay hint formats.
        self.endpoint.online().await;
        let node_addr = self.endpoint.addr();
        let node_id = node_addr.id.to_string();
        let mut out = Vec::new();
        let direct_addrs: Vec<_> = if self.transport_profile.exposes_direct_addresses() {
            node_addr.ip_addrs().cloned().collect()
        } else {
            Vec::new()
        };
        let relay_urls: Vec<_> = node_addr
            .relay_urls()
            .map(|relay_url| relay_url.to_string())
            .collect();

        if !relay_urls.is_empty() {
            if direct_addrs.is_empty() {
                for relay_url in &relay_urls {
                    out.push(format!("{node_id}|relay={relay_url}"));
                }
            } else {
                for relay_url in &relay_urls {
                    for addr in &direct_addrs {
                        out.push(format!("{node_id}|relay={relay_url}|addr={addr}"));
                    }
                }
            }
        }

        for addr in direct_addrs {
            out.push(format!("{node_id}@{addr}"));
        }
        if out.is_empty() {
            out.push(node_id);
        }
        out.sort();
        out.dedup();
        Ok(out)
    }

    /// Join a DHT topic when mainline transport is available.
    pub async fn join_dht_topic(&self, topic_name: &str) -> Result<(), AppError> {
        if !self.mainline_enabled {
            tracing::debug!(
                "Skipping DHT topic join for {} because mainline is disabled",
                topic_name
            );
            return Ok(());
        }
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
            // Fall back to configured bootstrap peers when DHT initialization failed.
            self.connect_fallback().await?;
        }
        Ok(())
    }

    /// Leave a DHT topic when mainline transport is available.
    pub async fn leave_dht_topic(&self, topic_name: &str) -> Result<(), AppError> {
        if !self.mainline_enabled {
            return Ok(());
        }
        let canonical = generate_topic_id(topic_name);
        let topic_bytes = topic_id_bytes(&canonical);
        if let Some(ref dht_gossip) = self.dht_gossip {
            dht_gossip.leave_topic(&topic_bytes).await?;
            tracing::info!("Left DHT topic: {} (requested: {})", canonical, topic_name);
        }
        Ok(())
    }

    /// Broadcast a message over the DHT topic when mainline transport is available.
    pub async fn broadcast_dht(&self, topic_name: &str, message: Vec<u8>) -> Result<(), AppError> {
        if !self.mainline_enabled {
            tracing::debug!(
                "Skipping DHT broadcast for {} because mainline is disabled",
                topic_name
            );
            return Ok(());
        }
        let canonical = generate_topic_id(topic_name);
        let topic_bytes = topic_id_bytes(&canonical);
        if let Some(ref dht_gossip) = self.dht_gossip {
            dht_gossip.broadcast(&topic_bytes, message).await?;
        } else {
            return Err(AppError::P2PError("DHT service not available".to_string()));
        }
        Ok(())
    }

    /// Connect to fallback bootstrap peers when DHT is unavailable.
    async fn connect_fallback(&self) -> Result<(), AppError> {
        // Prefer configured bootstrap peers before using the built-in fallback set.
        let fallback_peers =
            match super::dht_bootstrap::fallback::connect_from_config(&self.endpoint).await {
                Ok(peers) => peers,
                Err(_) => {
                    // Fall back to the built-in peer list if configured bootstrap peers fail.
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

        // Add fallback peers to the in-memory peer list and discovery lookup.
        let mut peers = self.peers.write().await;
        let now = chrono::Utc::now().timestamp();

        for node_addr in fallback_peers {
            let node_addr = normalize_endpoint_addr(
                &node_addr,
                self.transport_profile.exposes_direct_addresses(),
            );
            peers.push(Peer {
                id: node_addr.id.to_string(),
                address: format!("{}@fallback", node_addr.id),
                connected_at: now,
                last_seen: now,
            });
            self.static_discovery.add_endpoint_info(node_addr);
        }

        // Refresh connection metrics.
        let mut stats = self.stats.write().await;
        stats.connected_peers = peers.len();
        super::metrics::set_mainline_connected_peers(stats.connected_peers as u64);

        Ok(())
    }

    /// Rotate the shared secret used by DHT fallback peers.
    pub async fn rotate_dht_secret(&self) -> Result<(), AppError> {
        secret::rotate_secret()
            .await
            .map_err(|e| AppError::P2PError(format!("Failed to rotate secret: {e:?}")))?;
        tracing::info!("DHT shared secret rotated");
        Ok(())
    }
}

fn resolve_endpoint_relay_mode() -> Result<RelayMode, AppError> {
    relay_mode_from_env_values(
        std::env::var(KUKURI_IROH_RELAY_MODE_ENV).ok(),
        std::env::var(KUKURI_IROH_RELAY_URLS_ENV).ok(),
    )
}

fn resolve_endpoint_transport_profile() -> Result<EndpointTransportProfile, AppError> {
    EndpointTransportProfile::from_env_value(
        std::env::var(KUKURI_IROH_TRANSPORT_PROFILE_ENV)
            .ok()
            .as_deref(),
    )
}

fn relay_mode_from_env_values(
    relay_mode_raw: Option<String>,
    relay_urls_raw: Option<String>,
) -> Result<RelayMode, AppError> {
    if relay_mode_raw
        .as_deref()
        .map(|value| value.trim().eq_ignore_ascii_case("default"))
        .unwrap_or(false)
    {
        return Ok(RelayMode::Default);
    }

    let relay_urls = parse_custom_relay_urls(relay_urls_raw.as_deref())?;
    if relay_urls.is_empty() {
        return Ok(RelayMode::Default);
    }

    Ok(RelayMode::custom(relay_urls))
}

fn parse_custom_relay_urls(raw: Option<&str>) -> Result<Vec<RelayUrl>, AppError> {
    let Some(raw) = raw else {
        return Ok(Vec::new());
    };

    let mut relay_urls = Vec::new();
    for entry in raw.split(',') {
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            continue;
        }
        let relay_url = RelayUrl::from_str(trimmed).map_err(|err| {
            AppError::ConfigurationError(format!(
                "Invalid {} entry `{trimmed}`: {err}",
                KUKURI_IROH_RELAY_URLS_ENV
            ))
        })?;
        if !relay_urls.contains(&relay_url) {
            relay_urls.push(relay_url);
        }
    }
    Ok(relay_urls)
}

#[cfg(test)]
mod relay_mode_tests {
    use super::{EndpointTransportProfile, parse_custom_relay_urls, relay_mode_from_env_values};
    use crate::infrastructure::p2p::DiscoveryOptions;
    use iroh::RelayMode;

    #[test]
    fn relay_mode_defaults_when_env_is_empty() {
        let mode = relay_mode_from_env_values(None, None).expect("relay mode");
        assert!(matches!(mode, RelayMode::Default));
    }

    #[test]
    fn relay_mode_defaults_when_flag_is_default() {
        let mode = relay_mode_from_env_values(
            Some("default".to_string()),
            Some("https://relay.example".to_string()),
        )
        .expect("relay mode");
        assert!(matches!(mode, RelayMode::Default));
    }

    #[test]
    fn relay_mode_uses_custom_when_urls_exist() {
        let mode = relay_mode_from_env_values(None, Some("https://relay.example".to_string()))
            .expect("relay mode");
        assert!(matches!(mode, RelayMode::Custom(_)));
    }

    #[test]
    fn parse_custom_relay_urls_rejects_invalid_url() {
        let err = parse_custom_relay_urls(Some("not-a-url")).expect_err("invalid relay url");
        assert!(
            err.to_string()
                .contains("Invalid KUKURI_IROH_RELAY_URLS entry")
        );
    }

    #[test]
    fn transport_profile_supports_relay_only() {
        let profile = EndpointTransportProfile::from_env_value(Some("relay-only"))
            .expect("transport profile should parse");
        assert_eq!(profile, EndpointTransportProfile::RelayOnly);

        let config = profile
            .quic_transport_config()
            .expect("relay-only config should be present");
        let debug = format!("{config:?}");
        assert!(debug.contains("enable_segmentation_offload: false"));
    }

    #[test]
    fn relay_only_profile_disables_mainline_even_if_discovery_requests_it() {
        let profile = EndpointTransportProfile::RelayOnly;
        assert!(!profile.supports_mainline(DiscoveryOptions::new(true, true, false)));
        assert!(EndpointTransportProfile::Default
            .supports_mainline(DiscoveryOptions::new(true, true, false)));
    }

    #[test]
    fn transport_profile_rejects_unknown_values() {
        let err = EndpointTransportProfile::from_env_value(Some("invalid-profile"))
            .expect_err("invalid transport profile should fail");
        assert!(
            err.to_string()
                .contains("Invalid KUKURI_IROH_TRANSPORT_PROFILE value")
        );
    }
}

#[cfg(test)]
mod connectivity_tests {
    use super::IrohNetworkService;
    use crate::infrastructure::p2p::{DiscoveryOptions, NetworkService};
    use crate::shared::config::AppConfig;
    use iroh::{Endpoint, RelayMode, SecretKey, protocol::Router};
    use iroh_gossip::Gossip;

    fn test_secret_key(seed: u8) -> SecretKey {
        SecretKey::from_bytes(&[seed; 32])
    }

    async fn spawn_gossip_peer(
        seed: u8,
    ) -> (
        Endpoint,
        Gossip,
        Router,
        String,
    ) {
        let endpoint = Endpoint::empty_builder(RelayMode::Disabled)
            .secret_key(test_secret_key(seed))
            .bind()
            .await
            .expect("bind endpoint");
        let gossip = Gossip::builder().spawn(endpoint.clone());
        let router = Router::builder(endpoint.clone())
            .accept(iroh_gossip::ALPN, gossip.clone())
            .spawn();
        let socket_addr = endpoint
            .addr()
            .ip_addrs()
            .next()
            .cloned()
            .expect("endpoint should expose direct address");
        let peer_hint = format!("{}@{}", endpoint.id(), socket_addr);

        (endpoint, gossip, router, peer_hint)
    }

    #[tokio::test]
    async fn add_peer_registers_peer_hint_without_immediate_dial() {
        let (_remote_endpoint, _remote_gossip, _remote_router, peer_hint) =
            spawn_gossip_peer(11).await;

        let service = IrohNetworkService::new(
            test_secret_key(22),
            AppConfig::default().network,
            DiscoveryOptions::default(),
            None,
        )
        .await
        .expect("network service");

        service
            .add_peer(&peer_hint)
            .await
            .expect("register reachable peer hint");

        let peers = service.get_peers().await.expect("peers");
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].address, peer_hint);
    }

    #[tokio::test]
    async fn add_peer_keeps_unreachable_hint_for_future_gossip_dial() {
        let (remote_endpoint, remote_gossip, remote_router, peer_hint) = spawn_gossip_peer(33).await;
        let remote_node_id = remote_endpoint.id();
        drop(remote_router);
        drop(remote_gossip);
        drop(remote_endpoint);

        let service = IrohNetworkService::new(
            test_secret_key(44),
            AppConfig::default().network,
            DiscoveryOptions::default(),
            None,
        )
        .await
        .expect("network service");

        service
            .add_peer(&peer_hint)
            .await
            .expect("unreachable peer hint should still be registered");

        let peers = service.get_peers().await.expect("peers");
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].id, remote_node_id.to_string());
        assert_eq!(peers[0].address, peer_hint);
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

        // Clear the tracked peer list on disconnect.
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
        let parsed_peer = parse_peer_hint(address).map_err(|e| {
            super::metrics::record_mainline_connection_failure();
            AppError::from(format!("Failed to parse peer address: {e}"))
        })?;
        let node_id = parsed_peer.node_id.to_string();

        if let Some(node_addr) = parsed_peer.node_addr {
            let node_addr =
                normalize_endpoint_addr(&node_addr, self.transport_profile.exposes_direct_addresses());
            self.register_peer_endpoint(&node_id, address, node_addr).await?;
        } else {
            tracing::info!(
                node_id = %node_id,
                "Peer hint has no direct address; relying on gossip discovery"
            );
            self.upsert_known_peer(&node_id, address).await;
        }
        super::metrics::record_mainline_connection_success();

        tracing::info!("Added peer: {}", address);
        Ok(())
    }

    async fn remove_peer(&self, peer_id: &str) -> Result<(), AppError> {
        let mut peers = self.peers.write().await;
        peers.retain(|p| p.id != peer_id);

        // Refresh connection metrics after peer removal.
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
