use std::collections::{BTreeMap, BTreeSet, HashMap};
#[cfg(not(test))]
use std::net::SocketAddr;
#[cfg(test)]
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
#[cfg(test)]
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock as StdRwLock};
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use chrono::Utc;
use futures_util::StreamExt;
#[cfg(test)]
use iroh::RelayMode;
use iroh::address_lookup::{
    AddrFilter, AddressLookup, DhtAddressLookup, EndpointInfo, Item as AddressLookupItem,
    MemoryLookup,
};
use iroh::endpoint::{Builder as EndpointBuilder, MtuDiscoveryConfig, QuicTransportConfig};
use iroh::protocol::Router;
#[cfg(test)]
use iroh::tls::CaRootsConfig;
use iroh::{Endpoint, EndpointAddr, EndpointId, RelayConfig, RelayUrl, SecretKey};
use iroh_gossip::api::{Event as GossipEvent, GossipSender};
use iroh_gossip::{ALPN as GOSSIP_ALPN, Gossip, TopicId as GossipTopicId};
use kukuri_core::{GossipHint, TopicId};
#[cfg(test)]
use kukuri_core::{HintObjectRef, KukuriEnvelope, build_post_envelope, generate_keys};
#[cfg(test)]
use pkarr::Client as PkarrClient;
#[cfg(test)]
use pkarr::Timestamp;
use tokio::sync::{Mutex, Notify, RwLock, broadcast};
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout};
use tokio_stream::wrappers::BroadcastStream;

use crate::config::{
    ConnectMode, DhtDiscoveryOptions, DiscoveryMode, DiscoverySnapshot, SeedPeer,
    TransportNetworkConfig, TransportRelayConfig,
};
use crate::diagnostics::{peer_status_detail, topic_status_detail};
#[cfg(test)]
use crate::discovery::build_signed_packet_with_timestamp;
use crate::discovery::prepare_endpoint_for_discovery;
use crate::tickets::{
    encode_endpoint_ticket, endpoint_addr_with_relays, parse_endpoint_ticket, ticket_network_config,
};
use crate::traits::{
    HintEnvelope, HintStream, HintTransport, PeerSnapshot, TopicPeerSnapshot, Transport,
};

fn initial_topic_join_timeout() -> Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        Duration::from_secs(180)
    } else {
        Duration::from_secs(15)
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
    relay_urls: Arc<StdRwLock<Vec<RelayUrl>>>,
}

impl RelayFallbackLookup {
    fn new(relay_urls: Arc<StdRwLock<Vec<RelayUrl>>>) -> Self {
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
        let relay_urls = self
            .relay_urls
            .read()
            .expect("relay fallback lookup poisoned")
            .clone();
        if relay_urls.is_empty() {
            return None;
        }
        let endpoint_info = EndpointInfo::from(endpoint_addr_with_relays(endpoint_id, &relay_urls));
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
    relay_urls: Arc<StdRwLock<Vec<RelayUrl>>>,
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
        let relay_urls = Arc::new(StdRwLock::new(relay_config.parsed_relay_urls()?));
        let (endpoint, discovery, publish_task) = bind_endpoint_with_options(
            network_config.bind_addr,
            &dht_options,
            &relay_config,
            Arc::clone(&relay_urls),
            None,
        )
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
            relay_urls,
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
        let relay_urls = Arc::new(StdRwLock::new(relay_config.parsed_relay_urls()?));
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
            relay_urls,
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
        *self
            .relay_urls
            .write()
            .expect("transport relay urls poisoned") = relay_urls;
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
            topics.get(topic.as_str()).map(|state| {
                (
                    state.broadcaster.clone(),
                    state.bootstrap_peer_ids.clone(),
                    Arc::clone(&state.neighbors),
                    Arc::clone(&state.last_error),
                )
            })
        };

