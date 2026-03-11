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
use iroh::address_lookup::MemoryLookup;
use iroh::endpoint::Builder as EndpointBuilder;
use iroh::protocol::Router;
use iroh::{Endpoint, EndpointAddr, EndpointId, RelayMode};
use iroh_gossip::api::{Event as GossipEvent, GossipSender};
use iroh_gossip::{ALPN as GOSSIP_ALPN, Gossip, TopicId as GossipTopicId};
use next_core::{Event, TopicId};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, Notify, RwLock, broadcast};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_stream::wrappers::BroadcastStream;

pub type EventStream = Pin<Box<dyn Stream<Item = EventEnvelope> + Send>>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub event: Event,
    pub received_at: i64,
    pub source_peer: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerSnapshot {
    pub connected: bool,
    pub peer_count: usize,
    pub connected_peers: Vec<String>,
    pub subscribed_topics: Vec<String>,
    pub pending_events: usize,
    pub topic_diagnostics: Vec<TopicPeerSnapshot>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicPeerSnapshot {
    pub topic: String,
    pub joined: bool,
    pub peer_count: usize,
    pub connected_peers: Vec<String>,
    pub last_received_at: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransportNetworkConfig {
    pub bind_addr: SocketAddr,
    pub advertised_host: Option<String>,
    pub advertised_port: Option<u16>,
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
        let bind_addr = std::env::var("KUKURI_NEXT_BIND_ADDR")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(|value| SocketAddr::from_str(value.trim()))
            .transpose()
            .context("failed to parse KUKURI_NEXT_BIND_ADDR")?
            .unwrap_or_else(|| SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)));
        let advertised_host = std::env::var("KUKURI_NEXT_ADVERTISE_HOST")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let advertised_port = std::env::var("KUKURI_NEXT_ADVERTISE_PORT")
            .ok()
            .map(|value| value.trim().parse::<u16>())
            .transpose()
            .context("failed to parse KUKURI_NEXT_ADVERTISE_PORT")?;

        Ok(Self {
            bind_addr,
            advertised_host,
            advertised_port,
        })
    }
}

#[async_trait]
pub trait Transport: Send + Sync {
    async fn subscribe(&self, topic: &TopicId) -> Result<EventStream>;
    async fn publish(&self, topic: &TopicId, event: Event) -> Result<()>;
    async fn peers(&self) -> Result<PeerSnapshot>;
    async fn export_ticket(&self) -> Result<Option<String>>;
    async fn import_ticket(&self, ticket: &str) -> Result<()>;
}

#[derive(Clone, Default)]
pub struct FakeNetwork {
    topics: Arc<Mutex<HashMap<String, broadcast::Sender<EventEnvelope>>>>,
    known_peers: Arc<Mutex<BTreeSet<String>>>,
}

#[derive(Clone)]
pub struct FakeTransport {
    local_id: String,
    network: FakeNetwork,
    imported_peers: Arc<Mutex<BTreeSet<String>>>,
    subscribed_topics: Arc<Mutex<BTreeSet<String>>>,
}

impl FakeTransport {
    pub fn new(local_id: impl Into<String>, network: FakeNetwork) -> Self {
        Self {
            local_id: local_id.into(),
            network,
            imported_peers: Arc::new(Mutex::new(BTreeSet::new())),
            subscribed_topics: Arc::new(Mutex::new(BTreeSet::new())),
        }
    }

    fn stream_from_sender(sender: &broadcast::Sender<EventEnvelope>) -> EventStream {
        let receiver = sender.subscribe();
        let stream = BroadcastStream::new(receiver).filter_map(|event| async move { event.ok() });
        Box::pin(stream)
    }

    async fn topic_sender(&self, topic: &TopicId) -> broadcast::Sender<EventEnvelope> {
        let mut topics = self.network.topics.lock().await;
        topics
            .entry(topic.0.clone())
            .or_insert_with(|| broadcast::channel(128).0)
            .clone()
    }
}

#[async_trait]
impl Transport for FakeTransport {
    async fn subscribe(&self, topic: &TopicId) -> Result<EventStream> {
        self.subscribed_topics.lock().await.insert(topic.0.clone());
        let sender = self.topic_sender(topic).await;
        Ok(Self::stream_from_sender(&sender))
    }

