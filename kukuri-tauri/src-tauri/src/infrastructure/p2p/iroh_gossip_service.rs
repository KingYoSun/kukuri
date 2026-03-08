use super::GossipService;
use crate::domain::entities::Event;
use crate::infrastructure::p2p::utils::{
    ParsedPeer, normalize_endpoint_addr, parse_peer_hint, sanitize_remote_endpoint_addr,
};
use crate::shared::error::AppError;
use async_trait::async_trait;
use futures::StreamExt;
use iroh::{address_lookup::MemoryLookup, protocol::Router};
use iroh_gossip::{
    ALPN as GOSSIP_ALPN,
    api::{Event as GossipApiEvent, GossipSender, GossipTopic},
    net::Gossip,
    proto::TopicId,
};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex as TokioMutex, RwLock, broadcast, mpsc};
use tokio::time::timeout;

use crate::domain::p2p::events::P2PEvent;
use crate::domain::p2p::message::{GossipMessage, MessageType};
use crate::domain::p2p::{TopicMesh, TopicStats, generate_topic_id, topic_id_bytes};

const LOG_TARGET: &str = "kukuri::p2p::gossip";
const METRICS_TARGET: &str = "kukuri::p2p::metrics";

pub struct IrohGossipService {
    endpoint: Arc<iroh::Endpoint>,
    static_discovery: Arc<MemoryLookup>,
    gossip: Arc<Gossip>,
    _router: Arc<Router>,
    topics: Arc<RwLock<HashMap<String, TopicHandle>>>,
    event_tx: Option<broadcast::Sender<P2PEvent>>,
    allow_direct_addrs: bool,
}

struct TopicHandle {
    sender: Arc<TokioMutex<GossipSender>>, // GossipSenderでbroadcast可能
    receiver_task: tokio::task::JoinHandle<()>,
    mesh: Arc<TopicMesh>,
}

#[derive(Debug, Deserialize)]
struct LegacyRawEventPayload {
    id: String,
    pubkey: String,
    created_at: i64,
    kind: u32,
    tags: Vec<Vec<String>>,
    content: String,
    sig: String,
}

fn parse_domain_event_payload(payload: &[u8]) -> Result<Event, String> {
    if let Ok(event) = serde_json::from_slice::<Event>(payload) {
        return Ok(event);
    }

    let legacy = serde_json::from_slice::<LegacyRawEventPayload>(payload)
        .map_err(|err| format!("failed to decode event payload: {err}"))?;
    let created_at = chrono::DateTime::<chrono::Utc>::from_timestamp(legacy.created_at, 0)
        .ok_or_else(|| format!("invalid legacy created_at: {}", legacy.created_at))?;

    Ok(Event {
        id: legacy.id,
        pubkey: legacy.pubkey,
        created_at,
        kind: legacy.kind,
        tags: legacy.tags,
        content: legacy.content,
        sig: legacy.sig,
    })
}

impl IrohGossipService {
    pub fn new(
        endpoint: Arc<iroh::Endpoint>,
        static_discovery: Arc<MemoryLookup>,
        allow_direct_addrs: bool,
    ) -> Result<Self, AppError> {
        // Gossipインスタンスの作成
        let gossip = Gossip::builder().spawn((*endpoint).clone());

        // Routerの作成とGossipプロトコルの登録
        let router = Router::builder((*endpoint).clone())
            .accept(GOSSIP_ALPN, gossip.clone())
            .spawn();

        Ok(Self {
            endpoint,
            static_discovery,
            gossip: Arc::new(gossip),
            _router: Arc::new(router),
            topics: Arc::new(RwLock::new(HashMap::new())),
            event_tx: None,
            allow_direct_addrs,
        })
    }

    pub fn set_event_sender(&mut self, tx: broadcast::Sender<P2PEvent>) {
        self.event_tx = Some(tx);
    }

