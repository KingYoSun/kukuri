use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
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
        Ok(PeerSnapshot {
            connected: !imported.is_empty(),
            peer_count: imported.len(),
            connected_peers: imported,
            subscribed_topics: topics,
            pending_events: 0,
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
    _receiver_task: JoinHandle<()>,
}

#[derive(Clone)]
pub struct IrohGossipTransport {
    endpoint: Endpoint,
    gossip: Gossip,
    _router: Router,
    discovery: Arc<MemoryLookup>,
    imported_peers: Arc<Mutex<BTreeMap<String, EndpointAddr>>>,
    connected_peers: Arc<RwLock<BTreeSet<String>>>,
    subscribed_topics: Arc<Mutex<BTreeSet<String>>>,
    topic_states: Arc<Mutex<HashMap<String, TopicState>>>,
}

impl IrohGossipTransport {
    pub async fn bind_local() -> Result<Self> {
        let discovery = Arc::new(MemoryLookup::new());
        let endpoint = Endpoint::empty_builder(RelayMode::Disabled)
            .address_lookup(discovery.clone())
            .bind_addr(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
            .context("failed to bind local endpoint address")?
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
            imported_peers: Arc::new(Mutex::new(BTreeMap::new())),
            connected_peers: Arc::new(RwLock::new(BTreeSet::new())),
            subscribed_topics: Arc::new(Mutex::new(BTreeSet::new())),
            topic_states: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    async fn ensure_topic(&self, topic: &TopicId) -> Result<broadcast::Sender<EventEnvelope>> {
        if let Some(existing) = self.topic_states.lock().await.get(topic.as_str()) {
            self.subscribed_topics.lock().await.insert(topic.0.clone());
            return Ok(existing.broadcaster.clone());
        }

        let imported = self
            .imported_peers
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
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
        let peers = Arc::clone(&self.connected_peers);
        let joined = Arc::new(AtomicBool::new(imported.is_empty()));
        let joined_notify = Arc::new(Notify::new());
        let joined_task_state = Arc::clone(&joined);
        let joined_task_notify = Arc::clone(&joined_notify);
        let imported_count = imported.len();

        let task = tokio::spawn(async move {
            if imported_count > 0
                && timeout(Duration::from_secs(15), receiver.joined())
                    .await
                    .is_ok_and(|result| result.is_ok())
            {
                joined_task_state.store(true, Ordering::SeqCst);
                joined_task_notify.notify_waiters();
            }
            while let Some(event) = receiver.next().await {
                match event {
                    Ok(GossipEvent::Received(message)) => {
                        if let Ok(parsed) = serde_json::from_slice::<Event>(&message.content) {
                            let _ = outbound.send(EventEnvelope {
                                event: parsed,
                                received_at: Utc::now().timestamp_millis(),
                                source_peer: String::new(),
                            });
                        }
                    }
                    Ok(GossipEvent::NeighborUp(peer_id)) => {
                        peers.write().await.insert(peer_id.to_string());
                    }
                    Ok(GossipEvent::NeighborDown(peer_id)) => {
                        peers.write().await.remove(peer_id.to_string().as_str());
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
        let connected = self
            .connected_peers
            .read()
            .await
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let subscribed_topics = self
            .subscribed_topics
            .lock()
            .await
            .iter()
            .cloned()
            .collect::<Vec<_>>();

        Ok(PeerSnapshot {
            connected: !connected.is_empty(),
            peer_count: connected.len(),
            connected_peers: connected,
            subscribed_topics,
            pending_events: 0,
        })
    }

    async fn export_ticket(&self) -> Result<Option<String>> {
        Ok(Some(encode_ticket(&self.endpoint.addr())?))
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

fn encode_ticket(endpoint_addr: &EndpointAddr) -> Result<String> {
    let socket_addr = endpoint_addr
        .ip_addrs()
        .next()
        .copied()
        .ok_or_else(|| anyhow!("endpoint does not expose a direct socket address"))?;
    Ok(format!("{}@{socket_addr}", endpoint_addr.id))
}

fn parse_ticket(ticket: &str) -> Result<EndpointAddr> {
    let (node_id, socket_addr) = ticket
        .split_once('@')
        .ok_or_else(|| anyhow!("ticket must be formatted as <node_id>@<host:port>"))?;
    let endpoint_id = EndpointId::from_str(node_id).context("invalid endpoint id")?;
    let socket_addr = SocketAddr::from_str(socket_addr).context("invalid socket address")?;
    Ok(EndpointAddr::new(endpoint_id).with_ip_addr(socket_addr))
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
}