    async fn publish(&self, topic: &TopicId, event: Event) -> Result<()> {
        let sender = self.topic_sender(topic).await;
        let _ = sender.send(EventEnvelope {
            event,
            received_at: Utc::now().timestamp_millis(),
            source_peer: self.local_id.clone(),
        });
        Ok(())
    }

    async fn peers(&self) -> Result<PeerSnapshot> {
        let imported = self
            .imported_peers
            .lock()
            .await
            .iter()
            .cloned()
            .collect::<Vec<_>>();
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
                last_received_at: None,
            })
            .collect::<Vec<_>>();
        Ok(PeerSnapshot {
            connected: !imported.is_empty(),
            peer_count: imported.len(),
            connected_peers: imported,
            subscribed_topics: topics,
            pending_events: 0,
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
}

struct TopicState {
    sender: Arc<Mutex<GossipSender>>,
    broadcaster: broadcast::Sender<EventEnvelope>,
    joined: Arc<AtomicBool>,
    joined_notify: Arc<Notify>,
    bootstrap_peer_ids: BTreeSet<String>,
    neighbors: Arc<RwLock<BTreeSet<String>>>,
    last_received_at: Arc<Mutex<Option<i64>>>,
    _receiver_task: JoinHandle<()>,
}

#[derive(Clone)]
pub struct IrohGossipTransport {
    endpoint: Endpoint,
    gossip: Gossip,
    _router: Router,
    discovery: Arc<MemoryLookup>,
    network_config: TransportNetworkConfig,
    imported_peers: Arc<Mutex<BTreeMap<String, EndpointAddr>>>,
    subscribed_topics: Arc<Mutex<BTreeSet<String>>>,
    topic_states: Arc<Mutex<HashMap<String, TopicState>>>,
}

impl IrohGossipTransport {
    pub async fn bind(network_config: TransportNetworkConfig) -> Result<Self> {
        let discovery = Arc::new(MemoryLookup::new());
        let mut builder =
            Endpoint::empty_builder(RelayMode::Disabled).address_lookup(discovery.clone());
        builder = apply_bind(builder, network_config.bind_addr)?;
        let endpoint = builder
            .bind()
            .await
            .context("failed to bind iroh endpoint")?;
        discovery.add_endpoint_info(endpoint.addr());

        let gossip = Gossip::builder().spawn(endpoint.clone());
        let router = Router::builder(endpoint.clone())
            .accept(GOSSIP_ALPN, gossip.clone())
            .spawn();

        Ok(Self {
            endpoint,
            gossip,
            _router: router,
            discovery,
            network_config,
            imported_peers: Arc::new(Mutex::new(BTreeMap::new())),
            subscribed_topics: Arc::new(Mutex::new(BTreeSet::new())),
            topic_states: Arc::new(Mutex::new(HashMap::new())),
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
    }

    async fn ensure_topic(&self, topic: &TopicId) -> Result<broadcast::Sender<EventEnvelope>> {
        let imported = self
            .imported_peers
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let bootstrap_peer_ids = imported
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

        let bootstrap = imported.iter().map(|peer| peer.id).collect::<Vec<_>>();

        for peer in &imported {
            self.discovery.add_endpoint_info(peer.clone());
        }

        let topic_handle = self
            .gossip
            .subscribe(topic_to_gossip_id(topic), bootstrap)
            .await
            .context("failed to subscribe gossip topic")?;
        let (sender, mut receiver) = topic_handle.split();
        let (broadcaster, _) = broadcast::channel(256);
        let outbound = broadcaster.clone();
        let joined = Arc::new(AtomicBool::new(imported.is_empty()));
        let joined_notify = Arc::new(Notify::new());
        let joined_task_state = Arc::clone(&joined);
        let joined_task_notify = Arc::clone(&joined_notify);
        let neighbors = Arc::new(RwLock::new(BTreeSet::new()));
        let neighbors_task = Arc::clone(&neighbors);
        let last_received_at = Arc::new(Mutex::new(None));
        let last_received_at_task = Arc::clone(&last_received_at);
        let imported_count = imported.len();

        let task = tokio::spawn(async move {
            if imported_count > 0
                && timeout(Duration::from_secs(15), receiver.joined())
                    .await
                    .is_ok_and(|result| result.is_ok())
            {
                joined_task_state.store(true, Ordering::SeqCst);
                joined_task_notify.notify_waiters();
                let current_neighbors = receiver
                    .neighbors()
                    .map(|peer| peer.to_string())
                    .collect::<BTreeSet<_>>();
                *neighbors_task.write().await = current_neighbors;
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
                        if let Ok(parsed) = serde_json::from_slice::<Event>(&message.content) {
                            let _ = outbound.send(EventEnvelope {
                                event: parsed,
                                received_at: Utc::now().timestamp_millis(),
                                source_peer: String::new(),
                            });
                        }
                    }
                    Ok(GossipEvent::NeighborUp(peer_id)) => {
                        let mut guard = neighbors_task.write().await;
                        guard.insert(peer_id.to_string());
                    }
                    Ok(GossipEvent::NeighborDown(peer_id)) => {
                        let mut guard = neighbors_task.write().await;
                        guard.remove(peer_id.to_string().as_str());
                    }
                    Ok(GossipEvent::Lagged) => {}
                    Err(_) => break,
                }
            }
        });

        self.subscribed_topics.lock().await.insert(topic.0.clone());
        self.topic_states.lock().await.insert(
            topic.0.clone(),
            TopicState {
                sender: Arc::new(Mutex::new(sender)),
                broadcaster: broadcaster.clone(),
                joined,
                joined_notify,
                bootstrap_peer_ids,
                neighbors,
                last_received_at,
                _receiver_task: task,
            },
        );

        Ok(broadcaster)
    }

    fn stream_from_sender(sender: &broadcast::Sender<EventEnvelope>) -> EventStream {
        let stream =
            BroadcastStream::new(sender.subscribe()).filter_map(|event| async move { event.ok() });
        Box::pin(stream)
    }
}

#[async_trait]
impl Transport for IrohGossipTransport {
    async fn subscribe(&self, topic: &TopicId) -> Result<EventStream> {
        let sender = self.ensure_topic(topic).await?;
        Ok(Self::stream_from_sender(&sender))
    }

    async fn publish(&self, topic: &TopicId, event: Event) -> Result<()> {
        let _ = self.ensure_topic(topic).await?;
        let peer_ids = self
            .imported_peers
            .lock()
            .await
            .values()
            .map(|peer| peer.id)
            .collect::<Vec<_>>();
        let states = self.topic_states.lock().await;
        let state = states
            .get(topic.as_str())
            .ok_or_else(|| anyhow!("missing topic sender"))?;
        if !peer_ids.is_empty() && !state.joined.load(Ordering::SeqCst) {
            timeout(Duration::from_secs(10), state.joined_notify.notified())
                .await
                .context("timed out waiting for gossip topic join")?;
        }
        let payload = serde_json::to_vec(&event)?;
        let sender = state.sender.lock().await;
        sender
            .broadcast(payload.into())
            .await
            .context("failed to broadcast gossip event")?;
        Ok(())
    }

    async fn peers(&self) -> Result<PeerSnapshot> {
        let topic_states = self
            .topic_states
            .lock()
            .await
            .iter()
            .map(|(topic, state)| {
                (
                    topic.clone(),
                    Arc::clone(&state.neighbors),
                    Arc::clone(&state.last_received_at),
                )
            })
            .collect::<Vec<_>>();
        let mut connected = BTreeSet::new();
        let mut topic_diagnostics = Vec::with_capacity(topic_states.len());
        for (topic, neighbors, last_received_at) in topic_states {
            let peers = neighbors.read().await.iter().cloned().collect::<Vec<_>>();
            let last_received_at = *last_received_at.lock().await;
            for peer in &peers {
                connected.insert(peer.clone());
            }
            topic_diagnostics.push(TopicPeerSnapshot {
                topic,
                joined: !peers.is_empty(),
                peer_count: peers.len(),
                connected_peers: peers,
                last_received_at,
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

        Ok(PeerSnapshot {
            connected: !connected_peers.is_empty(),
            peer_count: connected_peers.len(),
            connected_peers,
            subscribed_topics,
            pending_events: 0,
            topic_diagnostics,
        })
    }

    async fn export_ticket(&self) -> Result<Option<String>> {
        Ok(Some(encode_ticket(
            &self.endpoint.addr(),
            &self.network_config,
        )?))
    }

    async fn import_ticket(&self, ticket: &str) -> Result<()> {
        let endpoint_addr = parse_ticket(ticket)?;
        self.discovery.add_endpoint_info(endpoint_addr.clone());
        self.imported_peers
            .lock()
            .await
            .insert(endpoint_addr.id.to_string(), endpoint_addr);
        Ok(())
    }
}

fn topic_to_gossip_id(topic: &TopicId) -> GossipTopicId {
    let hash = blake3::hash(topic.as_str().as_bytes());
    GossipTopicId::from_bytes(*hash.as_bytes())
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

fn encode_ticket(endpoint_addr: &EndpointAddr, config: &TransportNetworkConfig) -> Result<String> {
    let advertised_port = config
        .advertised_port
        .or_else(|| endpoint_addr.ip_addrs().next().map(|addr| addr.port()))
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

fn parse_ticket(ticket: &str) -> Result<EndpointAddr> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use next_core::{TopicId, build_text_note, generate_keys};
    use std::net::{Ipv4Addr, SocketAddrV4};

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn transport_two_process_roundtrip_static_peer() {
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
        let (_stream_a, mut stream_b) =
            tokio::try_join!(transport_a.subscribe(&topic), transport_b.subscribe(&topic))
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
        let event =
            build_text_note(&generate_keys(), &topic, "hello transport", None).expect("event");

        transport_a
            .publish(&topic, event.clone())
            .await
            .expect("publish");
        let envelope = timeout(Duration::from_secs(10), stream_b.next())
            .await
            .expect("receive timeout")
            .expect("stream event");

        assert_eq!(envelope.event.id, event.id);
        assert_eq!(envelope.event.content, "hello transport");
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

        let event = build_text_note(
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
                        let parsed: Event =
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
        assert_eq!(received.content, "hello baseline");
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
        let addr_b = parse_ticket(&ticket_b).expect("parse ticket b");
        timeout(
            Duration::from_secs(5),
            transport_a.endpoint.connect(addr_b, GOSSIP_ALPN),
        )
        .await
        .expect("connect timeout")
        .expect("connect");
    }

    #[tokio::test]
    async fn fake_transport_roundtrip() {
        let network = FakeNetwork::default();
        let left = FakeTransport::new("left", network.clone());
        let right = FakeTransport::new("right", network);
        let topic = TopicId::new("kukuri:topic:fake");
        let _left_stream = left.subscribe(&topic).await.expect("left subscribe");
        let mut right_stream = right.subscribe(&topic).await.expect("right subscribe");

        left.import_ticket("right").await.expect("import");
        let event = build_text_note(&generate_keys(), &topic, "hello fake", None).expect("event");
        left.publish(&topic, event.clone()).await.expect("publish");

        let received = timeout(Duration::from_secs(1), right_stream.next())
            .await
            .expect("receive timeout")
            .expect("event");
        assert_eq!(received.event.id, event.id);
    }

    #[test]
    fn ticket_roundtrip() {
        let ticket =
            "f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0@127.0.0.1:4444";
        let parsed = parse_ticket(ticket).expect("ticket must parse");
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

        let ticket = encode_ticket(&endpoint_addr, &config).expect("ticket");
        assert_eq!(
            ticket,
            "f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0@192.168.10.5:40123"
        );
    }

    #[test]
    fn parse_ticket_resolves_localhost_hostname() {
        let parsed = parse_ticket(
            "f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0@localhost:40123",
        )
        .expect("ticket");
        assert_eq!(
            parsed.ip_addrs().next().copied(),
            Some("127.0.0.1:40123".parse().expect("socket addr"))
        );
    }
}