    pub fn local_peer_hint(&self) -> Option<String> {
        let node_addr = normalize_endpoint_addr(&self.endpoint.addr(), self.allow_direct_addrs);
        let node_id = node_addr.id.to_string();
        if let Some(addr) = node_addr.ip_addrs().next() {
            return Some(format!("{node_id}@{addr}"));
        }
        node_addr
            .relay_urls()
            .next()
            .map(|relay_url| format!("{node_id}|relay={relay_url}"))
    }

    fn create_topic_id(topic: &str) -> TopicId {
        let bytes = topic_id_bytes(topic);
        TopicId::from_bytes(bytes)
    }

    fn register_initial_peer_addrs(
        &self,
        topic: &str,
        parsed_peers: &[ParsedPeer],
        reuse_existing_topic: bool,
    ) -> Vec<iroh::EndpointAddr> {
        if parsed_peers.is_empty() {
            return Vec::new();
        }

        let action = if reuse_existing_topic {
            "re-applying"
        } else {
            "applying"
        };
        let mut registered = Vec::new();
        eprintln!(
            "[iroh_gossip_service] {} {} initial peers for topic {}",
            action,
            parsed_peers.len(),
            topic
        );

        for peer in parsed_peers {
            if let Some(addr) = &peer.node_addr {
                let sanitized_addr = sanitize_remote_endpoint_addr(addr, self.allow_direct_addrs);
                eprintln!(
                    "[iroh_gossip_service] {} node addr {} for topic {}",
                    action, sanitized_addr.id, topic
                );
                self.static_discovery
                    .add_endpoint_info(sanitized_addr.clone());
                registered.push(sanitized_addr);
            }
        }

        registered
    }

    async fn preconnect_initial_peers(&self, topic: &str, peer_addrs: &[iroh::EndpointAddr]) {
        let mut connected = HashSet::new();
        for node_addr in peer_addrs {
            if !connected.insert(node_addr.id) {
                continue;
            }

            match timeout(
                Duration::from_secs(5),
                self.endpoint.connect(node_addr.clone(), GOSSIP_ALPN),
            )
            .await
            {
                Ok(Ok(_connection)) => {
                    tracing::debug!(
                        topic = %topic,
                        peer = %node_addr.id,
                        "Preconnected gossip peer from initial hint"
                    );
                }
                Ok(Err(error)) => {
                    tracing::warn!(
                        topic = %topic,
                        peer = %node_addr.id,
                        error = %error,
                        "Failed to preconnect gossip peer from initial hint"
                    );
                }
                Err(_) => {
                    tracing::warn!(
                        topic = %topic,
                        peer = %node_addr.id,
                        "Timed out preconnecting gossip peer from initial hint"
                    );
                }
            }
        }
    }

    async fn remove_topic_handle(&self, topic: &str) -> Option<Arc<TopicMesh>> {
        let existing_handle = {
            let mut topics = self.topics.write().await;
            topics.remove(topic)
        };

        existing_handle.map(|handle| {
            handle.receiver_task.abort();
            drop(handle.sender);
            handle.mesh
        })
    }

