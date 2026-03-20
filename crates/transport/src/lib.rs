use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4, ToSocketAddrs};
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use chrono::Utc;
use futures_util::{Stream, StreamExt};
use iroh::address_lookup::{
    AddressLookup, DhtAddressLookup, EndpointInfo, Item as AddressLookupItem, MemoryLookup,
};
use iroh::endpoint::Builder as EndpointBuilder;
use iroh::protocol::Router;
use iroh::{
    Endpoint, EndpointAddr, EndpointId, RelayConfig, RelayMap, RelayMode, RelayUrl, SecretKey,
};
use iroh_gossip::api::{Event as GossipEvent, GossipSender};
use iroh_gossip::{ALPN as GOSSIP_ALPN, Gossip, TopicId as GossipTopicId};
use kukuri_core::{GossipHint, TopicId};
use pkarr::Client as PkarrClient;
use pkarr::{SignedPacket, Timestamp};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, Notify, RwLock, broadcast};
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout};
use tokio_stream::wrappers::BroadcastStream;
use tracing::{debug, warn};

pub type HintStream = Pin<Box<dyn Stream<Item = HintEnvelope> + Send>>;

const IROH_TXT_NAME: &str = "_iroh";
const DHT_PUBLISH_TTL_SECONDS: u32 = 30;
const DHT_PUBLISH_RETRY_INTERVAL: Duration = Duration::from_secs(2);
const DHT_PUBLISH_REPUBLISH_INTERVAL: Duration = Duration::from_secs(30);
const DHT_PUBLISH_STARTUP_TIMEOUT: Duration = Duration::from_secs(6);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HintEnvelope {
    pub hint: GossipHint,
    pub received_at: i64,
    pub source_peer: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerSnapshot {
    pub connected: bool,
    pub peer_count: usize,
    pub connected_peers: Vec<String>,
    pub configured_peers: Vec<String>,
    pub subscribed_topics: Vec<String>,
    pub pending_events: usize,
    pub status_detail: String,
    pub last_error: Option<String>,
    pub topic_diagnostics: Vec<TopicPeerSnapshot>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicPeerSnapshot {
    pub topic: String,
    pub joined: bool,
    pub peer_count: usize,
    pub connected_peers: Vec<String>,
    pub configured_peer_ids: Vec<String>,
    pub missing_peer_ids: Vec<String>,
    pub last_received_at: Option<i64>,
    pub status_detail: String,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransportNetworkConfig {
    pub bind_addr: SocketAddr,
    pub advertised_host: Option<String>,
    pub advertised_port: Option<u16>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryMode {
    #[default]
    StaticPeer,
    SeededDht,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectMode {
    #[default]
    DirectOnly,
    DirectOrRelay,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportRelayConfig {
    #[serde(default)]
    pub iroh_relay_urls: Vec<String>,
}

impl TransportRelayConfig {
    pub fn normalized(mut self) -> Self {
        self.iroh_relay_urls = self
            .iroh_relay_urls
            .into_iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        self
    }

    pub fn connect_mode(&self) -> ConnectMode {
        if self.iroh_relay_urls.is_empty() {
            ConnectMode::DirectOnly
        } else {
            ConnectMode::DirectOrRelay
        }
    }

    pub fn parsed_relay_urls(&self) -> Result<Vec<RelayUrl>> {
        self.iroh_relay_urls
            .iter()
            .map(|value| {
                value
                    .parse::<RelayUrl>()
                    .with_context(|| format!("invalid iroh relay url `{value}`"))
            })
            .collect()
    }

    pub fn relay_mode(&self) -> Result<RelayMode> {
        if self.iroh_relay_urls.is_empty() {
            return Ok(RelayMode::Disabled);
        }
        let relay_urls = self.parsed_relay_urls()?;
        Ok(RelayMode::Custom(RelayMap::from_iter(relay_urls)))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeedPeer {
    pub endpoint_id: String,
    pub addr_hint: Option<String>,
}

impl SeedPeer {
    pub fn to_endpoint_addr(&self) -> Result<EndpointAddr> {
        self.to_endpoint_addr_with_relays(&[])
    }

    pub fn to_endpoint_addr_with_relays(&self, relay_urls: &[RelayUrl]) -> Result<EndpointAddr> {
        let endpoint_id = EndpointId::from_str(self.endpoint_id.trim())
            .with_context(|| format!("invalid seed endpoint id `{}`", self.endpoint_id))?;
        let mut endpoint_addr = match self.addr_hint.as_deref() {
            Some(addr_hint) => {
                let socket_addrs = resolve_socket_addrs(addr_hint)?;
                build_endpoint_addr(endpoint_id, socket_addrs).ok_or_else(|| {
                    anyhow!("seed peer must resolve to at least one socket address")
                })?
            }
            None => endpoint_addr_with_relays(endpoint_id, relay_urls),
        };
        for relay_url in relay_urls {
            endpoint_addr = endpoint_addr.with_relay_url(relay_url.clone());
        }
        Ok(endpoint_addr)
    }

    pub fn display(&self) -> String {
        match self.addr_hint.as_deref() {
            Some(addr_hint) => format!("{}@{}", self.endpoint_id, addr_hint.trim()),
            None => self.endpoint_id.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoverySnapshot {
    pub mode: DiscoveryMode,
    pub connect_mode: ConnectMode,
    pub env_locked: bool,
    pub configured_seed_peer_ids: Vec<String>,
    pub bootstrap_seed_peer_ids: Vec<String>,
    pub manual_ticket_peer_ids: Vec<String>,
    pub connected_peer_ids: Vec<String>,
    pub local_endpoint_id: String,
    pub last_discovery_error: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct DhtDiscoveryOptions {
    pub enabled: bool,
    pub client: Option<PkarrClient>,
}

impl DhtDiscoveryOptions {
    pub fn disabled() -> Self {
        Self::default()
    }

    pub fn seeded_dht() -> Self {
        Self {
            enabled: true,
            client: None,
        }
    }

    pub fn with_client(client: PkarrClient) -> Self {
        Self {
            enabled: true,
            client: Some(client),
        }
    }

    fn publish_client(&self) -> Result<Option<PkarrClient>> {
        if !self.enabled {
            return Ok(None);
        }
        if let Some(client) = self.client.as_ref() {
            return Ok(Some(client.clone()));
        }
        let mut builder = PkarrClient::builder();
        builder.no_default_network();
        builder.cache_size(0);
        builder.dht(|dht| dht);
        Ok(Some(builder.build().context(
            "failed to build pkarr client for endpoint publication",
        )?))
    }
}

impl Default for TransportNetworkConfig {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)),
            advertised_host: None,
            advertised_port: None,
        }
    }
}

impl TransportNetworkConfig {
    pub fn loopback() -> Self {
        Self {
            bind_addr: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)),
            advertised_host: None,
            advertised_port: None,
        }
    }

    pub fn from_env() -> Result<Self> {
        let bind_addr = std::env::var("KUKURI_BIND_ADDR")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(|value| SocketAddr::from_str(value.trim()))
            .transpose()
            .context("failed to parse KUKURI_BIND_ADDR")?
            .unwrap_or_else(|| SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)));
        let advertised_host = std::env::var("KUKURI_ADVERTISE_HOST")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let advertised_port = std::env::var("KUKURI_ADVERTISE_PORT")
            .ok()
            .map(|value| value.trim().parse::<u16>())
            .transpose()
            .context("failed to parse KUKURI_ADVERTISE_PORT")?;

        Ok(Self {
            bind_addr,
            advertised_host,
            advertised_port,
        })
    }
}

#[async_trait]
pub trait Transport: Send + Sync {
    async fn peers(&self) -> Result<PeerSnapshot>;
    async fn export_ticket(&self) -> Result<Option<String>>;
    async fn import_ticket(&self, ticket: &str) -> Result<()>;
    async fn configure_discovery(
        &self,
        _mode: DiscoveryMode,
        _env_locked: bool,
        _configured_seed_peers: Vec<SeedPeer>,
        _bootstrap_seed_peers: Vec<SeedPeer>,
    ) -> Result<()> {
        Ok(())
    }
    async fn discovery(&self) -> Result<DiscoverySnapshot> {
        Ok(DiscoverySnapshot::default())
    }
}

#[async_trait]
pub trait HintTransport: Send + Sync {
    async fn subscribe_hints(&self, topic: &TopicId) -> Result<HintStream>;
    async fn unsubscribe_hints(&self, topic: &TopicId) -> Result<()>;
    async fn publish_hint(&self, topic: &TopicId, hint: GossipHint) -> Result<()>;
}

#[derive(Clone, Default)]
pub struct FakeNetwork {
    hints: Arc<Mutex<HashMap<String, broadcast::Sender<HintEnvelope>>>>,
    known_peers: Arc<Mutex<BTreeSet<String>>>,
}

#[derive(Clone)]
pub struct FakeTransport {
    local_id: String,
    network: FakeNetwork,
    configured_seed_peers: Arc<Mutex<BTreeSet<String>>>,
    bootstrap_seed_peers: Arc<Mutex<BTreeSet<String>>>,
    imported_peers: Arc<Mutex<BTreeSet<String>>>,
    subscribed_topics: Arc<Mutex<BTreeSet<String>>>,
    discovery_mode: Arc<Mutex<DiscoveryMode>>,
    env_locked: Arc<Mutex<bool>>,
}

impl FakeTransport {
    pub fn new(local_id: impl Into<String>, network: FakeNetwork) -> Self {
        Self {
            local_id: local_id.into(),
            network,
            configured_seed_peers: Arc::new(Mutex::new(BTreeSet::new())),
            bootstrap_seed_peers: Arc::new(Mutex::new(BTreeSet::new())),
            imported_peers: Arc::new(Mutex::new(BTreeSet::new())),
            subscribed_topics: Arc::new(Mutex::new(BTreeSet::new())),
            discovery_mode: Arc::new(Mutex::new(DiscoveryMode::StaticPeer)),
            env_locked: Arc::new(Mutex::new(false)),
        }
    }

    async fn hint_sender(&self, topic: &TopicId) -> broadcast::Sender<HintEnvelope> {
        let mut topics = self.network.hints.lock().await;
        topics
            .entry(topic.0.clone())
            .or_insert_with(|| broadcast::channel(128).0)
            .clone()
    }
}

#[async_trait]
impl Transport for FakeTransport {
    async fn peers(&self) -> Result<PeerSnapshot> {
        let mut imported = self
            .imported_peers
            .lock()
            .await
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        for peer in self.configured_seed_peers.lock().await.iter() {
            if !imported.contains(peer) {
                imported.push(peer.clone());
            }
        }
        for peer in self.bootstrap_seed_peers.lock().await.iter() {
            if !imported.contains(peer) {
                imported.push(peer.clone());
            }
        }
        let topics = self
            .subscribed_topics
            .lock()
            .await
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let topic_diagnostics = topics
            .iter()
            .cloned()
            .map(|topic| TopicPeerSnapshot {
                topic,
                joined: !imported.is_empty(),
                peer_count: imported.len(),
                connected_peers: imported.clone(),
                configured_peer_ids: imported.clone(),
                missing_peer_ids: Vec::new(),
                last_received_at: None,
                status_detail: topic_status_detail(imported.len(), imported.len()),
                last_error: None,
            })
            .collect::<Vec<_>>();
        Ok(PeerSnapshot {
            connected: !imported.is_empty(),
            peer_count: imported.len(),
            connected_peers: imported.clone(),
            configured_peers: imported,
            subscribed_topics: topics,
            pending_events: 0,
            status_detail: peer_status_detail(
                topic_diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.configured_peer_ids.len())
                    .max()
                    .unwrap_or(0),
                topic_diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.connected_peers.len())
                    .max()
                    .unwrap_or(0),
                topic_diagnostics.len(),
            ),
            last_error: None,
            topic_diagnostics,
        })
    }

    async fn export_ticket(&self) -> Result<Option<String>> {
        self.network
            .known_peers
            .lock()
            .await
            .insert(self.local_id.clone());
        Ok(Some(self.local_id.clone()))
    }

    async fn import_ticket(&self, ticket: &str) -> Result<()> {
        self.imported_peers.lock().await.insert(ticket.to_string());
        self.network
            .known_peers
            .lock()
            .await
            .insert(ticket.to_string());
        Ok(())
    }

    async fn configure_discovery(
        &self,
        mode: DiscoveryMode,
        env_locked: bool,
        configured_seed_peers: Vec<SeedPeer>,
        bootstrap_seed_peers: Vec<SeedPeer>,
    ) -> Result<()> {
        *self.discovery_mode.lock().await = mode;
        *self.env_locked.lock().await = env_locked;
        let configured = configured_seed_peers
            .into_iter()
            .map(|peer| peer.endpoint_id)
            .collect::<BTreeSet<_>>();
        let bootstrap = bootstrap_seed_peers
            .into_iter()
            .map(|peer| peer.endpoint_id)
            .collect::<BTreeSet<_>>();
        *self.configured_seed_peers.lock().await = configured;
        *self.bootstrap_seed_peers.lock().await = bootstrap;
        Ok(())
    }

    async fn discovery(&self) -> Result<DiscoverySnapshot> {
        let configured_seed_peer_ids = self
            .configured_seed_peers
            .lock()
            .await
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let bootstrap_seed_peer_ids = self
            .bootstrap_seed_peers
            .lock()
            .await
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let manual_ticket_peer_ids = self
            .imported_peers
            .lock()
            .await
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let mut connected_peer_ids = manual_ticket_peer_ids.clone();
        for peer in configured_seed_peer_ids
            .iter()
            .chain(bootstrap_seed_peer_ids.iter())
        {
            if !connected_peer_ids.contains(peer) {
                connected_peer_ids.push(peer.clone());
            }
        }
        Ok(DiscoverySnapshot {
            mode: self.discovery_mode.lock().await.clone(),
            connect_mode: ConnectMode::DirectOnly,
            env_locked: *self.env_locked.lock().await,
            configured_seed_peer_ids,
            bootstrap_seed_peer_ids,
            manual_ticket_peer_ids,
            connected_peer_ids,
            local_endpoint_id: self.local_id.clone(),
            last_discovery_error: None,
        })
    }
}

#[async_trait]
impl HintTransport for FakeTransport {
    async fn subscribe_hints(&self, topic: &TopicId) -> Result<HintStream> {
        let hint_topic = TopicId::new(format!("hint/{}", topic.as_str()));
        self.subscribed_topics
            .lock()
            .await
            .insert(hint_topic.as_str().to_string());
        let sender = self.hint_sender(topic).await;
        let stream =
            BroadcastStream::new(sender.subscribe()).filter_map(|event| async move { event.ok() });
        Ok(Box::pin(stream))
    }

    async fn unsubscribe_hints(&self, topic: &TopicId) -> Result<()> {
        let hint_topic = TopicId::new(format!("hint/{}", topic.as_str()));
        self.subscribed_topics
            .lock()
            .await
            .remove(hint_topic.as_str());
        Ok(())
    }

    async fn publish_hint(&self, topic: &TopicId, hint: GossipHint) -> Result<()> {
        let sender = self.hint_sender(topic).await;
        let _ = sender.send(HintEnvelope {
            hint,
            received_at: Utc::now().timestamp_millis(),
            source_peer: self.local_id.clone(),
        });
        Ok(())
    }
}

struct HintTopicState {
    sender: Arc<Mutex<GossipSender>>,
    broadcaster: broadcast::Sender<HintEnvelope>,
    bootstrap_peer_ids: BTreeSet<String>,
    neighbors: Arc<RwLock<BTreeSet<String>>>,
    last_received_at: Arc<Mutex<Option<i64>>>,
    last_error: Arc<Mutex<Option<String>>>,
    _receiver_task: JoinHandle<()>,
}

#[derive(Clone, Debug)]
struct RelayFallbackLookup {
    relay_urls: Vec<RelayUrl>,
}

impl RelayFallbackLookup {
    fn new(relay_urls: Vec<RelayUrl>) -> Self {
        Self { relay_urls }
    }
}

impl AddressLookup for RelayFallbackLookup {
    fn resolve(
        &self,
        endpoint_id: EndpointId,
    ) -> Option<
        futures_util::stream::BoxStream<
            'static,
            Result<AddressLookupItem, iroh::address_lookup::Error>,
        >,
    > {
        if self.relay_urls.is_empty() {
            return None;
        }
        let endpoint_info =
            EndpointInfo::from(endpoint_addr_with_relays(endpoint_id, &self.relay_urls));
        Some(Box::pin(futures_util::stream::once(async move {
            Ok(AddressLookupItem::new(
                endpoint_info,
                "community-relay-fallback",
                None,
            ))
        })))
    }
}

pub struct IrohGossipTransport {
    endpoint: Endpoint,
    gossip: Gossip,
    _router: Option<Router>,
    _endpoint_publish_task: Option<JoinHandle<()>>,
    discovery: Arc<MemoryLookup>,
    network_config: TransportNetworkConfig,
    configured_seed_peers: Arc<Mutex<BTreeMap<String, EndpointAddr>>>,
    bootstrap_seed_peers: Arc<Mutex<BTreeMap<String, EndpointAddr>>>,
    imported_peers: Arc<Mutex<BTreeMap<String, EndpointAddr>>>,
    subscribed_topics: Arc<Mutex<BTreeSet<String>>>,
    topic_states: Arc<Mutex<HashMap<String, HintTopicState>>>,
    last_error: Arc<Mutex<Option<String>>>,
    discovery_mode: Arc<Mutex<DiscoveryMode>>,
    connect_mode: Arc<Mutex<ConnectMode>>,
    relay_urls: Mutex<Vec<RelayUrl>>,
    env_locked: Arc<Mutex<bool>>,
}

impl IrohGossipTransport {
    pub async fn bind(network_config: TransportNetworkConfig) -> Result<Self> {
        Self::bind_with_options(
            network_config,
            DhtDiscoveryOptions::disabled(),
            TransportRelayConfig::default(),
        )
        .await
    }

    pub async fn bind_with_options(
        network_config: TransportNetworkConfig,
        dht_options: DhtDiscoveryOptions,
        relay_config: TransportRelayConfig,
    ) -> Result<Self> {
        let relay_config = relay_config.normalized();
        let relay_urls = relay_config.parsed_relay_urls()?;
        let (endpoint, discovery, publish_task) =
            bind_endpoint_with_options(network_config.bind_addr, &dht_options, &relay_config, None)
                .await?;

        let gossip = Gossip::builder().spawn(endpoint.clone());
        let router = Router::builder(endpoint.clone())
            .accept(GOSSIP_ALPN, gossip.clone())
            .spawn();

        Ok(Self {
            endpoint,
            gossip,
            _router: Some(router),
            _endpoint_publish_task: publish_task,
            discovery,
            network_config,
            configured_seed_peers: Arc::new(Mutex::new(BTreeMap::new())),
            bootstrap_seed_peers: Arc::new(Mutex::new(BTreeMap::new())),
            imported_peers: Arc::new(Mutex::new(BTreeMap::new())),
            subscribed_topics: Arc::new(Mutex::new(BTreeSet::new())),
            topic_states: Arc::new(Mutex::new(HashMap::new())),
            last_error: Arc::new(Mutex::new(None)),
            discovery_mode: Arc::new(Mutex::new(DiscoveryMode::StaticPeer)),
            connect_mode: Arc::new(Mutex::new(relay_config.connect_mode())),
            relay_urls: Mutex::new(relay_urls),
            env_locked: Arc::new(Mutex::new(false)),
        })
    }

    pub async fn bind_with_discovery(
        network_config: TransportNetworkConfig,
        dht_options: DhtDiscoveryOptions,
    ) -> Result<Self> {
        Self::bind_with_options(network_config, dht_options, TransportRelayConfig::default()).await
    }

    pub fn from_shared_parts(
        endpoint: Endpoint,
        gossip: Gossip,
        discovery: Arc<MemoryLookup>,
        network_config: TransportNetworkConfig,
        relay_config: TransportRelayConfig,
    ) -> Result<Self> {
        let relay_config = relay_config.normalized();
        let relay_urls = relay_config.parsed_relay_urls()?;
        discovery.add_endpoint_info(endpoint.addr());
        Ok(Self {
            endpoint,
            gossip,
            _router: None,
            _endpoint_publish_task: None,
            discovery,
            network_config,
            configured_seed_peers: Arc::new(Mutex::new(BTreeMap::new())),
            bootstrap_seed_peers: Arc::new(Mutex::new(BTreeMap::new())),
            imported_peers: Arc::new(Mutex::new(BTreeMap::new())),
            subscribed_topics: Arc::new(Mutex::new(BTreeSet::new())),
            topic_states: Arc::new(Mutex::new(HashMap::new())),
            last_error: Arc::new(Mutex::new(None)),
            discovery_mode: Arc::new(Mutex::new(DiscoveryMode::StaticPeer)),
            connect_mode: Arc::new(Mutex::new(relay_config.connect_mode())),
            relay_urls: Mutex::new(relay_urls),
            env_locked: Arc::new(Mutex::new(false)),
        })
    }

    pub async fn bind_local() -> Result<Self> {
        Self::bind(TransportNetworkConfig::loopback()).await
    }

    pub async fn bind_from_env() -> Result<Self> {
        Self::bind(TransportNetworkConfig::from_env()?).await
    }

    async fn remove_topic_state(&self, topic: &str) {
        if let Some(state) = self.topic_states.lock().await.remove(topic) {
            state._receiver_task.abort();
            drop(state.sender);
        }
        self.subscribed_topics.lock().await.remove(topic);
    }

    pub async fn update_relay_config(&self, relay_config: TransportRelayConfig) -> Result<()> {
        let relay_config = relay_config.normalized();
        let relay_urls = relay_config.parsed_relay_urls()?;
        *self.connect_mode.lock().await = relay_config.connect_mode();
        *self.relay_urls.lock().await = relay_urls;
        *self.last_error.lock().await = None;
        Ok(())
    }

    async fn bootstrap_peers(&self) -> Vec<EndpointAddr> {
        let mut peers = self
            .configured_seed_peers
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for peer in self.bootstrap_seed_peers.lock().await.values() {
            if !peers.iter().any(|existing| existing.id == peer.id) {
                peers.push(peer.clone());
            }
        }
        for peer in self.imported_peers.lock().await.values() {
            if !peers.iter().any(|existing| existing.id == peer.id) {
                peers.push(peer.clone());
            }
        }
        peers
    }

    async fn configured_seed_peer_ids(&self) -> Vec<String> {
        self.configured_seed_peers
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>()
    }

    async fn bootstrap_seed_peer_ids(&self) -> Vec<String> {
        self.bootstrap_seed_peers
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>()
    }

    async fn configured_peer_ids(&self) -> Vec<String> {
        self.bootstrap_peers()
            .await
            .into_iter()
            .map(|peer| peer.id.to_string())
            .collect::<Vec<_>>()
    }

    async fn connected_peer_ids(&self) -> Vec<String> {
        let mut connected = BTreeSet::new();
        for (_, state) in self.topic_states.lock().await.iter() {
            for peer in state.neighbors.read().await.iter() {
                connected.insert(peer.clone());
            }
        }
        connected.into_iter().collect::<Vec<_>>()
    }

    async fn ensure_hint_topic(&self, topic: &TopicId) -> Result<broadcast::Sender<HintEnvelope>> {
        let bootstrap_peers = self.bootstrap_peers().await;
        let bootstrap_peer_ids = bootstrap_peers
            .iter()
            .map(|peer| peer.id.to_string())
            .collect::<BTreeSet<_>>();

        let existing = {
            let topics = self.topic_states.lock().await;
            topics
                .get(topic.as_str())
                .map(|state| (state.broadcaster.clone(), state.bootstrap_peer_ids.clone()))
        };

        if let Some((broadcaster, existing_bootstrap_peer_ids)) = existing {
            if existing_bootstrap_peer_ids == bootstrap_peer_ids {
                self.subscribed_topics.lock().await.insert(topic.0.clone());
                return Ok(broadcaster);
            }
            self.remove_topic_state(topic.as_str()).await;
        }

        let bootstrap = bootstrap_peers
            .iter()
            .map(|peer| peer.id)
            .collect::<Vec<_>>();

        for peer in &bootstrap_peers {
            self.discovery.add_endpoint_info(peer.clone());
        }

        let topic_handle = match self
            .gossip
            .subscribe(topic_to_gossip_id(topic), bootstrap)
            .await
        {
            Ok(topic_handle) => topic_handle,
            Err(error) => {
                let message = format!("failed to subscribe gossip topic: {error}");
                *self.last_error.lock().await = Some(message.clone());
                return Err(anyhow!(message));
            }
        };
        let (sender, mut receiver) = topic_handle.split();
        let (broadcaster, _) = broadcast::channel(256);
        let outbound = broadcaster.clone();
        let joined = Arc::new(AtomicBool::new(bootstrap_peers.is_empty()));
        let joined_notify = Arc::new(Notify::new());
        let joined_task_state = Arc::clone(&joined);
        let joined_task_notify = Arc::clone(&joined_notify);
        let neighbors = Arc::new(RwLock::new(BTreeSet::new()));
        let neighbors_task = Arc::clone(&neighbors);
        let last_received_at = Arc::new(Mutex::new(None));
        let last_received_at_task = Arc::clone(&last_received_at);
        let last_error = Arc::new(Mutex::new(None));
        let last_error_task = Arc::clone(&last_error);
        let transport_last_error = Arc::clone(&self.last_error);
        let imported_count = bootstrap_peers.len();

        let task = tokio::spawn(async move {
            if imported_count > 0 {
                if timeout(Duration::from_secs(15), receiver.joined())
                    .await
                    .is_ok_and(|result| result.is_ok())
                {
                    joined_task_state.store(true, Ordering::SeqCst);
                    joined_task_notify.notify_waiters();
                    *last_error_task.lock().await = None;
                    *transport_last_error.lock().await = None;
                    let current_neighbors = receiver
                        .neighbors()
                        .map(|peer| peer.to_string())
                        .collect::<BTreeSet<_>>();
                    *neighbors_task.write().await = current_neighbors;
                } else {
                    let message = "timed out waiting for initial topic join".to_string();
                    *last_error_task.lock().await = Some(message.clone());
                    *transport_last_error.lock().await =
                        Some(format!("topic join pending: {message}"));
                }
            }
            while let Some(event) = receiver.next().await {
                match event {
                    Ok(GossipEvent::Received(message)) => {
                        let current_neighbors = receiver
                            .neighbors()
                            .map(|peer| peer.to_string())
                            .collect::<BTreeSet<_>>();
                        *neighbors_task.write().await = current_neighbors;
                        *last_received_at_task.lock().await = Some(Utc::now().timestamp_millis());
                        if let Ok(parsed) = serde_json::from_slice::<GossipHint>(&message.content) {
                            *last_error_task.lock().await = None;
                            *transport_last_error.lock().await = None;
                            let _ = outbound.send(HintEnvelope {
                                hint: parsed,
                                received_at: Utc::now().timestamp_millis(),
                                source_peer: String::new(),
                            });
                        } else {
                            *last_error_task.lock().await =
                                Some("failed to decode hint payload".to_string());
                        }
                    }
                    Ok(GossipEvent::NeighborUp(peer_id)) => {
                        let mut guard = neighbors_task.write().await;
                        guard.insert(peer_id.to_string());
                        *last_error_task.lock().await = None;
                        *transport_last_error.lock().await = None;
                    }
                    Ok(GossipEvent::NeighborDown(peer_id)) => {
                        let mut guard = neighbors_task.write().await;
                        guard.remove(peer_id.to_string().as_str());
                    }
                    Ok(GossipEvent::Lagged) => {}
                    Err(error) => {
                        let message = format!("gossip receiver closed: {error}");
                        *last_error_task.lock().await = Some(message.clone());
                        *transport_last_error.lock().await = Some(message);
                        break;
                    }
                }
            }
        });

        self.subscribed_topics.lock().await.insert(topic.0.clone());
        self.topic_states.lock().await.insert(
            topic.0.clone(),
            HintTopicState {
                sender: Arc::new(Mutex::new(sender)),
                broadcaster: broadcaster.clone(),
                bootstrap_peer_ids,
                neighbors,
                last_received_at,
                last_error,
                _receiver_task: task,
            },
        );

        Ok(broadcaster)
    }

    fn stream_from_sender(sender: &broadcast::Sender<HintEnvelope>) -> HintStream {
        let stream =
            BroadcastStream::new(sender.subscribe()).filter_map(|event| async move { event.ok() });
        Box::pin(stream)
    }
}

async fn bind_endpoint_with_options(
    bind_addr: SocketAddr,
    dht_options: &DhtDiscoveryOptions,
    relay_config: &TransportRelayConfig,
    secret_key: Option<SecretKey>,
) -> Result<(Endpoint, Arc<MemoryLookup>, Option<JoinHandle<()>>)> {
    let discovery = Arc::new(MemoryLookup::new());
    let mut builder = build_endpoint_builder(
        Endpoint::empty_builder(relay_config.relay_mode()?),
        &discovery,
        Some(dht_options),
        relay_config,
    )?;
    if let Some(secret_key) = secret_key {
        builder = builder.secret_key(secret_key);
    }
    #[cfg(test)]
    {
        builder = builder.insecure_skip_relay_cert_verify(true);
    }
    builder = apply_bind(builder, bind_addr)?;
    let endpoint = builder
        .bind()
        .await
        .context("failed to bind iroh endpoint")?;
    let publish_task =
        prepare_endpoint_for_discovery(&endpoint, &discovery, dht_options, relay_config).await?;
    Ok((endpoint, discovery, publish_task))
}

pub async fn prepare_endpoint_for_discovery(
    endpoint: &Endpoint,
    discovery: &Arc<MemoryLookup>,
    dht_options: &DhtDiscoveryOptions,
    relay_config: &TransportRelayConfig,
) -> Result<Option<JoinHandle<()>>> {
    let relay_backed = relay_config.connect_mode() == ConnectMode::DirectOrRelay;
    if relay_backed {
        endpoint.online().await;
    }
    discovery.add_endpoint_info(endpoint.addr());

    let Some(client) = dht_options.publish_client()? else {
        return Ok(None);
    };

    match timeout(DHT_PUBLISH_STARTUP_TIMEOUT, async {
        loop {
            match publish_endpoint_addr_once(endpoint, &client).await {
                Ok(true) => return Ok::<(), anyhow::Error>(()),
                Ok(false) => sleep(DHT_PUBLISH_RETRY_INTERVAL).await,
                Err(error) => {
                    debug!("initial endpoint publish retrying: {error:#}");
                    sleep(DHT_PUBLISH_RETRY_INTERVAL).await;
                }
            }
        }
    })
    .await
    {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            if relay_backed {
                debug!(
                    "initial endpoint publication failed; continuing with relay-only startup: {error:#}"
                );
            } else {
                warn!("initial endpoint publication failed: {error:#}");
            }
        }
        Err(_) => {
            if relay_backed {
                debug!(
                    "initial endpoint publication timed out; continuing with relay-only startup"
                );
            } else {
                warn!("initial endpoint publication timed out; continuing with background retries");
            }
        }
    }

    let endpoint = endpoint.clone();
    let task = tokio::spawn(async move {
        loop {
            let delay = match publish_endpoint_addr_once(&endpoint, &client).await {
                Ok(true) => DHT_PUBLISH_REPUBLISH_INTERVAL,
                Ok(false) => DHT_PUBLISH_RETRY_INTERVAL,
                Err(error) => {
                    if relay_backed {
                        debug!(
                            "failed to publish endpoint address to pkarr; relay path remains available: {error:#}"
                        );
                    } else {
                        warn!("failed to publish endpoint address to pkarr: {error:#}");
                    }
                    DHT_PUBLISH_RETRY_INTERVAL
                }
            };
            sleep(delay).await;
        }
    });
    Ok(Some(task))
}

async fn publish_endpoint_addr_once(endpoint: &Endpoint, client: &PkarrClient) -> Result<bool> {
    let endpoint_addr = endpoint.addr();
    if endpoint_addr.is_empty() {
        return Ok(false);
    }
    let endpoint_info = EndpointInfo::from(endpoint_addr);
    let public_key =
        pkarr::PublicKey::try_from(endpoint.id().as_bytes()).expect("pkarr public key");
    let previous_timestamp = client
        .resolve_most_recent(&public_key)
        .await
        .map(|packet| packet.timestamp());
    let now = Timestamp::now();
    let timestamp = match previous_timestamp {
        Some(previous) if previous >= now => previous + 1,
        _ => now,
    };
    let signed_packet = build_signed_packet_with_timestamp(
        &endpoint_info,
        endpoint.secret_key(),
        DHT_PUBLISH_TTL_SECONDS,
        timestamp,
    )?;
    client
        .publish(&signed_packet, previous_timestamp)
        .await
        .context("pkarr publish failed")?;
    Ok(true)
}

fn build_signed_packet_with_timestamp(
    endpoint_info: &EndpointInfo,
    secret_key: &SecretKey,
    ttl: u32,
    timestamp: Timestamp,
) -> Result<SignedPacket> {
    use pkarr::dns::{self, rdata};

    let keypair = pkarr::Keypair::from_secret_key(&secret_key.to_bytes());
    let mut builder = SignedPacket::builder().timestamp(timestamp);
    let name = dns::Name::new(IROH_TXT_NAME).expect("iroh txt name");
    for entry in endpoint_info.to_txt_strings() {
        let mut txt = rdata::TXT::new();
        txt.add_string(&entry)
            .context("invalid endpoint info txt entry")?;
        builder = builder.txt(name.clone(), txt.into_owned(), ttl);
    }
    builder
        .sign(&keypair)
        .context("failed to sign endpoint info packet")
}

impl Drop for IrohGossipTransport {
    fn drop(&mut self) {
        if let Some(task) = self._endpoint_publish_task.take() {
            task.abort();
        }
        if let Ok(mut topics) = self.topic_states.try_lock() {
            for (_, state) in topics.drain() {
                state._receiver_task.abort();
            }
        }
        if let Ok(mut subscribed_topics) = self.subscribed_topics.try_lock() {
            subscribed_topics.clear();
        }
    }
}

#[async_trait]
impl Transport for IrohGossipTransport {
    async fn peers(&self) -> Result<PeerSnapshot> {
        let topic_states = self
            .topic_states
            .lock()
            .await
            .iter()
            .map(|(topic, state)| {
                (
                    topic.clone(),
                    state.bootstrap_peer_ids.iter().cloned().collect::<Vec<_>>(),
                    Arc::clone(&state.neighbors),
                    Arc::clone(&state.last_received_at),
                    Arc::clone(&state.last_error),
                )
            })
            .collect::<Vec<_>>();
        let mut connected = BTreeSet::new();
        let configured_peers = self.configured_peer_ids().await;
        let mut topic_diagnostics = Vec::with_capacity(topic_states.len());
        for (topic, configured_peer_ids, neighbors, last_received_at, last_error) in topic_states {
            let peers = neighbors.read().await.iter().cloned().collect::<Vec<_>>();
            let last_received_at = *last_received_at.lock().await;
            let last_error = last_error.lock().await.clone();
            for peer in &peers {
                connected.insert(peer.clone());
            }
            let configured_peer_count = configured_peer_ids.len();
            let connected_peer_count = peers.len();
            let missing_peer_ids = configured_peer_ids
                .iter()
                .filter(|peer| !peers.iter().any(|connected_peer| connected_peer == *peer))
                .cloned()
                .collect::<Vec<_>>();
            topic_diagnostics.push(TopicPeerSnapshot {
                topic,
                joined: !peers.is_empty(),
                peer_count: connected_peer_count,
                connected_peers: peers,
                configured_peer_ids,
                missing_peer_ids,
                last_received_at,
                status_detail: topic_status_detail(configured_peer_count, connected_peer_count),
                last_error,
            });
        }
        topic_diagnostics.sort_by(|left, right| left.topic.cmp(&right.topic));
        let subscribed_topics = self
            .subscribed_topics
            .lock()
            .await
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let connected_peers = connected.into_iter().collect::<Vec<_>>();
        let configured_peer_count = configured_peers.len();
        let connected_peer_count = connected_peers.len();
        let subscribed_topic_count = topic_diagnostics.len();

        Ok(PeerSnapshot {
            connected: !connected_peers.is_empty(),
            peer_count: connected_peer_count,
            connected_peers,
            configured_peers,
            subscribed_topics,
            pending_events: 0,
            status_detail: peer_status_detail(
                configured_peer_count,
                connected_peer_count,
                subscribed_topic_count,
            ),
            last_error: self.last_error.lock().await.clone(),
            topic_diagnostics,
        })
    }

    async fn export_ticket(&self) -> Result<Option<String>> {
        let endpoint_addr = self.endpoint.addr();
        let ticket_config = ticket_network_config(
            &endpoint_addr,
            &self.endpoint.bound_sockets(),
            &self.network_config,
        );
        Ok(Some(encode_endpoint_ticket(
            &endpoint_addr,
            &ticket_config,
        )?))
    }

    async fn import_ticket(&self, ticket: &str) -> Result<()> {
        let endpoint_addr = match parse_endpoint_ticket(ticket) {
            Ok(endpoint_addr) => endpoint_addr,
            Err(error) => {
                let message = format!("failed to import peer ticket: {error}");
                *self.last_error.lock().await = Some(message.clone());
                return Err(anyhow!(message));
            }
        };
        self.discovery.add_endpoint_info(endpoint_addr.clone());
        self.imported_peers
            .lock()
            .await
            .insert(endpoint_addr.id.to_string(), endpoint_addr);
        *self.last_error.lock().await = None;
        Ok(())
    }

    async fn configure_discovery(
        &self,
        mode: DiscoveryMode,
        env_locked: bool,
        configured_seed_peers: Vec<SeedPeer>,
        bootstrap_seed_peers: Vec<SeedPeer>,
    ) -> Result<()> {
        let relay_urls = self.relay_urls.lock().await.clone();
        let mut configured = BTreeMap::new();
        for seed in configured_seed_peers {
            let endpoint_addr = seed.to_endpoint_addr_with_relays(&relay_urls)?;
            if !endpoint_addr.is_empty() {
                self.discovery.add_endpoint_info(endpoint_addr.clone());
            }
            configured.insert(endpoint_addr.id.to_string(), endpoint_addr);
        }
        let mut bootstrap = BTreeMap::new();
        for seed in bootstrap_seed_peers {
            let endpoint_addr = seed.to_endpoint_addr_with_relays(&relay_urls)?;
            if !endpoint_addr.is_empty() {
                self.discovery.add_endpoint_info(endpoint_addr.clone());
            }
            bootstrap.insert(endpoint_addr.id.to_string(), endpoint_addr);
        }
        *self.discovery_mode.lock().await = mode;
        *self.env_locked.lock().await = env_locked;
        *self.configured_seed_peers.lock().await = configured;
        *self.bootstrap_seed_peers.lock().await = bootstrap;
        *self.last_error.lock().await = None;
        Ok(())
    }

    async fn discovery(&self) -> Result<DiscoverySnapshot> {
        let configured_seed_peer_ids = self.configured_seed_peer_ids().await;
        let bootstrap_seed_peer_ids = self.bootstrap_seed_peer_ids().await;
        let manual_ticket_peer_ids = self
            .imported_peers
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        Ok(DiscoverySnapshot {
            mode: self.discovery_mode.lock().await.clone(),
            connect_mode: self.connect_mode.lock().await.clone(),
            env_locked: *self.env_locked.lock().await,
            configured_seed_peer_ids,
            bootstrap_seed_peer_ids,
            manual_ticket_peer_ids,
            connected_peer_ids: self.connected_peer_ids().await,
            local_endpoint_id: self.endpoint.id().to_string(),
            last_discovery_error: self.last_error.lock().await.clone(),
        })
    }
}

impl IrohGossipTransport {
    pub async fn shutdown(&self) {
        let topics = self
            .subscribed_topics
            .lock()
            .await
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        for topic in topics {
            self.remove_topic_state(topic.as_str()).await;
        }
    }
}

#[async_trait]
impl HintTransport for IrohGossipTransport {
    async fn subscribe_hints(&self, topic: &TopicId) -> Result<HintStream> {
        let hint_topic = TopicId::new(format!("hint/{}", topic.as_str()));
        let sender = self.ensure_hint_topic(&hint_topic).await?;
        Ok(Self::stream_from_sender(&sender))
    }

    async fn unsubscribe_hints(&self, topic: &TopicId) -> Result<()> {
        let hint_topic = TopicId::new(format!("hint/{}", topic.as_str()));
        self.remove_topic_state(hint_topic.as_str()).await;
        Ok(())
    }

    async fn publish_hint(&self, topic: &TopicId, hint: GossipHint) -> Result<()> {
        let hint_topic = TopicId::new(format!("hint/{}", topic.as_str()));
        let _ = self.ensure_hint_topic(&hint_topic).await?;
        let states = self.topic_states.lock().await;
        let state = states
            .get(hint_topic.as_str())
            .ok_or_else(|| anyhow!("missing hint topic sender"))?;
        let sender = state.sender.lock().await;
        let payload = serde_json::to_vec(&hint)?;
        if let Err(error) = sender.broadcast(payload.into()).await {
            let message = format!("failed to broadcast gossip hint: {error}");
            *state.last_error.lock().await = Some(message.clone());
            *self.last_error.lock().await = Some(message.clone());
            return Err(anyhow!(message));
        }
        *state.last_error.lock().await = None;
        *self.last_error.lock().await = None;
        Ok(())
    }
}

fn topic_to_gossip_id(topic: &TopicId) -> GossipTopicId {
    let hash = blake3::hash(topic.as_str().as_bytes());
    GossipTopicId::from_bytes(*hash.as_bytes())
}

pub fn build_endpoint_builder(
    builder: EndpointBuilder,
    discovery: &Arc<MemoryLookup>,
    dht_options: Option<&DhtDiscoveryOptions>,
    relay_config: &TransportRelayConfig,
) -> Result<EndpointBuilder> {
    let mut builder = builder.address_lookup(discovery.clone());
    let relay_urls = relay_config.parsed_relay_urls()?;
    if !relay_urls.is_empty() {
        builder = builder.address_lookup(RelayFallbackLookup::new(relay_urls));
    }
    if let Some(dht_options) = dht_options.filter(|options| options.enabled) {
        let mut dht_builder = DhtAddressLookup::builder()
            .include_direct_addresses(true)
            .no_publish();
        if let Some(client) = dht_options.client.as_ref() {
            dht_builder = dht_builder.client(client.clone());
        }
        builder = builder.address_lookup(dht_builder);
    }
    Ok(builder)
}

pub async fn sync_endpoint_relay_config(
    endpoint: &Endpoint,
    current: &[RelayUrl],
    next: &[RelayUrl],
) -> Result<()> {
    let current = current.iter().cloned().collect::<BTreeSet<_>>();
    let next = next.iter().cloned().collect::<BTreeSet<_>>();
    for relay_url in current.difference(&next) {
        endpoint.remove_relay(relay_url).await;
    }
    for relay_url in next.difference(&current) {
        endpoint
            .insert_relay(
                relay_url.clone(),
                Arc::new(RelayConfig::from(relay_url.clone())),
            )
            .await;
    }
    Ok(())
}

fn apply_bind(builder: EndpointBuilder, bind_addr: SocketAddr) -> Result<EndpointBuilder> {
    match bind_addr {
        SocketAddr::V4(addr) => builder
            .bind_addr(addr)
            .map_err(|error| anyhow!("failed to bind IPv4 address: {error}")),
        SocketAddr::V6(addr) => builder
            .bind_addr(addr)
            .map_err(|error| anyhow!("failed to bind IPv6 address: {error}")),
    }
}

fn ticket_network_config(
    endpoint_addr: &EndpointAddr,
    bound_sockets: &[SocketAddr],
    config: &TransportNetworkConfig,
) -> TransportNetworkConfig {
    let advertised_host = config.advertised_host.clone().or_else(|| {
        bound_sockets
            .iter()
            .find(|addr| is_reachable_ip(addr.ip()) || addr.ip().is_loopback())
            .map(|addr| addr.ip().to_string())
    });
    let advertised_port = config.advertised_port.or_else(|| {
        bound_sockets
            .iter()
            .find(|addr| addr.port() != 0)
            .map(|addr| addr.port())
    });

    if advertised_host.is_none() && advertised_port.is_none() {
        return config.clone();
    }

    TransportNetworkConfig {
        bind_addr: config.bind_addr,
        advertised_host: advertised_host.or_else(|| {
            endpoint_addr
                .ip_addrs()
                .find(|addr| is_reachable_ip(addr.ip()) || addr.ip().is_loopback())
                .map(|addr| addr.ip().to_string())
        }),
        advertised_port: advertised_port.or_else(|| {
            endpoint_addr
                .ip_addrs()
                .find(|addr| addr.port() != 0)
                .map(|addr| addr.port())
        }),
    }
}

pub fn encode_endpoint_ticket(
    endpoint_addr: &EndpointAddr,
    config: &TransportNetworkConfig,
) -> Result<String> {
    let advertised_port = config
        .advertised_port
        .or_else(|| {
            endpoint_addr
                .ip_addrs()
                .find(|addr| addr.port() != 0)
                .map(|addr| addr.port())
        })
        .or_else(|| match config.bind_addr {
            SocketAddr::V4(addr) if addr.port() != 0 => Some(addr.port()),
            SocketAddr::V6(addr) if addr.port() != 0 => Some(addr.port()),
            _ => None,
        })
        .ok_or_else(|| anyhow!("could not determine advertised port"))?;
    let advertised_host = config
        .advertised_host
        .clone()
        .or_else(|| {
            endpoint_addr
                .ip_addrs()
                .filter(|addr| is_reachable_ip(addr.ip()))
                .map(|addr| addr.ip().to_string())
                .next()
        })
        .or_else(|| match config.bind_addr.ip() {
            ip if is_reachable_ip(ip) => Some(ip.to_string()),
            IpAddr::V4(ip) if ip.is_loopback() => Some(ip.to_string()),
            IpAddr::V6(ip) if ip.is_loopback() => Some(ip.to_string()),
            _ => None,
        })
        .ok_or_else(|| anyhow!("could not determine advertised host"))?;

    Ok(format!(
        "{}@{}",
        endpoint_addr.id,
        format_host_port(&advertised_host, advertised_port)
    ))
}

pub fn parse_seed_peer(value: &str) -> Result<SeedPeer> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        anyhow::bail!("seed peer must not be empty");
    }
    let seed = if let Some((endpoint_id, addr_hint)) = trimmed.split_once('@') {
        SeedPeer {
            endpoint_id: endpoint_id.trim().to_string(),
            addr_hint: Some(addr_hint.trim().to_string()),
        }
    } else {
        SeedPeer {
            endpoint_id: trimmed.to_string(),
            addr_hint: None,
        }
    };
    let _ = seed.to_endpoint_addr()?;
    Ok(seed)
}

