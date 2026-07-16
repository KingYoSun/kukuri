//! ネットワーク非依存の高速テストダブル(FakeNetwork / FakeTransport)。
//!
//! Decision(WP-B15, 2026-07-16): FakeTransport は実 iroh 実装との「等価ダブル」
//! ではなく、ドメインロジックを高速に検証するための意図的な単純化と定義する
//! (例: active_path は経路判定をせず機械的に返す)。trait 挙動の実物基準の
//! 検証は実 iroh 統合テスト(transport の iroh/tests 16 本 + app-api の
//! TestIrohStack 経由 50 箇所超)が担い、Fake と実物を同一テストベクタで回す
//! 共有 conformance suite は作らない(恒久 Out of Scope)。
//! 再検討のトリガ: Fake と実物の挙動乖離に起因するバグが実際に発生した場合。

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
#[cfg(test)]
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use futures_util::StreamExt;
use kukuri_core::{GossipHint, TopicId};
use tokio::sync::{Mutex, broadcast};
#[cfg(test)]
use tokio::time::timeout;
use tokio_stream::wrappers::BroadcastStream;

use crate::config::{ConnectMode, ConnectionPath, DiscoveryMode, DiscoverySnapshot, SeedPeer};
use crate::diagnostics::{peer_status_detail, topic_status_detail};
use crate::traits::{
    HintEnvelope, HintStream, HintTransport, PeerSnapshot, TopicPeerSnapshot, Transport,
};