    async fn subscribe_topic(
        &self,
        topic: &str,
        parsed_peers: &[ParsedPeer],
        mesh: Option<Arc<TopicMesh>>,
    ) -> Result<(), AppError> {
        let registered_peer_addrs =
            self.register_initial_peer_addrs(topic, parsed_peers, mesh.is_some());
        self.preconnect_initial_peers(topic, &registered_peer_addrs)
            .await;

        let canonical_topic = generate_topic_id(topic);
        let topic_id = Self::create_topic_id(&canonical_topic);
        let peer_ids: Vec<_> = parsed_peers.iter().map(|p| p.node_id).collect();
        eprintln!(
            "[iroh_gossip_service] subscribing topic {} (canonical {}) with {} peer hints",
            topic,
            canonical_topic,
            peer_ids.len()
        );

        let gossip_topic: GossipTopic = self
            .gossip
            .subscribe(topic_id, peer_ids.clone())
            .await
            .map_err(|e| AppError::P2PError(format!("Failed to subscribe to topic: {e:?}")))?;

        let mesh = mesh.unwrap_or_else(|| Arc::new(TopicMesh::new(topic.to_string())));
        let mut initial_neighbors = Vec::new();
        let (sender_handle, mut receiver) = gossip_topic.split();

        if !peer_ids.is_empty() {
            sender_handle
                .join_peers(peer_ids.clone())
                .await
                .map_err(|e| {
                    AppError::P2PError(format!("Failed to join gossip bootstrap peers: {e:?}"))
                })?;
            eprintln!(
                "[iroh_gossip_service] join_peers issued for topic {} ({} peers)",
                topic,
                peer_ids.len()
            );
        }

        let wait_duration = Duration::from_secs(12);
        match timeout(wait_duration, receiver.joined()).await {
            Ok(Ok(())) => {
                eprintln!("[iroh_gossip_service] first neighbor joined for topic {topic}");
                initial_neighbors = receiver
                    .neighbors()
                    .map(|neighbor| neighbor.as_bytes().to_vec())
                    .collect();
            }
            Ok(Err(e)) => {
                tracing::debug!("Waiting for neighbor on {} returned error: {:?}", topic, e);
            }
            Err(_) => {
                tracing::warn!(
                    "Timed out ({:?}) waiting for neighbor on {}",
                    wait_duration,
                    topic
                );
            }
        }

        for neighbor in receiver.neighbors() {
            let peer_bytes = neighbor.as_bytes().to_vec();
            if !initial_neighbors
                .iter()
                .any(|existing| existing == &peer_bytes)
            {
                initial_neighbors.push(peer_bytes);
            }
        }

        if !initial_neighbors.is_empty() {
            for peer in &initial_neighbors {
                mesh.update_peer_status(peer.clone(), true).await;
            }
            eprintln!(
                "[iroh_gossip_service] synced {} existing neighbors into mesh for topic {}",
                initial_neighbors.len(),
                topic
            );
        }

        let sender = Arc::new(TokioMutex::new(sender_handle));

        // 受信タスクを起動（UI配信用にサブスクライバへ配布 & 任意でP2PEventを送出）
        let topic_clone = topic.to_string();
        let event_tx_clone = self.event_tx.clone();
        let mesh_for_task = mesh.clone();
        let receiver_task = tokio::spawn(async move {
            while let Some(event) = receiver.next().await {
                match event {
                    Ok(GossipApiEvent::Received(msg)) => {
                        let decoded_message = match GossipMessage::from_bytes(&msg.content) {
                            Ok(message) => Some(message),
                            Err(e) => {
                                tracing::debug!(
                                    target: LOG_TARGET,
                                    topic = %topic_clone,
                                    error = ?e,
                                    "Failed to decode gossip payload into GossipMessage"
                                );
                                None
                            }
                        };

                        if let Some(message) = decoded_message.as_ref()
                            && let Err(e) = mesh_for_task.handle_message(message.clone()).await
                        {
                            tracing::debug!(
                                target: LOG_TARGET,
                                topic = %topic_clone,
                                error = ?e,
                                "Failed to record gossip message in TopicMesh"
                            );
                        }

                        if let (Some(tx), Some(message)) =
                            (event_tx_clone.as_ref(), decoded_message.clone())
                        {
                            let _ = tx.send(P2PEvent::MessageReceived {
                                topic_id: topic_clone.clone(),
                                message,
                                _from_peer: msg.delivered_from.as_bytes().to_vec(),
                            });
                        }

                        let event_result = if let Some(message) = decoded_message.as_ref() {
                            // msg_type が想定外でも payload 側がイベントJSONなら取り込む。
                            parse_domain_event_payload(&message.payload)
                                .or_else(|_| parse_domain_event_payload(&msg.content))
                        } else {
                            // 後方互換のため、生バイト列をそのまま JSON として扱う
                            parse_domain_event_payload(&msg.content)
                        };

                        match event_result {
                            Ok(domain_event) => {
                                match domain_event
                                    .validate_nip01()
                                    .and_then(|_| domain_event.validate_nip10_19())
                                {
                                    Ok(_) => {
                                        super::metrics::record_receive_success();
                                        let snap = super::metrics::snapshot();
                                        tracing::trace!(
                                            target: METRICS_TARGET,
                                            action = "receive",
                                            topic = %topic_clone,
                                            received = snap.messages_received,
                                            receive_failures = snap.receive_details.failures,
                                            "Validated gossip payload"
                                        );
                                    }
                                    Err(e) => {
                                        super::metrics::record_receive_failure();
                                        let snap = super::metrics::snapshot();
                                        tracing::warn!(
                                            target: METRICS_TARGET,
                                            action = "receive_failure",
                                            topic = %topic_clone,
                                            failures = snap.receive_details.failures,
                                            error = %e,
                                            "Dropped invalid Nostr event after validation"
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                super::metrics::record_receive_failure();
                                let snap = super::metrics::snapshot();
                                tracing::warn!(
                                    target: METRICS_TARGET,
                                    action = "receive_failure",
                                    topic = %topic_clone,
                                    failures = snap.receive_details.failures,
                                    error = %e,
                                    "Failed to decode gossip payload as Nostr event"
                                );
                            }
                        }
                    }
                    Ok(GossipApiEvent::NeighborUp(peer)) => {
                        let peer_bytes = peer.as_bytes().to_vec();
                        if let Some(tx) = &event_tx_clone {
                            let _ = tx.send(P2PEvent::PeerJoined {
                                topic_id: topic_clone.clone(),
                                peer_id: peer_bytes.clone(),
                            });
                        } else {
                            tracing::info!("Neighbor up on {}: {:?}", topic_clone, peer);
                        }
                        mesh_for_task.update_peer_status(peer_bytes, true).await;
                    }
                    Ok(GossipApiEvent::NeighborDown(peer)) => {
                        let peer_bytes = peer.as_bytes().to_vec();
                        if let Some(tx) = &event_tx_clone {
                            let _ = tx.send(P2PEvent::PeerLeft {
                                topic_id: topic_clone.clone(),
                                peer_id: peer_bytes.clone(),
                            });
                        } else {
                            tracing::info!("Neighbor down on {}: {:?}", topic_clone, peer);
                        }
                        mesh_for_task.update_peer_status(peer_bytes, false).await;
                    }
                    Ok(GossipApiEvent::Lagged) => {
                        tracing::warn!("Receiver lagged on topic {}", topic_clone);
                    }
                    Err(e) => {
                        tracing::error!("Gossip receiver error on {}: {:?}", topic_clone, e);
                    }
                }
            }
        });

        let handle = TopicHandle {
            sender,
            receiver_task,
            mesh,
        };

        let mut topics = self.topics.write().await;
        topics.insert(topic.to_string(), handle);

        Ok(())
    }
}

#[async_trait]
impl GossipService for IrohGossipService {
    fn local_peer_hint(&self) -> Option<String> {
        IrohGossipService::local_peer_hint(self)
    }

    async fn join_topic(&self, topic: &str, initial_peers: Vec<String>) -> Result<(), AppError> {
        eprintln!(
            "[iroh_gossip_service] join_topic start: {} (initial peers: {:?})",
            topic, initial_peers
        );
        let parsed_peers: Vec<ParsedPeer> = initial_peers
            .into_iter()
            .filter_map(|entry| match parse_peer_hint(&entry) {
                Ok(parsed) => Some(parsed),
                Err(e) => {
                    tracing::warn!("Failed to parse initial peer '{}': {:?}", entry, e);
                    None
                }
            })
            .collect();

        eprintln!(
            "[iroh_gossip_service] parsed {} peers for topic {}",
            parsed_peers.len(),
            topic
        );

        let topic_exists = {
            let topics = self.topics.read().await;
            topics.contains_key(topic)
        };

        if topic_exists {
            if parsed_peers.is_empty() {
                tracing::debug!(
                    "Topic {} already joined and no new peer hints were provided",
                    topic
                );
                return Ok(());
            }

            eprintln!(
                "[iroh_gossip_service] rebuilding existing topic {} with {} peer hints",
                topic,
                parsed_peers.len()
            );
            let preserved_mesh = self.remove_topic_handle(topic).await;
            return self
                .subscribe_topic(topic, &parsed_peers, preserved_mesh)
                .await;
        }

        self.subscribe_topic(topic, &parsed_peers, None).await
    }

    async fn leave_topic(&self, topic: &str) -> Result<(), AppError> {
        let mut topics = self.topics.write().await;

        if let Some(handle) = topics.remove(topic) {
            // レシーバータスクをキャンセルし、Senderをドロップ
            handle.receiver_task.abort();
            drop(handle.sender);

            tracing::info!("Left gossip topic: {}", topic);
        } else {
            tracing::debug!("Topic not found: {}", topic);
        }

        Ok(())
    }

    async fn broadcast(&self, topic: &str, event: &Event) -> Result<(), AppError> {
        let topics = self.topics.read().await;

        if let Some(handle) = topics.get(topic) {
            // イベントをシリアライズ
            let payload = serde_json::to_vec(event)?;
            let sender_id = self.endpoint.addr().id.to_string().into_bytes();
            let gossip_message = GossipMessage::new(MessageType::NostrEvent, payload, sender_id);
            let message_bytes = gossip_message.to_bytes().map_err(|e| {
                AppError::P2PError(format!("Failed to serialize gossip message: {e}"))
            })?;

            // Senderを取得してブロードキャスト
            let sender = handle.sender.clone();
            drop(topics);

            let guard = sender.lock().await;
            guard
                .broadcast(message_bytes.into())
                .await
                .map_err(|e| AppError::P2PError(format!("Failed to broadcast: {e:?}")))?;

            tracing::debug!("Broadcasted event to topic {}", topic);
            Ok(())
        } else {
            Err(format!("Not joined to topic: {topic}").into())
        }
    }

    async fn subscribe(&self, topic: &str) -> Result<mpsc::Receiver<Event>, AppError> {
        // トピックに参加していることを確認
        self.join_topic(topic, vec![]).await?;

        let mesh = {
            let topics = self.topics.read().await;
            topics.get(topic).map(|handle| handle.mesh.clone())
        };

        if let Some(mesh) = mesh {
            let topic_name = topic.to_string();
            let subscription = mesh.subscribe().await;
            let subscription_id = subscription.id;
            let mut message_rx = subscription.receiver;
            let mesh_clone = mesh.clone();
            let (tx, rx) = mpsc::channel(100);

            tokio::spawn(async move {
                while let Some(message) = message_rx.recv().await {
                    match parse_domain_event_payload(&message.payload) {
                        Ok(domain_event) => {
                            match domain_event
                                .validate_nip01()
                                .and_then(|_| domain_event.validate_nip10_19())
                            {
                                Ok(_) => {
                                    if tx.send(domain_event.clone()).await.is_err() {
                                        break;
                                    }
                                }
                                Err(e) => {
                                    tracing::debug!(
                                        target: LOG_TARGET,
                                        topic = %topic_name,
                                        error = %e,
                                        "Dropped invalid domain event in subscription bridge"
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            tracing::debug!(
                                target: LOG_TARGET,
                                topic = %topic_name,
                                error = %e,
                                "Failed to decode gossip payload into Event for subscription"
                            );
                        }
                    }
                }

                mesh_clone.unsubscribe(subscription_id).await;
            });

            Ok(rx)
        } else {
            Err(format!("Not joined to topic: {topic}").into())
        }
    }

    async fn get_joined_topics(&self) -> Result<Vec<String>, AppError> {
        let topics = self.topics.read().await;
        Ok(topics.keys().cloned().collect())
    }

    async fn get_topic_peers(&self, topic: &str) -> Result<Vec<String>, AppError> {
        let topics = self.topics.read().await;

        if let Some(_handle) = topics.get(topic) {
            // iroh-gossipのAPIでピアリストを取得
            // Note: iroh-gossip doesn't expose a direct way to get topic peers
            // Return empty list for now
            Ok(Vec::new())
        } else {
            Err(format!("Not joined to topic: {topic}").into())
        }
    }

    async fn get_topic_stats(&self, topic: &str) -> Result<Option<TopicStats>, AppError> {
        let mesh = {
            let topics = self.topics.read().await;
            topics.get(topic).map(|handle| handle.mesh.clone())
        };

        if let Some(mesh) = mesh {
            Ok(Some(mesh.get_stats().await))
        } else {
            Ok(None)
        }
    }

    async fn broadcast_message(&self, topic: &str, message: &[u8]) -> Result<(), AppError> {
        let topics = self.topics.read().await;

        if let Some(_handle) = topics.get(topic) {
            // メッセージをブロードキャスト
            // Simplified - actual implementation needs proper API
            tracing::debug!(
                "Broadcasting raw message to topic {}: {} bytes",
                topic,
                message.len()
            );
            Ok(())
        } else {
            Err(format!("Not joined to topic: {topic}").into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::Event;
    use iroh::Endpoint;

    fn should_run_p2p_tests(test_name: &str) -> bool {
        if std::env::var("ENABLE_P2P_INTEGRATION").unwrap_or_default() != "1" {
            eprintln!("skipping {test_name} (ENABLE_P2P_INTEGRATION!=1)");
            false
        } else {
            true
        }
    }

    #[tokio::test]
    async fn test_join_and_broadcast_without_peers() {
        if !should_run_p2p_tests("test_join_and_broadcast_without_peers") {
            return;
        }
        // エンドポイント作成（ローカル、ディスカバリ無し）
        let static_discovery = Arc::new(MemoryLookup::new());
        let endpoint = Arc::new(
            Endpoint::empty_builder(iroh::RelayMode::Default)
                .address_lookup(static_discovery.clone())
                .bind()
                .await
                .unwrap(),
        );
        let service = IrohGossipService::new(endpoint, static_discovery, true).unwrap();

        // トピック参加
        let topic = "test-topic-ig";
        service.join_topic(topic, vec![]).await.unwrap();

        // ダミーイベントでブロードキャスト（ピア不在でもエラーにならない）
        let event = Event::new(1, "hello igossip".to_string(), "pubkey_test".to_string());
        let result = service.broadcast(topic, &event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_join_and_leave_topic() {
        if !should_run_p2p_tests("test_join_and_leave_topic") {
            return;
        }
        let static_discovery = Arc::new(MemoryLookup::new());
        let endpoint = Arc::new(
            Endpoint::empty_builder(iroh::RelayMode::Default)
                .address_lookup(static_discovery.clone())
                .bind()
                .await
                .unwrap(),
        );
        let service = IrohGossipService::new(endpoint, static_discovery, true).unwrap();

        let topic = "test-topic-leave";
        service.join_topic(topic, vec![]).await.unwrap();
        let result = service.leave_topic(topic).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_existing_topic_with_peer_hints_rebuilds_handle() {
        if !should_run_p2p_tests("test_existing_topic_with_peer_hints_rebuilds_handle") {
            return;
        }

        let discovery_a = Arc::new(MemoryLookup::new());
        let endpoint_a = Arc::new(
            Endpoint::empty_builder(iroh::RelayMode::Default)
                .address_lookup(discovery_a.clone())
                .bind()
                .await
                .unwrap(),
        );
        let service_a = IrohGossipService::new(endpoint_a, discovery_a, true).unwrap();

        let discovery_b = Arc::new(MemoryLookup::new());
        let endpoint_b = Arc::new(
            Endpoint::empty_builder(iroh::RelayMode::Default)
                .address_lookup(discovery_b.clone())
                .bind()
                .await
                .unwrap(),
        );
        let service_b = IrohGossipService::new(endpoint_b, discovery_b, true).unwrap();

        let topic = "test-topic-rebuild";
        service_a.join_topic(topic, vec![]).await.unwrap();
        service_b.join_topic(topic, vec![]).await.unwrap();

        let sender_ptr_before = {
            let topics = service_b.topics.read().await;
            Arc::as_ptr(&topics.get(topic).unwrap().sender) as usize
        };

        let peer_hint = service_a
            .local_peer_hint()
            .expect("service_a should expose a local peer hint");
        service_b.join_topic(topic, vec![peer_hint]).await.unwrap();

        let sender_ptr_after = {
            let topics = service_b.topics.read().await;
            Arc::as_ptr(&topics.get(topic).unwrap().sender) as usize
        };

        assert_ne!(
            sender_ptr_before, sender_ptr_after,
            "existing topic handle should be replaced when peer hints are appended"
        );

        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            let stats = service_b
                .get_topic_stats(topic)
                .await
                .unwrap()
                .expect("topic stats");
            if stats.peer_count > 0 {
                break;
            }

            assert!(
                tokio::time::Instant::now() < deadline,
                "rebuilt topic should observe at least one neighbor"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    #[tokio::test]
    async fn test_relay_only_registration_preserves_public_remote_direct_addr() {
        let static_discovery = Arc::new(MemoryLookup::new());
        let endpoint = Arc::new(
            Endpoint::empty_builder(iroh::RelayMode::Default)
                .address_lookup(static_discovery.clone())
                .bind()
                .await
                .unwrap(),
        );
        let service = IrohGossipService::new(endpoint, static_discovery.clone(), false).unwrap();

        let parsed_peer = parse_peer_hint(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef|relay=https://relay.example|addr=1.1.1.1:11223",
        )
        .expect("relay+addr peer hint should parse");

        let registered =
            service.register_initial_peer_addrs("topic", &[parsed_peer.clone()], false);

        assert_eq!(registered.len(), 1);
        assert_eq!(registered[0].ip_addrs().count(), 1);
        assert_eq!(registered[0].relay_urls().count(), 1);

        let stored = static_discovery
            .get_endpoint_info(parsed_peer.node_id)
            .expect("peer should be stored in discovery")
            .into_endpoint_addr();
        assert_eq!(stored.ip_addrs().count(), 1);
        assert_eq!(stored.relay_urls().count(), 1);
    }

    #[tokio::test]
    async fn test_relay_only_registration_drops_private_remote_direct_addr() {
        let static_discovery = Arc::new(MemoryLookup::new());
        let endpoint = Arc::new(
            Endpoint::empty_builder(iroh::RelayMode::Default)
                .address_lookup(static_discovery.clone())
                .bind()
                .await
                .unwrap(),
        );
        let service = IrohGossipService::new(endpoint, static_discovery.clone(), false).unwrap();

        let parsed_peer = parse_peer_hint(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef|relay=https://relay.example|addr=127.0.0.1:11223",
        )
        .expect("relay+addr peer hint should parse");

        let registered =
            service.register_initial_peer_addrs("topic", &[parsed_peer.clone()], false);

        assert_eq!(registered.len(), 1);
        assert_eq!(registered[0].ip_addrs().count(), 0);
        assert_eq!(registered[0].relay_urls().count(), 1);

        let stored = static_discovery
            .get_endpoint_info(parsed_peer.node_id)
            .expect("peer should be stored in discovery")
            .into_endpoint_addr();
        assert_eq!(stored.ip_addrs().count(), 0);
        assert_eq!(stored.relay_urls().count(), 1);
    }
}