pub fn parse_endpoint_ticket(ticket: &str) -> Result<EndpointAddr> {
    let (node_id, socket_addr) = ticket
        .split_once('@')
        .ok_or_else(|| anyhow!("ticket must be formatted as <node_id>@<host:port>"))?;
    let endpoint_id = EndpointId::from_str(node_id).context("invalid endpoint id")?;
    let socket_addrs = resolve_socket_addrs(socket_addr)?;
    build_endpoint_addr(endpoint_id, socket_addrs)
        .ok_or_else(|| anyhow!("ticket must resolve to at least one socket address"))
}

fn build_endpoint_addr(
    endpoint_id: EndpointId,
    socket_addrs: Vec<SocketAddr>,
) -> Option<EndpointAddr> {
    if socket_addrs.is_empty() {
        return None;
    }

    let mut endpoint_addr = EndpointAddr::new(endpoint_id);
    for socket_addr in socket_addrs {
        endpoint_addr = endpoint_addr.with_ip_addr(socket_addr);
    }
    Some(endpoint_addr)
}

fn endpoint_addr_with_relays(endpoint_id: EndpointId, relay_urls: &[RelayUrl]) -> EndpointAddr {
    let mut endpoint_addr = EndpointAddr::new(endpoint_id);
    for relay_url in relay_urls {
        endpoint_addr = endpoint_addr.with_relay_url(relay_url.clone());
    }
    endpoint_addr
}