#[derive(Clone, Default)]
pub struct FakeNetwork {
    hints: Arc<Mutex<HashMap<String, broadcast::Sender<HintEnvelope>>>>,
    topic_subscribers: Arc<Mutex<HashMap<String, BTreeSet<String>>>>,
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
        let topic_subscribers = self.network.topic_subscribers.lock().await.clone();
        let topic_diagnostics = topics
            .iter()
            .cloned()
            .map(|topic| {
                let subscribed_peers = topic_subscribers.get(&topic).cloned().unwrap_or_default();
                let connected_peers = imported
                    .iter()
                    .filter(|peer| subscribed_peers.contains(*peer))
                    .cloned()
                    .collect::<Vec<_>>();
                let missing_peer_ids = imported
                    .iter()
                    .filter(|peer| !subscribed_peers.contains(*peer))
                    .cloned()
                    .collect::<Vec<_>>();
                TopicPeerSnapshot {
                    topic,
                    joined: !connected_peers.is_empty(),
                    peer_count: connected_peers.len(),
                    connected_peers: connected_peers.clone(),
                    configured_peer_ids: imported.clone(),
                    missing_peer_ids,
                    active_path: if connected_peers.is_empty() {
                        ConnectionPath::DirectP2p
                    } else {
                        ConnectionPath::RelaySupportedP2p
                    },
                    rendezvous_peer_ids: connected_peers.clone(),
                    fallback_peer_ids: Vec::new(),
                    last_received_at: None,
                    status_detail: topic_status_detail(imported.len(), connected_peers.len()),
                    last_error: None,
                }
            })
            .collect::<Vec<_>>();
        Ok(PeerSnapshot {
            connected: !imported.is_empty(),
            peer_count: imported.len(),
            connected_peers: imported.clone(),
            configured_peers: imported,
            subscribed_topics: topics,
            active_path: ConnectionPath::DirectP2p,
            fallback_peer_ids: Vec::new(),
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
            active_path: ConnectionPath::DirectP2p,
            fallback_peer_ids: Vec::new(),
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
        let hint_topic = kukuri_core::wire::hint_topic_id(topic);
        self.subscribed_topics
            .lock()
            .await
            .insert(hint_topic.as_str().to_string());
        self.network
            .topic_subscribers
            .lock()
            .await
            .entry(hint_topic.as_str().to_string())
            .or_default()
            .insert(self.local_id.clone());
        let sender = self.hint_sender(topic).await;
        let stream =
            BroadcastStream::new(sender.subscribe()).filter_map(|event| async move { event.ok() });
        Ok(Box::pin(stream))
    }

    async fn unsubscribe_hints(&self, topic: &TopicId) -> Result<()> {
        let hint_topic = kukuri_core::wire::hint_topic_id(topic);
        self.subscribed_topics
            .lock()
            .await
            .remove(hint_topic.as_str());
        let mut subscribers = self.network.topic_subscribers.lock().await;
        if let Some(topic_subscribers) = subscribers.get_mut(hint_topic.as_str()) {
            topic_subscribers.remove(self.local_id.as_str());
            if topic_subscribers.is_empty() {
                subscribers.remove(hint_topic.as_str());
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_support::{
        HintRoundtripParticipant, format_peer_snapshot, wait_for_hint_roundtrip,
    };

    fn initial_topic_join_timeout() -> Duration {
        if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
            Duration::from_secs(180)
        } else {
            Duration::from_secs(15)
        }
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
        let network = FakeNetwork::default();
        let transport_a = FakeTransport::new("transport-a", network.clone());
        let transport_b = FakeTransport::new("transport-b", network);
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
        let join_timeout = initial_topic_join_timeout();
        let (mut demo_stream_a, mut demo_stream_b) = tokio::try_join!(
            transport_a.subscribe_hints(&demo),
            transport_b.subscribe_hints(&demo)
        )
        .expect("subscribe demo hints");
        wait_for_hint_roundtrip(
            HintRoundtripParticipant {
                transport: &transport_a,
                stream: &mut demo_stream_a,
                expected_source_peer: None,
            },
            HintRoundtripParticipant {
                transport: &transport_b,
                stream: &mut demo_stream_b,
                expected_source_peer: None,
            },
            &demo,
            join_timeout,
            "demo",
        )
        .await;

        match timeout(join_timeout, async {
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
        {
            Ok(()) => {}
            Err(_) => {
                let peers_a = transport_a.peers().await.expect("peers a");
                let peers_b = transport_b.peers().await.expect("peers b");
                panic!(
                    "peer readiness timeout: a={} b={}",
                    format_peer_snapshot(&peers_a),
                    format_peer_snapshot(&peers_b)
                );
            }
        }

        match timeout(join_timeout, async {
            loop {
                let peers_a = transport_a.peers().await.expect("peers a");
                let demo_diag = peers_a
                    .topic_diagnostics
                    .iter()
                    .find(|topic| topic.topic == "hint/kukuri:topic:demo")
                    .expect("demo diag");
                if demo_diag.peer_count == 1 {
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
                    "demo peer count timeout: a={} b={}",
                    format_peer_snapshot(&peers_a),
                    format_peer_snapshot(&peers_b)
                );
            }
        }

        let mut test7_stream_a = transport_a
            .subscribe_hints(&test7)
            .await
            .expect("subscribe test7 a");

        match timeout(join_timeout, async {
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
        {
            Ok(()) => {}
            Err(_) => {
                let peers_a = transport_a.peers().await.expect("peers a");
                let peers_b = transport_b.peers().await.expect("peers b");
                panic!(
                    "initial peer counts timeout: a={} b={}",
                    format_peer_snapshot(&peers_a),
                    format_peer_snapshot(&peers_b)
                );
            }
        }

        let mut test7_stream_b = transport_b
            .subscribe_hints(&test7)
            .await
            .expect("subscribe test7 b");
        wait_for_hint_roundtrip(
            HintRoundtripParticipant {
                transport: &transport_a,
                stream: &mut test7_stream_a,
                expected_source_peer: None,
            },
            HintRoundtripParticipant {
                transport: &transport_b,
                stream: &mut test7_stream_b,
                expected_source_peer: None,
            },
            &test7,
            join_timeout,
            "test7",
        )
        .await;
        match timeout(join_timeout, async {
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
        {
            Ok(()) => {}
            Err(_) => {
                let peers_a = transport_a.peers().await.expect("peers a");
                let peers_b = transport_b.peers().await.expect("peers b");
                panic!(
                    "join peer count timeout: a={} b={}",
                    format_peer_snapshot(&peers_a),
                    format_peer_snapshot(&peers_b)
                );
            }
        }

        transport_b
            .unsubscribe_hints(&test7)
            .await
            .expect("unsubscribe test7 b");
        match timeout(join_timeout, async {
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
        {
            Ok(()) => {}
            Err(_) => {
                let peers_a = transport_a.peers().await.expect("peers a");
                let peers_b = transport_b.peers().await.expect("peers b");
                panic!(
                    "leave peer count timeout: a={} b={}",
                    format_peer_snapshot(&peers_a),
                    format_peer_snapshot(&peers_b)
                );
            }
        }
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
}
