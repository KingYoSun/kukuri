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
use iroh::endpoint::{
    Builder as EndpointBuilder, MtuDiscoveryConfig, QuicTransportConfig, presets,
};
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
use tracing::{info, warn};

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

struct HintTopicState {
    sender: Arc<Mutex<GossipSender>>,
    broadcaster: broadcast::Sender<HintEnvelope>,
    bootstrap_peer_ids: BTreeSet<String>,
    neighbors: Arc<RwLock<BTreeSet<String>>>,
    last_received_at: Arc<Mutex<Option<i64>>>,
    last_error: Arc<Mutex<Option<String>>>,
    _receiver_task: JoinHandle<()>,
}

#[derive(Clone, Debug, Default)]
pub struct TransportPeerState {
    pub imported_peers: Vec<EndpointAddr>,
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

mod discovery;
mod endpoint;
mod peer_state;
mod relay;
#[cfg(test)]
mod tests;
mod topics;

#[cfg(test)]
pub(crate) use endpoint::bind_endpoint_with_options;
pub use relay::{build_endpoint_builder, sync_endpoint_relay_config};
#[cfg(test)]
pub(crate) use topics::{initial_topic_join_timeout, topic_to_gossip_id};

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
        self.transport_peers_impl().await
    }
    async fn export_ticket(&self) -> Result<Option<String>> {
        self.transport_export_ticket_impl().await
    }
    async fn import_ticket(&self, ticket: &str) -> Result<()> {
        self.transport_import_ticket_impl(ticket).await
    }
    async fn configure_discovery(
        &self,
        mode: DiscoveryMode,
        env_locked: bool,
        configured_seed_peers: Vec<SeedPeer>,
        bootstrap_seed_peers: Vec<SeedPeer>,
    ) -> Result<()> {
        self.transport_configure_discovery_impl(
            mode,
            env_locked,
            configured_seed_peers,
            bootstrap_seed_peers,
        )
        .await
    }
    async fn discovery(&self) -> Result<DiscoverySnapshot> {
        self.transport_discovery_impl().await
    }
}

#[async_trait]
impl HintTransport for IrohGossipTransport {
    async fn subscribe_hints(&self, topic: &TopicId) -> Result<HintStream> {
        self.hint_subscribe_hints_impl(topic).await
    }
    async fn unsubscribe_hints(&self, topic: &TopicId) -> Result<()> {
        self.hint_unsubscribe_hints_impl(topic).await
    }
    async fn publish_hint(&self, topic: &TopicId, hint: GossipHint) -> Result<()> {
        self.hint_publish_hint_impl(topic, hint).await
    }
}