fn resolve_socket_addrs(value: &str) -> Result<Vec<SocketAddr>> {
    let trimmed = value.trim();
    if let Ok(socket_addr) = trimmed.parse::<SocketAddr>() {
        return Ok(vec![socket_addr]);
    }

    let (host, port_raw) = trimmed
        .rsplit_once(':')
        .ok_or_else(|| anyhow!("invalid socket address: {value}"))?;
    let host = host.trim().trim_start_matches('[').trim_end_matches(']');
    let port = port_raw
        .trim()
        .parse::<u16>()
        .with_context(|| format!("invalid port in `{value}`"))?;

    let addrs = if host.eq_ignore_ascii_case("localhost") {
        vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)]
    } else {
        (host, port)
            .to_socket_addrs()
            .with_context(|| format!("failed to resolve host `{host}`"))?
            .collect::<Vec<_>>()
    };

    Ok(prioritize_socket_addrs(addrs))
}

fn prioritize_socket_addrs(addrs: Vec<SocketAddr>) -> Vec<SocketAddr> {
    let mut unique = Vec::new();
    for addr in addrs {
        if !unique.contains(&addr) {
            unique.push(addr);
        }
    }

    let mut ipv4 = Vec::new();
    let mut other = Vec::new();
    for addr in unique {
        if addr.is_ipv4() {
            ipv4.push(addr);
        } else {
            other.push(addr);
        }
    }
    ipv4.extend(other);
    ipv4
}