        if let Some((broadcaster, existing_bootstrap_peer_ids, neighbors, last_error)) = existing {
            let has_neighbors = !neighbors.read().await.is_empty();
            let timed_out_join = last_error
                .lock()
                .await
                .as_deref()
                .is_some_and(|message| message.contains("initial topic join"));
            if existing_bootstrap_peer_ids == bootstrap_peer_ids
                && (!timed_out_join || has_neighbors)
            {
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
        let warm_endpoint = self.endpoint.clone();
        let warm_bootstrap_peers = bootstrap_peers.clone();
        let warm_gossip = self.gossip.clone();

        let task = tokio::spawn(async move {
            if imported_count > 0 {
                let join_timeout = initial_topic_join_timeout();
                let warmup_task = tokio::spawn(async move {
                    let join_deadline = tokio::time::Instant::now() + join_timeout;
                    loop {
                        for peer in &warm_bootstrap_peers {
                            if let Ok(connection) =
                                warm_endpoint.connect(peer.clone(), GOSSIP_ALPN).await
                            {
                                let _ = warm_gossip.handle_connection(connection).await;
                            }
                        }
                        if tokio::time::Instant::now() >= join_deadline {
                            return;
                        }
                        sleep(Duration::from_millis(100)).await;
                    }
                });
                let joined = timeout(join_timeout, receiver.joined())
                    .await
                    .is_ok_and(|result| result.is_ok());
                warmup_task.abort();
                if joined {
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
                        joined_task_state.store(true, Ordering::SeqCst);
                        joined_task_notify.notify_waiters();
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
                                source_peer: message.delivered_from.to_string(),
                            });
                        } else {
                            *last_error_task.lock().await =
                                Some("failed to decode hint payload".to_string());
                        }
                    }
                    Ok(GossipEvent::NeighborUp(peer_id)) => {
                        joined_task_state.store(true, Ordering::SeqCst);
                        joined_task_notify.notify_waiters();
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

pub(crate) async fn bind_endpoint_with_options(
    bind_addr: SocketAddr,
    dht_options: &DhtDiscoveryOptions,
    relay_config: &TransportRelayConfig,
    relay_urls: Arc<StdRwLock<Vec<RelayUrl>>>,
    secret_key: Option<SecretKey>,
) -> Result<(Endpoint, Arc<MemoryLookup>, Option<JoinHandle<()>>)> {
    let discovery = Arc::new(MemoryLookup::new());
    let mut builder = build_endpoint_builder(
        Endpoint::empty_builder().relay_mode(relay_config.relay_mode()?),
        &discovery,
        Some(dht_options),
        relay_urls,
    )?;
    if let Some(secret_key) = secret_key {
        builder = builder.secret_key(secret_key);
    }
    #[cfg(test)]
    {
        builder = builder.ca_roots_config(CaRootsConfig::insecure_skip_verify());
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
        let relay_urls = self
            .relay_urls
            .read()
            .expect("transport relay urls poisoned")
            .clone();
        let mut configured = BTreeMap::new();
        for seed in configured_seed_peers {
            let endpoint_addr = seed.to_endpoint_addr_with_relays(&relay_urls)?;
            self.discovery.add_endpoint_info(endpoint_addr.clone());
            configured.insert(endpoint_addr.id.to_string(), endpoint_addr);
        }
        let mut bootstrap = BTreeMap::new();
        for seed in bootstrap_seed_peers {
            let endpoint_addr = seed.to_endpoint_addr_with_relays(&relay_urls)?;
            self.discovery.add_endpoint_info(endpoint_addr.clone());
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

fn relay_backed_windows_transport_config(relay_urls: &[RelayUrl]) -> Option<QuicTransportConfig> {
    if !cfg!(target_os = "windows") || relay_urls.is_empty() {
        return None;
    }
    Some(
        QuicTransportConfig::builder()
            .enable_segmentation_offload(false)
            .initial_mtu(1200)
            .min_mtu(1200)
            .mtu_discovery_config(None::<MtuDiscoveryConfig>)
            .send_observed_address_reports(false)
            .receive_observed_address_reports(false)
            .build(),
    )
}

pub fn build_endpoint_builder(
    builder: EndpointBuilder,
    discovery: &Arc<MemoryLookup>,
    dht_options: Option<&DhtDiscoveryOptions>,
    relay_urls: Arc<StdRwLock<Vec<RelayUrl>>>,
) -> Result<EndpointBuilder> {
    let mut builder = builder.address_lookup(discovery.clone());
    let relay_urls_snapshot = relay_urls
        .read()
        .expect("relay transport config poisoned")
        .clone();
    if let Some(transport_config) = relay_backed_windows_transport_config(&relay_urls_snapshot) {
        builder = builder.transport_config(transport_config);
    }
    builder = builder.address_lookup(RelayFallbackLookup::new(relay_urls));
    if let Some(dht_options) = dht_options.filter(|options| options.enabled) {
        let mut dht_builder = DhtAddressLookup::builder()
            .addr_filter(AddrFilter::unfiltered())
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

#[cfg(test)]
mod tests {
    use super::*;

    use pkarr::errors::{ConcurrencyError, PublishError};
    use pkarr::mainline::Testnet;

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

    struct HintRoundtripParticipant<'a, T> {
        transport: &'a T,
        stream: &'a mut HintStream,
        expected_source_peer: Option<&'a str>,
    }

    async fn wait_for_hint_roundtrip<T>(
        participant_a: HintRoundtripParticipant<'_, T>,
        participant_b: HintRoundtripParticipant<'_, T>,
        topic: &TopicId,
        step_timeout: Duration,
        label: &str,
    ) where
        T: Transport + HintTransport + Sync,
    {
        let hint_from_a = GossipHint::TopicObjectsChanged {
            topic_id: topic.clone(),
            objects: vec![HintObjectRef {
                object_id: format!("{label}-from-a"),
                object_kind: "post".into(),
            }],
        };
        let hint_from_b = GossipHint::TopicObjectsChanged {
            topic_id: topic.clone(),
            objects: vec![HintObjectRef {
                object_id: format!("{label}-from-b"),
                object_kind: "post".into(),
            }],
        };
        match timeout(step_timeout, async {
            let mut received_on_a = false;
            let mut received_on_b = false;
            loop {
                if !received_on_a {
                    participant_b
                        .transport
                        .publish_hint(topic, hint_from_b.clone())
                        .await
                        .expect("publish hint from b");
                }
                if !received_on_b {
                    participant_a
                        .transport
                        .publish_hint(topic, hint_from_a.clone())
                        .await
                        .expect("publish hint from a");
                }
                if !received_on_a
                    && let Ok(Some(envelope)) =
                        timeout(Duration::from_millis(500), participant_a.stream.next()).await
                {
                    received_on_a = envelope.hint == hint_from_b
                        && participant_b
                            .expected_source_peer
                            .is_none_or(|peer_id| envelope.source_peer == peer_id);
                }
                if !received_on_b
                    && let Ok(Some(envelope)) =
                        timeout(Duration::from_millis(500), participant_b.stream.next()).await
                {
                    received_on_b = envelope.hint == hint_from_a
                        && participant_a
                            .expected_source_peer
                            .is_none_or(|peer_id| envelope.source_peer == peer_id);
                }
                if received_on_a && received_on_b {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        {
            Ok(()) => {}
            Err(_) => {
                let peers_a = participant_a.transport.peers().await.expect("peers a");
                let peers_b = participant_b.transport.peers().await.expect("peers b");
                panic!(
                    "{label} hint roundtrip timeout: a={} b={}",
                    format_peer_snapshot(&peers_a),
                    format_peer_snapshot(&peers_b)
                );
            }
        }
    }

    fn format_peer_snapshot(snapshot: &PeerSnapshot) -> String {
        let topics = snapshot
            .topic_diagnostics
            .iter()
            .map(|topic| {
                format!(
                    "{}: joined={}, peer_count={}, connected_peers={:?}, missing_peer_ids={:?}, status_detail={}, last_error={:?}",
                    topic.topic,
                    topic.joined,
                    topic.peer_count,
                    topic.connected_peers,
                    topic.missing_peer_ids,
                    topic.status_detail,
                    topic.last_error
                )
            })
            .collect::<Vec<_>>();
        format!(
            "connected={}, peer_count={}, connected_peers={:?}, configured_peers={:?}, status_detail={}, last_error={:?}, topics={topics:?}",
            snapshot.connected,
            snapshot.peer_count,
            snapshot.connected_peers,
            snapshot.configured_peers,
            snapshot.status_detail,
            snapshot.last_error
        )
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn transport_two_process_hint_roundtrip_static_peer() {
        if std::env::var_os("GITHUB_ACTIONS").is_some() {
            return;
        }
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
        let join_timeout = initial_topic_join_timeout();
        let peer_id_a = transport_a.endpoint.id().to_string();
        let peer_id_b = transport_b.endpoint.id().to_string();
        let (mut stream_a, mut stream_b) = tokio::try_join!(
            transport_a.subscribe_hints(&topic),
            transport_b.subscribe_hints(&topic)
        )
        .expect("subscribe both");
        wait_for_hint_roundtrip(
            HintRoundtripParticipant {
                transport: &transport_a,
                stream: &mut stream_a,
                expected_source_peer: Some(peer_id_a.as_str()),
            },
            HintRoundtripParticipant {
                transport: &transport_b,
                stream: &mut stream_b,
                expected_source_peer: Some(peer_id_b.as_str()),
            },
            &topic,
            join_timeout,
            "static-peer",
        )
        .await;

        match timeout(join_timeout, async {
            loop {
                let peers_a = transport_a.peers().await.expect("peers a");
                let peers_b = transport_b.peers().await.expect("peers b");
                let diag_a = peers_a
                    .topic_diagnostics
                    .iter()
                    .find(|topic| topic.topic == "hint/kukuri:topic:transport");
                let diag_b = peers_b
                    .topic_diagnostics
                    .iter()
                    .find(|topic| topic.topic == "hint/kukuri:topic:transport");
                if peers_a.peer_count >= 1
                    && peers_b.peer_count >= 1
                    && diag_a.is_some_and(|topic| topic.peer_count >= 1)
                    && diag_b.is_some_and(|topic| topic.peer_count >= 1)
                {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        {
            Ok(()) => {}
            Err(_) => {
                let peers_a = transport_a.peers().await.expect("peers a");
                let peers_b = transport_b.peers().await.expect("peers b");
                panic!(
                    "peer snapshot timeout: a={} b={}",
                    format_peer_snapshot(&peers_a),
                    format_peer_snapshot(&peers_b)
                );
            }
        }
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
        let join_timeout = initial_topic_join_timeout();
        let _stream_a = transport_a
            .subscribe_hints(&topic)
            .await
            .expect("subscribe a");
        let mut stream_c = transport_c
            .subscribe_hints(&topic)
            .await
            .expect("subscribe c");

        timeout(join_timeout, async {
            loop {
                let peers_c = transport_c.peers().await.expect("peers c before b");
                let diag_c = peers_c
                    .topic_diagnostics
                    .iter()
                    .find(|topic| topic.topic == "hint/kukuri:topic:late-peer")
                    .expect("diag c before b");
                if diag_c
                    .missing_peer_ids
                    .iter()
                    .any(|peer_id| peer_id == &discovery_b.local_endpoint_id)
                {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("initial partial join timeout");

        let mut stream_b = transport_b
            .subscribe_hints(&topic)
            .await
            .expect("subscribe b");
        wait_for_hint_roundtrip(
            HintRoundtripParticipant {
                transport: &transport_b,
                stream: &mut stream_b,
                expected_source_peer: None,
            },
            HintRoundtripParticipant {
                transport: &transport_c,
                stream: &mut stream_c,
                expected_source_peer: None,
            },
            &topic,
            join_timeout,
            "late-subscriber",
        )
        .await;

        match timeout(join_timeout, async {
            loop {
                let peers_c = transport_c.peers().await.expect("peers c after b");
                let diag_c = peers_c
                    .topic_diagnostics
                    .iter()
                    .find(|topic| topic.topic == "hint/kukuri:topic:late-peer")
                    .expect("diag c after b");
                if !diag_c
                    .missing_peer_ids
                    .iter()
                    .any(|peer_id| peer_id == &discovery_b.local_endpoint_id)
                {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        {
            Ok(()) => {}
            Err(_) => {
                let peers_b = transport_b.peers().await.expect("peers b after timeout");
                let peers_c = transport_c.peers().await.expect("peers c after timeout");
                panic!(
                    "late subscriber should clear missing peer ids: b={} c={}",
                    format_peer_snapshot(&peers_b),
                    format_peer_snapshot(&peers_c)
                );
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn gossip_low_level_roundtrip_baseline() {
        let endpoint_a = Endpoint::empty_builder()
            .relay_mode(RelayMode::Disabled)
            .bind_addr(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
            .expect("bind addr a")
            .bind()
            .await
            .expect("endpoint a");
        let gossip_a = Gossip::builder().spawn(endpoint_a.clone());
        let _router_a = Router::builder(endpoint_a.clone())
            .accept(GOSSIP_ALPN, gossip_a.clone())
            .spawn();

        let endpoint_b = Endpoint::empty_builder()
            .relay_mode(RelayMode::Disabled)
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
        endpoint_a
            .address_lookup()
            .expect("address lookup a")
            .add(discovery.clone());
        endpoint_b
            .address_lookup()
            .expect("address lookup b")
            .add(discovery);

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
        endpoint_a.close().await;
        endpoint_b.close().await;
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
}
