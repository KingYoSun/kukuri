use std::pin::Pin;

use anyhow::Result;
use async_trait::async_trait;
use futures_util::Stream;
use kukuri_core::{GossipHint, TopicId};
use serde::{Deserialize, Serialize};

use crate::config::{DiscoveryMode, DiscoverySnapshot, SeedPeer};

pub type HintStream = Pin<Box<dyn Stream<Item = HintEnvelope> + Send>>;

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