fn format_host_port(host: &str, port: u16) -> String {
    let trimmed = host.trim().trim_start_matches('[').trim_end_matches(']');
    if trimmed.contains(':') {
        format!("[{trimmed}]:{port}")
    } else {
        format!("{trimmed}:{port}")
    }
}

fn is_reachable_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => !ip.is_unspecified() && !ip.is_loopback(),
        IpAddr::V6(ip) => !ip.is_unspecified() && !ip.is_loopback(),
    }
}

fn peer_status_detail(
    configured_peer_count: usize,
    connected_peer_count: usize,
    subscribed_topic_count: usize,
) -> String {
    if configured_peer_count == 0 {
        "No peers configured".to_string()
    } else if subscribed_topic_count == 0 {
        "No topics subscribed locally".to_string()
    } else if connected_peer_count == 0 {
        "Waiting for configured peers to connect".to_string()
    } else if connected_peer_count < configured_peer_count {
        "Connected to a subset of configured peers".to_string()
    } else {
        "Connected to all configured peers".to_string()
    }
}

fn topic_status_detail(configured_peer_count: usize, connected_peer_count: usize) -> String {
    if configured_peer_count == 0 {
        "No peers configured for this topic".to_string()
    } else if connected_peer_count == 0 {
        "Waiting for configured peers to join this topic".to_string()
    } else if connected_peer_count < configured_peer_count {
        "Connected to a subset of configured peers for this topic".to_string()
    } else {
        "Connected to all configured peers for this topic".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iroh::address_lookup::EndpointInfo;
    use kukuri_core::{
        GossipHint, HintObjectRef, KukuriEnvelope, TopicId, build_post_envelope, generate_keys,
    };
    use pkarr::Timestamp;
    use pkarr::errors::{ConcurrencyError, PublishError};
    use pkarr::mainline::Testnet;
    use std::net::{Ipv4Addr, SocketAddrV4};
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock")
    }

    #[test]
    fn old_next_env_vars_are_not_used() {
        let _guard = env_lock();
        let legacy_bind_addr = legacy_env("BIND_ADDR");
        let legacy_advertise_host = legacy_env("ADVERTISE_HOST");
        let legacy_advertise_port = legacy_env("ADVERTISE_PORT");
        for key in [
            "KUKURI_BIND_ADDR",
            "KUKURI_ADVERTISE_HOST",
            "KUKURI_ADVERTISE_PORT",
            legacy_bind_addr.as_str(),
            legacy_advertise_host.as_str(),
            legacy_advertise_port.as_str(),
        ] {
            unsafe { std::env::remove_var(key) };
        }
        unsafe {
            std::env::set_var(legacy_bind_addr, "127.0.0.1:40123");
            std::env::set_var(legacy_advertise_host, "legacy-host");
            std::env::set_var(legacy_advertise_port, "40123");
        }

        let config = TransportNetworkConfig::from_env().expect("config");

        assert_eq!(
            config.bind_addr,
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))
        );
        assert_eq!(config.advertised_host, None);
        assert_eq!(config.advertised_port, None);
    }

    fn legacy_env(name: &str) -> String {
        format!("KUKURI_{}_{}", "NEXT", name)
    }

    fn dht_test_client(testnet: &Testnet) -> PkarrClient {
        let mut builder = PkarrClient::builder();
        builder.no_default_network().bootstrap(&testnet.bootstrap);
        builder.build().expect("pkarr client")
    }

    async fn publish_endpoint_to_testnet(endpoint: &Endpoint, testnet: &Testnet) {
        let client = dht_test_client(testnet);
        let public_key =
            pkarr::PublicKey::try_from(endpoint.id().as_bytes()).expect("pkarr public key");
        let expected_info = EndpointInfo::from(endpoint.addr());
        for _ in 0..20 {
            let previous_timestamp = client
                .resolve_most_recent(&public_key)
                .await
                .map(|packet| packet.timestamp());
            let now = Timestamp::now();
            let timestamp = match previous_timestamp {
                Some(previous) if previous >= now => previous + 1,
                _ => now,
            };
            let signed_packet = build_signed_packet_with_timestamp(
                &expected_info,
                endpoint.secret_key(),
                1,
                timestamp,
            )
            .expect("signed packet");
            match client.publish(&signed_packet, previous_timestamp).await {
                Ok(()) => break,
                Err(PublishError::Concurrency(
                    ConcurrencyError::ConflictRisk
                    | ConcurrencyError::NotMostRecent
                    | ConcurrencyError::CasFailed,
                )) => tokio::time::sleep(Duration::from_millis(50)).await,
                Err(error) => panic!("publish endpoint info: {error}"),
            }
        }
        timeout(Duration::from_secs(5), async {
            loop {
                if client
                    .resolve_most_recent(&public_key)
                    .await
                    .as_ref()
                    .and_then(|packet| EndpointInfo::from_pkarr_signed_packet(packet).ok())
                    .is_some_and(|packet_info| {
                        packet_info.to_txt_strings() == expected_info.to_txt_strings()
                    })
                {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("resolve published endpoint info");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn transport_two_process_hint_roundtrip_static_peer() {
        let transport_a = IrohGossipTransport::bind_local()
            .await
            .expect("transport a");
        let transport_b = IrohGossipTransport::bind_local()
            .await
            .expect("transport b");
        let ticket_a = transport_a
            .export_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = transport_b
            .export_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        transport_a
            .import_ticket(&ticket_b)
            .await
            .expect("import b");
        transport_b
            .import_ticket(&ticket_a)
            .await
            .expect("import a");
        let topic = TopicId::new("kukuri:topic:transport");
        let (_stream_a, mut stream_b) = tokio::try_join!(
            transport_a.subscribe_hints(&topic),
            transport_b.subscribe_hints(&topic)
        )
        .expect("subscribe both");
        timeout(Duration::from_secs(10), async {
            loop {
                let peers_a = transport_a.peers().await.expect("peers a");
                let peers_b = transport_b.peers().await.expect("peers b");
                if peers_a.peer_count >= 1 && peers_b.peer_count >= 1 {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("peer snapshot timeout");
        let hint = GossipHint::TopicObjectsChanged {
            topic_id: topic.clone(),
            objects: vec![HintObjectRef {
                object_id: "object-1".into(),
                object_kind: "post".into(),
            }],
        };

        transport_a
            .publish_hint(&topic, hint.clone())
            .await
            .expect("publish hint");
        let envelope = timeout(Duration::from_secs(10), stream_b.next())
            .await
            .expect("receive timeout")
            .expect("stream event");

        assert_eq!(envelope.hint, hint);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn transport_seeded_dht_can_connect_by_endpoint_id_without_ticket() {
        let testnet = Testnet::new(5).expect("testnet");
        let config = TransportNetworkConfig::loopback();
        let transport_a = IrohGossipTransport::bind_with_discovery(
            config.clone(),
            DhtDiscoveryOptions::with_client(dht_test_client(&testnet)),
        )
        .await
        .expect("transport a");
        let transport_b = IrohGossipTransport::bind_with_discovery(
            config,
            DhtDiscoveryOptions::with_client(dht_test_client(&testnet)),
        )
        .await
        .expect("transport b");
        let discovery_a = transport_a.discovery().await.expect("discovery a");
        let discovery_b = transport_b.discovery().await.expect("discovery b");
        publish_endpoint_to_testnet(&transport_a.endpoint, &testnet).await;
        publish_endpoint_to_testnet(&transport_b.endpoint, &testnet).await;

        transport_a
            .configure_discovery(
                DiscoveryMode::SeededDht,
                false,
                vec![SeedPeer {
                    endpoint_id: discovery_b.local_endpoint_id.clone(),
                    addr_hint: None,
                }],
                Vec::new(),
            )
            .await
            .expect("configure a");
        transport_b
            .configure_discovery(
                DiscoveryMode::SeededDht,
                false,
                vec![SeedPeer {
                    endpoint_id: discovery_a.local_endpoint_id.clone(),
                    addr_hint: None,
                }],
                Vec::new(),
            )
            .await
            .expect("configure b");

        let endpoint_b = EndpointId::from_str(&discovery_b.local_endpoint_id).expect("endpoint b");
        let connection = timeout(Duration::from_secs(20), async {
            loop {
                match transport_a
                    .endpoint
                    .connect(EndpointAddr::new(endpoint_b), GOSSIP_ALPN)
                    .await
                {
                    Ok(connection) => return connection,
                    Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
                }
            }
        })
        .await
        .expect("seeded dht connect timeout");

        drop(connection);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn transport_relay_backed_dht_publish_replaces_newer_stale_packet() {
        let testnet = Testnet::new(5).expect("testnet");
        let (_relay_map, relay_url, _guard) = iroh::test_utils::run_relay_server()
            .await
            .expect("relay server");
        let relay_config = TransportRelayConfig {
            iroh_relay_urls: vec![relay_url.to_string()],
        }
        .normalized();
        let secret_key = SecretKey::from_bytes(&[7u8; 32]);
        let client = dht_test_client(&testnet);
        let stale_info = EndpointInfo::from_parts(
            secret_key.public(),
            iroh::address_lookup::EndpointData::new([iroh::TransportAddr::Relay(
                "https://stale-relay.invalid"
                    .parse()
                    .expect("stale relay url"),
            )]),
        );
        let stale_packet = build_signed_packet_with_timestamp(
            &stale_info,
            &secret_key,
            30,
            Timestamp::now() + 300_000_000,
        )
        .expect("build stale packet");
        client
            .publish(&stale_packet, None)
            .await
            .expect("publish stale packet");

        let (endpoint, _discovery, _publish_task) = bind_endpoint_with_options(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)),
            &DhtDiscoveryOptions::with_client(client.clone()),
            &relay_config,
            Some(secret_key.clone()),
        )
        .await
        .expect("bind endpoint");

        let public_key =
            pkarr::PublicKey::try_from(endpoint.id().as_bytes()).expect("pkarr public key");
        timeout(Duration::from_secs(6), async {
            loop {
                if let Some(packet) = client.resolve_most_recent(&public_key).await
                    && let Ok(info) = EndpointInfo::from_pkarr_signed_packet(&packet)
                    && info.relay_urls().any(|candidate| candidate == &relay_url)
                {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("endpoint relay info never replaced stale packet");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn transport_custom_relay_static_peer_seed_peers_connect_without_ticket_import() {
        let (_relay_map, relay_url, _guard) = iroh::test_utils::run_relay_server()
            .await
            .expect("relay server");
        let relay_config = TransportRelayConfig {
            iroh_relay_urls: vec![relay_url.to_string()],
        }
        .normalized();
        let config = TransportNetworkConfig::loopback();
        let transport_a = IrohGossipTransport::bind_with_options(
            config.clone(),
            DhtDiscoveryOptions::disabled(),
            relay_config.clone(),
        )
        .await
        .expect("transport a");
        let transport_b = IrohGossipTransport::bind_with_options(
            config,
            DhtDiscoveryOptions::disabled(),
            relay_config.clone(),
        )
        .await
        .expect("transport b");
        let discovery_a = transport_a.discovery().await.expect("discovery a");
        let discovery_b = transport_b.discovery().await.expect("discovery b");

        transport_a
            .configure_discovery(
                DiscoveryMode::StaticPeer,
                false,
                vec![SeedPeer {
                    endpoint_id: discovery_b.local_endpoint_id.clone(),
                    addr_hint: None,
                }],
                Vec::new(),
            )
            .await
            .expect("configure a");
        transport_b
            .configure_discovery(
                DiscoveryMode::StaticPeer,
                false,
                vec![SeedPeer {
                    endpoint_id: discovery_a.local_endpoint_id.clone(),
                    addr_hint: None,
                }],
                Vec::new(),
            )
            .await
            .expect("configure b");

        let endpoint_b = EndpointId::from_str(&discovery_b.local_endpoint_id).expect("endpoint b");
        let connection = timeout(Duration::from_secs(20), async {
            loop {
                match transport_a
                    .endpoint
                    .connect(EndpointAddr::new(endpoint_b), GOSSIP_ALPN)
                    .await
                {
                    Ok(connection) => return connection,
                    Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
                }
            }
        })
        .await
        .expect("custom relay seed connect timeout");

        drop(connection);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn transport_custom_relay_lookup_connects_unknown_peer_without_dht_publish() {
        let (_relay_map, relay_url, _guard) = iroh::test_utils::run_relay_server()
            .await
            .expect("relay server");
        let relay_config = TransportRelayConfig {
            iroh_relay_urls: vec![relay_url.to_string()],
        }
        .normalized();
        let config = TransportNetworkConfig::loopback();
        let transport_a = IrohGossipTransport::bind_with_options(
            config.clone(),
            DhtDiscoveryOptions::disabled(),
            relay_config.clone(),
        )
        .await
        .expect("transport a");
        let transport_b = IrohGossipTransport::bind_with_options(
            config,
            DhtDiscoveryOptions::disabled(),
            relay_config,
        )
        .await
        .expect("transport b");

        let endpoint_b = transport_b.endpoint.id();
        let connection = timeout(Duration::from_secs(20), async {
            loop {
                match transport_a
                    .endpoint
                    .connect(EndpointAddr::new(endpoint_b), GOSSIP_ALPN)
                    .await
                {
                    Ok(connection) => return connection,
                    Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
                }
            }
        })
        .await
        .expect("custom relay lookup connect timeout");

        drop(connection);
    }

    #[test]
    fn dht_publish_client_disables_pkarr_cache() {
        let client = DhtDiscoveryOptions::seeded_dht()
            .publish_client()
            .expect("publish client")
            .expect("pkarr client");

        assert!(client.cache().is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn transport_peer_snapshot_reports_seeded_dht_mode() {
        let testnet = Testnet::new(5).expect("testnet");
        let transport = IrohGossipTransport::bind_with_discovery(
            TransportNetworkConfig::loopback(),
            DhtDiscoveryOptions::with_client(dht_test_client(&testnet)),
        )
        .await
        .expect("transport");
        publish_endpoint_to_testnet(&transport.endpoint, &testnet).await;
        let local_endpoint_id = transport
            .discovery()
            .await
            .expect("discovery")
            .local_endpoint_id;
        transport
            .configure_discovery(
                DiscoveryMode::SeededDht,
                false,
                vec![SeedPeer {
                    endpoint_id: local_endpoint_id.clone(),
                    addr_hint: None,
                }],
                Vec::new(),
            )
            .await
            .expect("configure discovery");

        let snapshot = transport.discovery().await.expect("discovery snapshot");
        let peers = transport.peers().await.expect("peer snapshot");

        assert_eq!(snapshot.mode, DiscoveryMode::SeededDht);
        assert_eq!(snapshot.connect_mode, ConnectMode::DirectOnly);
        assert_eq!(
            snapshot.configured_seed_peer_ids,
            vec![local_endpoint_id.clone()]
        );
        assert!(snapshot.bootstrap_seed_peer_ids.is_empty());
        assert_eq!(peers.configured_peers, vec![local_endpoint_id]);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn transport_empty_seed_list_stays_idle_without_error() {
        let testnet = Testnet::new(5).expect("testnet");
        let transport = IrohGossipTransport::bind_with_discovery(
            TransportNetworkConfig::loopback(),
            DhtDiscoveryOptions::with_client(dht_test_client(&testnet)),
        )
        .await
        .expect("transport");
        publish_endpoint_to_testnet(&transport.endpoint, &testnet).await;

        transport
            .configure_discovery(DiscoveryMode::SeededDht, false, Vec::new(), Vec::new())
            .await
            .expect("configure discovery");

        let discovery = transport.discovery().await.expect("discovery");
        let peers = transport.peers().await.expect("peers");

        assert_eq!(discovery.mode, DiscoveryMode::SeededDht);
        assert!(discovery.configured_seed_peer_ids.is_empty());
        assert!(discovery.bootstrap_seed_peer_ids.is_empty());
        assert!(discovery.last_discovery_error.is_none());
        assert_eq!(peers.peer_count, 0);
        assert!(peers.last_error.is_none());
    }

    #[tokio::test]
    async fn fake_transport_discovery_reports_seed_sources_separately() {
        let transport = FakeTransport::new("local-peer", FakeNetwork::default());

        transport
            .configure_discovery(
                DiscoveryMode::StaticPeer,
                false,
                vec![SeedPeer {
                    endpoint_id: "configured-peer".into(),
                    addr_hint: None,
                }],
                vec![SeedPeer {
                    endpoint_id: "bootstrap-peer".into(),
                    addr_hint: None,
                }],
            )
            .await
            .expect("configure discovery");
        transport
            .import_ticket("manual-ticket-peer")
            .await
            .expect("import ticket");

        let discovery = transport.discovery().await.expect("discovery");

        assert_eq!(
            discovery.configured_seed_peer_ids,
            vec!["configured-peer".to_string()]
        );
        assert_eq!(
            discovery.bootstrap_seed_peer_ids,
            vec!["bootstrap-peer".to_string()]
        );
        assert_eq!(
            discovery.manual_ticket_peer_ids,
            vec!["manual-ticket-peer".to_string()]
        );
        assert_eq!(
            discovery.connected_peer_ids,
            vec![
                "manual-ticket-peer".to_string(),
                "configured-peer".to_string(),
                "bootstrap-peer".to_string(),
            ]
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn topic_hint_peer_count_tracks_real_subscribers() {
        let transport_a = IrohGossipTransport::bind_local()
            .await
            .expect("transport a");
        let transport_b = IrohGossipTransport::bind_local()
            .await
            .expect("transport b");
        let ticket_a = transport_a
            .export_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = transport_b
            .export_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        transport_a
            .import_ticket(&ticket_b)
            .await
            .expect("import b");
        transport_b
            .import_ticket(&ticket_a)
            .await
            .expect("import a");

        let demo = TopicId::new("kukuri:topic:demo");
        let test7 = TopicId::new("kukuri:topic:test7");
        let _ = transport_a
            .subscribe_hints(&demo)
            .await
            .expect("subscribe demo a");
        let _ = transport_b
            .subscribe_hints(&demo)
            .await
            .expect("subscribe demo b");
        let _ = transport_a
            .subscribe_hints(&test7)
            .await
            .expect("subscribe test7 a");

        timeout(Duration::from_secs(10), async {
            loop {
                let peers_a = transport_a.peers().await.expect("peers a");
                let demo_diag = peers_a
                    .topic_diagnostics
                    .iter()
                    .find(|topic| topic.topic == "hint/kukuri:topic:demo")
                    .expect("demo diag");
                let test7_diag = peers_a
                    .topic_diagnostics
                    .iter()
                    .find(|topic| topic.topic == "hint/kukuri:topic:test7")
                    .expect("test7 diag");
                if demo_diag.peer_count == 1 && test7_diag.peer_count == 0 {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("initial peer counts timeout");

        let _ = transport_b
            .subscribe_hints(&test7)
            .await
            .expect("subscribe test7 b");
        timeout(Duration::from_secs(10), async {
            loop {
                let peers_a = transport_a.peers().await.expect("peers a");
                let test7_diag = peers_a
                    .topic_diagnostics
                    .iter()
                    .find(|topic| topic.topic == "hint/kukuri:topic:test7")
                    .expect("test7 diag");
                if test7_diag.peer_count == 1 {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("join peer count timeout");

        transport_b
            .unsubscribe_hints(&test7)
            .await
            .expect("unsubscribe test7 b");
        timeout(Duration::from_secs(10), async {
            loop {
                let peers_a = transport_a.peers().await.expect("peers a");
                let test7_diag = peers_a
                    .topic_diagnostics
                    .iter()
                    .find(|topic| topic.topic == "hint/kukuri:topic:test7")
                    .expect("test7 diag");
                if test7_diag.peer_count == 0 {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("leave peer count timeout");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn topic_hint_late_subscriber_eventually_clears_missing_peer_ids_over_relay() {
        let (_relay_map, relay_url, _guard) = iroh::test_utils::run_relay_server()
            .await
            .expect("relay server");
        let relay_config = TransportRelayConfig {
            iroh_relay_urls: vec![relay_url.to_string()],
        }
        .normalized();
        let network_config = TransportNetworkConfig::loopback();
        let transport_a = IrohGossipTransport::bind_with_options(
            network_config.clone(),
            DhtDiscoveryOptions::disabled(),
            relay_config.clone(),
        )
        .await
        .expect("transport a");
        let transport_b = IrohGossipTransport::bind_with_options(
            network_config.clone(),
            DhtDiscoveryOptions::disabled(),
            relay_config.clone(),
        )
        .await
        .expect("transport b");
        let transport_c = IrohGossipTransport::bind_with_options(
            network_config,
            DhtDiscoveryOptions::disabled(),
            relay_config,
        )
        .await
        .expect("transport c");

        let discovery_a = transport_a.discovery().await.expect("discovery a");
        let discovery_b = transport_b.discovery().await.expect("discovery b");
        let discovery_c = transport_c.discovery().await.expect("discovery c");

        transport_a
            .configure_discovery(
                DiscoveryMode::StaticPeer,
                false,
                Vec::new(),
                vec![
                    SeedPeer {
                        endpoint_id: discovery_b.local_endpoint_id.clone(),
                        addr_hint: None,
                    },
                    SeedPeer {
                        endpoint_id: discovery_c.local_endpoint_id.clone(),
                        addr_hint: None,
                    },
                ],
            )
            .await
            .expect("configure a");
        transport_b
            .configure_discovery(
                DiscoveryMode::StaticPeer,
                false,
                Vec::new(),
                vec![
                    SeedPeer {
                        endpoint_id: discovery_a.local_endpoint_id.clone(),
                        addr_hint: None,
                    },
                    SeedPeer {
                        endpoint_id: discovery_c.local_endpoint_id.clone(),
                        addr_hint: None,
                    },
                ],
            )
            .await
            .expect("configure b");
        transport_c
            .configure_discovery(
                DiscoveryMode::StaticPeer,
                false,
                Vec::new(),
                vec![
                    SeedPeer {
                        endpoint_id: discovery_a.local_endpoint_id.clone(),
                        addr_hint: None,
                    },
                    SeedPeer {
                        endpoint_id: discovery_b.local_endpoint_id.clone(),
                        addr_hint: None,
                    },
                ],
            )
            .await
            .expect("configure c");

        let topic = TopicId::new("kukuri:topic:late-peer");
        let _stream_a = transport_a
            .subscribe_hints(&topic)
            .await
            .expect("subscribe a");
        let _stream_c = transport_c
            .subscribe_hints(&topic)
            .await
            .expect("subscribe c");

        timeout(Duration::from_secs(10), async {
            loop {
                let peers_c = transport_c.peers().await.expect("peers c before b");
                let diag_c = peers_c
                    .topic_diagnostics
                    .iter()
                    .find(|topic| topic.topic == "hint/kukuri:topic:late-peer")
                    .expect("diag c before b");
                if diag_c.missing_peer_ids.len() == 1 {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("initial partial join timeout");

        let _stream_b = transport_b
            .subscribe_hints(&topic)
            .await
            .expect("subscribe b");

        timeout(Duration::from_secs(10), async {
            loop {
                let peers_c = transport_c.peers().await.expect("peers c after b");
                let diag_c = peers_c
                    .topic_diagnostics
                    .iter()
                    .find(|topic| topic.topic == "hint/kukuri:topic:late-peer")
                    .expect("diag c after b");
                if diag_c.missing_peer_ids.is_empty() {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("late subscriber should clear missing peer ids");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn gossip_low_level_roundtrip_baseline() {
        let endpoint_a = Endpoint::empty_builder(RelayMode::Disabled)
            .bind_addr(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
            .expect("bind addr a")
            .bind()
            .await
            .expect("endpoint a");
        let gossip_a = Gossip::builder().spawn(endpoint_a.clone());
        let _router_a = Router::builder(endpoint_a.clone())
            .accept(GOSSIP_ALPN, gossip_a.clone())
            .spawn();

        let endpoint_b = Endpoint::empty_builder(RelayMode::Disabled)
            .bind_addr(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
            .expect("bind addr b")
            .bind()
            .await
            .expect("endpoint b");
        let gossip_b = Gossip::builder().spawn(endpoint_b.clone());
        let _router_b = Router::builder(endpoint_b.clone())
            .accept(GOSSIP_ALPN, gossip_b.clone())
            .spawn();

        let discovery = MemoryLookup::new();
        discovery.add_endpoint_info(endpoint_a.addr());
        discovery.add_endpoint_info(endpoint_b.addr());
        endpoint_a.address_lookup().add(discovery.clone());
        endpoint_b.address_lookup().add(discovery);

        let topic = topic_to_gossip_id(&TopicId::new("kukuri:topic:baseline"));
        let peer_a = endpoint_a.id();
        let peer_b = endpoint_b.id();
        let topic_a = gossip_a
            .subscribe(topic, vec![peer_b])
            .await
            .expect("subscribe a");
        let (sender_a, mut receiver_a) = topic_a.split();
        let topic_b = gossip_b
            .subscribe(topic, vec![peer_a])
            .await
            .expect("subscribe b");
        let (_sender_b, mut receiver_b) = topic_b.split();

        timeout(Duration::from_secs(10), receiver_a.joined())
            .await
            .expect("join a timeout")
            .expect("join a");
        timeout(Duration::from_secs(10), receiver_b.joined())
            .await
            .expect("join b timeout")
            .expect("join b");

        let event = build_post_envelope(
            &generate_keys(),
            &TopicId::new("kukuri:topic:baseline"),
            "hello baseline",
            None,
        )
        .expect("event");
        sender_a
            .broadcast(serde_json::to_vec(&event).expect("serialize").into())
            .await
            .expect("broadcast");

        let received = timeout(Duration::from_secs(10), async {
            while let Some(message) = receiver_b.next().await {
                match message.expect("gossip event") {
                    GossipEvent::Received(message) => {
                        let parsed: KukuriEnvelope =
                            serde_json::from_slice(&message.content).expect("parse event");
                        return parsed;
                    }
                    GossipEvent::Lagged => continue,
                    _ => {}
                }
            }
            panic!("receiver b closed");
        })
        .await
        .expect("receive timeout");

        assert_eq!(received.id, event.id);
        assert_eq!(
            received
                .post_content()
                .expect("post content")
                .expect("post content")
                .payload_ref,
            kukuri_core::PayloadRef::InlineText {
                text: "hello baseline".into(),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn transport_static_peer_can_connect_endpoint() {
        let transport_a = IrohGossipTransport::bind_local()
            .await
            .expect("transport a");
        let transport_b = IrohGossipTransport::bind_local()
            .await
            .expect("transport b");
        let ticket_b = transport_b
            .export_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");

        transport_a
            .import_ticket(&ticket_b)
            .await
            .expect("import b");
        let addr_b = parse_endpoint_ticket(&ticket_b).expect("parse ticket b");
        timeout(
            Duration::from_secs(5),
            transport_a.endpoint.connect(addr_b, GOSSIP_ALPN),
        )
        .await
        .expect("connect timeout")
        .expect("connect");
    }

    #[tokio::test]
    async fn fake_transport_hint_roundtrip() {
        let network = FakeNetwork::default();
        let left = FakeTransport::new("left", network.clone());
        let right = FakeTransport::new("right", network);
        let topic = TopicId::new("kukuri:topic:fake");
        let _left_stream = left
            .subscribe_hints(&topic)
            .await
            .expect("left subscribe hints");
        let mut right_stream = right
            .subscribe_hints(&topic)
            .await
            .expect("right subscribe hints");

        left.import_ticket("right").await.expect("import");
        let hint = GossipHint::Presence {
            topic_id: topic.clone(),
            author: "author-1".into(),
            ttl_ms: 30_000,
        };
        left.publish_hint(&topic, hint.clone())
            .await
            .expect("publish hint");

        let received = timeout(Duration::from_secs(1), right_stream.next())
            .await
            .expect("receive timeout")
            .expect("event");
        assert_eq!(received.hint, hint);
    }

    #[test]
    fn ticket_roundtrip() {
        let ticket =
            "f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0@127.0.0.1:4444";
        let parsed = parse_endpoint_ticket(ticket).expect("ticket must parse");
        assert_eq!(
            parsed.id.to_string(),
            "f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0"
        );
        assert_eq!(
            parsed.ip_addrs().next().copied(),
            Some("127.0.0.1:4444".parse().expect("socket addr"))
        );
    }

    #[test]
    fn encode_ticket_prefers_explicit_advertised_host() {
        let endpoint_id = EndpointId::from_str(
            "f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0",
        )
        .expect("endpoint id");
        let endpoint_addr = EndpointAddr::new(endpoint_id)
            .with_ip_addr("0.0.0.0:40123".parse().expect("socket addr"));
        let config = TransportNetworkConfig {
            bind_addr: "0.0.0.0:40123".parse().expect("bind addr"),
            advertised_host: Some("192.168.10.5".into()),
            advertised_port: Some(40123),
        };

        let ticket = encode_endpoint_ticket(&endpoint_addr, &config).expect("ticket");
        assert_eq!(
            ticket,
            "f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0@192.168.10.5:40123"
        );
    }

    #[test]
    fn encode_ticket_ignores_zero_port_from_endpoint_addr() {
        let endpoint_id = EndpointId::from_str(
            "f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0",
        )
        .expect("endpoint id");
        let endpoint_addr =
            EndpointAddr::new(endpoint_id).with_ip_addr("0.0.0.0:0".parse().expect("socket addr"));
        let config = TransportNetworkConfig {
            bind_addr: "127.0.0.1:40123".parse().expect("bind addr"),
            advertised_host: None,
            advertised_port: None,
        };

        let ticket = encode_endpoint_ticket(&endpoint_addr, &config).expect("ticket");

        assert_eq!(
            ticket,
            "f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0@127.0.0.1:40123"
        );
    }

    #[test]
    fn ticket_network_config_uses_bound_loopback_socket_for_port_zero_bind() {
        let endpoint_id = EndpointId::from_str(
            "f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0",
        )
        .expect("endpoint id");
        let endpoint_addr =
            EndpointAddr::new(endpoint_id).with_ip_addr("0.0.0.0:0".parse().expect("socket addr"));
        let config = TransportNetworkConfig::loopback();

        let resolved = ticket_network_config(
            &endpoint_addr,
            &["127.0.0.1:40123".parse().expect("bound socket")],
            &config,
        );

        assert_eq!(resolved.advertised_host.as_deref(), Some("127.0.0.1"));
        assert_eq!(resolved.advertised_port, Some(40123));
    }

    #[test]
    fn parse_ticket_resolves_localhost_hostname() {
        let parsed = parse_endpoint_ticket(
            "f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0@localhost:40123",
        )
        .expect("ticket");
        assert_eq!(
            parsed.ip_addrs().next().copied(),
            Some("127.0.0.1:40123".parse().expect("socket addr"))
        );
    }
}
