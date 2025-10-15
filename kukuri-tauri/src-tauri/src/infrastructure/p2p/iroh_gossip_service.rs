use super::GossipService;
use crate::shared::error::AppError;
use crate::domain::entities::Event;
use crate::infrastructure::p2p::utils::{parse_peer_hint, ParsedPeer};
use async_trait::async_trait;
use futures::StreamExt;
use iroh::protocol::Router;
use iroh_gossip::{
    api::{Event as GossipApiEvent, GossipSender, GossipTopic},
    net::Gossip,
    proto::TopicId,
    ALPN as GOSSIP_ALPN,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Mutex as TokioMutex};
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::timeout;
use std::sync::Mutex as StdMutex;
use std::time::Duration;

use crate::modules::p2p::events::P2PEvent;
use crate::modules::p2p::message::GossipMessage;

const LOG_TARGET: &str = "kukuri::p2p::gossip";
const METRICS_TARGET: &str = "kukuri::p2p::metrics";

pub struct IrohGossipService {
    endpoint: Arc<iroh::Endpoint>,
    gossip: Arc<Gossip>,
    router: Arc<Router>,
    topics: Arc<RwLock<HashMap<String, TopicHandle>>>,
    event_tx: Option<Arc<StdMutex<UnboundedSender<P2PEvent>>>>,
}

struct TopicHandle {
    topic_id: String,
    iroh_topic_id: TopicId,
    sender: Arc<TokioMutex<GossipSender>>, // GossipSenderでbroadcast可能
    receiver_task: tokio::task::JoinHandle<()>,
    subscribers: Arc<RwLock<Vec<mpsc::Sender<Event>>>>,
}

impl IrohGossipService {
    pub fn new(endpoint: Arc<iroh::Endpoint>) -> Result<Self, AppError> {
        // Gossipインスタンスの作成
        let gossip = Gossip::builder().spawn((*endpoint).clone());

        // Routerの作成とGossipプロトコルの登録
        let router = Router::builder((*endpoint).clone())
            .accept(GOSSIP_ALPN, gossip.clone())
            .spawn();

        Ok(Self {
            endpoint,
            gossip: Arc::new(gossip),
            router: Arc::new(router),
            topics: Arc::new(RwLock::new(HashMap::new())),
            event_tx: None,
        })
    }

    pub fn set_event_sender(&mut self, tx: UnboundedSender<P2PEvent>) {
        self.event_tx = Some(Arc::new(StdMutex::new(tx)));
    }

    pub fn local_peer_hint(&self) -> Option<String> {
        let node_addr = self.endpoint.node_addr();
        let node_id = node_addr.node_id.to_string();
        node_addr
            .direct_addresses()
            .next()
            .map(|addr| format!("{}@{}", node_id, addr))
    }

    fn create_topic_id(topic: &str) -> TopicId {
        // トピック名からTopicIdを生成
        use blake3::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(topic.as_bytes());
        let hash = hasher.finalize();
        TopicId::from_bytes(*hash.as_bytes())
    }

    async fn apply_initial_peers(&self, topic: &str, parsed_peers: &[ParsedPeer]) -> Result<(), AppError> {
        if parsed_peers.is_empty() {
            return Ok(());
        }

        eprintln!(
            "[iroh_gossip_service] applying {} initial peers to existing topic {}",
            parsed_peers.len(),
            topic
        );

        for peer in parsed_peers {
            if let Some(addr) = &peer.node_addr {
                eprintln!(
                    "[iroh_gossip_service] re-applying node addr {} for topic {}",
                    addr.node_id,
                    topic
                );
                if let Err(e) = self
                    .endpoint
                    .add_node_addr_with_source(addr.clone(), "gossip-bootstrap")
                {
                    tracing::warn!("Failed to add node addr for {}: {:?}", topic, e);
                }
            }
        }

        let peer_ids: Vec<_> = parsed_peers.iter().map(|p| p.node_id).collect();
        if peer_ids.is_empty() {
            return Ok(());
        }

        let topics = self.topics.read().await;
        if let Some(handle) = topics.get(topic) {
            let sender = handle.sender.clone();
            drop(topics);
            if let Err(e) = sender.lock().await.join_peers(peer_ids).await {
                tracing::warn!("Failed to join peers for topic {}: {:?}", topic, e);
            }
        } else {
            tracing::debug!("Topic {} not found when applying initial peers", topic);
        }

        Ok(())
    }
}

#[async_trait]
impl GossipService for IrohGossipService {
    async fn join_topic(&self, topic: &str, initial_peers: Vec<String>) -> Result<(), AppError> {
        eprintln!(
            "[iroh_gossip_service] join_topic start: {} (initial peers: {:?})",
            topic,
            initial_peers
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

        {
            let topics = self.topics.read().await;
            if topics.contains_key(topic) {
                drop(topics);
                self.apply_initial_peers(topic, &parsed_peers).await?;
                return Ok(());
            }
            drop(topics);
        }

        for peer in &parsed_peers {
            if let Some(addr) = &peer.node_addr {
                if let Err(e) = self
                    .endpoint
                    .add_node_addr_with_source(addr.clone(), "gossip-bootstrap")
                {
                    tracing::warn!("Failed to add node addr for {}: {:?}", topic, e);
                }
            }
        }

        let topic_id = Self::create_topic_id(topic);
        let peer_ids: Vec<_> = parsed_peers.iter().map(|p| p.node_id).collect();
        eprintln!(
            "[iroh_gossip_service] subscribing topic {} with {} peer hints",
            topic,
            peer_ids.len()
        );

        let gossip_topic: GossipTopic = self
            .gossip
            .subscribe(topic_id, peer_ids.clone())
            .await
            .map_err(|e| AppError::P2PError(format!("Failed to subscribe to topic: {:?}", e)))?;

        let (mut sender_handle, mut receiver) = gossip_topic.split();

        if !peer_ids.is_empty() {
            if let Err(e) = sender_handle.join_peers(peer_ids.clone()).await {
                tracing::warn!("Failed to join peers for topic {}: {:?}", topic, e);
            } else {
                eprintln!(
                    "[iroh_gossip_service] join_peers issued for topic {} ({} peers)",
                    topic,
                    peer_ids.len()
                );
            }
        }

        let wait_duration = Duration::from_secs(12);
        match timeout(wait_duration, receiver.joined()).await {
            Ok(Ok(peer)) => {
                eprintln!(
                    "[iroh_gossip_service] first neighbor joined for topic {} ({:?})",
                    topic,
                    peer
                );
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

        let sender = Arc::new(TokioMutex::new(sender_handle));

        // 受信タスクを起動（UI配信用にサブスクライバへ配布 & 任意でP2PEventを送出）
        let topic_clone = topic.to_string();
        let event_tx_clone = self.event_tx.clone();
        let subscribers: Arc<RwLock<Vec<mpsc::Sender<Event>>>> = Arc::new(RwLock::new(Vec::new()));
        let subscribers_for_task = subscribers.clone();
        let receiver_task = tokio::spawn(async move {
            while let Some(event) = receiver.next().await {
                match event {
                    Ok(GossipApiEvent::Received(msg)) => {
                        if let Some(tx) = &event_tx_clone {
                            match GossipMessage::from_bytes(&msg.content) {
                                Ok(message) => {
                                    let _ = tx.lock().unwrap().send(P2PEvent::MessageReceived {
                                        topic_id: topic_clone.clone(),
                                        message,
                                        _from_peer: msg.delivered_from.as_bytes().to_vec(),
                                    });
                                }
                                Err(e) => {
                                    tracing::debug!(
                                        target: LOG_TARGET,
                                        topic = %topic_clone,
                                        error = ?e,
                                        "Failed to decode gossip payload into GossipMessage"
                                    );
                                }
                            }
                        }

                        match serde_json::from_slice::<Event>(&msg.content) {
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

                                        let subs = subscribers_for_task.read().await;
                                        for s in subs.iter() {
                                            let _ = s.send(domain_event.clone()).await;
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
                                    error = ?e,
                                    "Failed to decode gossip payload as Nostr event"
                                );
                            }
                        }
                    }
                    Ok(GossipApiEvent::NeighborUp(peer)) => {
                        if let Some(tx) = &event_tx_clone {
                            let _ = tx.lock().unwrap().send(P2PEvent::PeerJoined {
                                topic_id: topic_clone.clone(),
                                peer_id: peer.as_bytes().to_vec(),
                            });
                        } else {
                            tracing::info!("Neighbor up on {}: {:?}", topic_clone, peer);
                        }
                    }
                    Ok(GossipApiEvent::NeighborDown(peer)) => {
                        if let Some(tx) = &event_tx_clone {
                            let _ = tx.lock().unwrap().send(P2PEvent::PeerLeft {
                                topic_id: topic_clone.clone(),
                                peer_id: peer.as_bytes().to_vec(),
                            });
                        } else {
                            tracing::info!("Neighbor down on {}: {:?}", topic_clone, peer);
                        }
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
            topic_id: topic.to_string(),
            iroh_topic_id: topic_id,
            sender,
            receiver_task,
            subscribers,
        };

        let mut topics = self.topics.write().await;
        topics.insert(topic.to_string(), handle);

        Ok(())
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
            let message_bytes = serde_json::to_vec(event)?;

            // Senderを取得してブロードキャスト
            let sender = handle.sender.clone();
            drop(topics);

            let mut guard = sender.lock().await;
            guard
                .broadcast(message_bytes.into())
                .await
                .map_err(|e| AppError::P2PError(format!("Failed to broadcast: {:?}", e)))?;

            tracing::debug!("Broadcasted event to topic {}", topic);
            Ok(())
        } else {
            Err(format!("Not joined to topic: {}", topic).into())
        }
    }

    async fn subscribe(&self, topic: &str) -> Result<mpsc::Receiver<Event>, AppError> {
        // トピックに参加していることを確認
        self.join_topic(topic, vec![]).await?;
        
        let topics = self.topics.read().await;
        
        if let Some(handle) = topics.get(topic) {
            // 新しいレシーバーを作成し、サブスクライバに登録
            let (tx, rx) = mpsc::channel(100);
            {
                let mut subs = handle.subscribers.write().await;
                subs.push(tx);
            }
            Ok(rx)
        } else {
            Err(format!("Not joined to topic: {}", topic).into())
        }
    }

    async fn get_joined_topics(&self) -> Result<Vec<String>, AppError> {
        let topics = self.topics.read().await;
        Ok(topics.keys().cloned().collect())
    }

    async fn get_topic_peers(&self, topic: &str) -> Result<Vec<String>, AppError> {
        let topics = self.topics.read().await;
        
        if let Some(handle) = topics.get(topic) {
            // iroh-gossipのAPIでピアリストを取得
            // Note: iroh-gossip doesn't expose a direct way to get topic peers
            // Return empty list for now
            let neighbors = vec![];
            
            Ok(neighbors
                .into_iter()
                .map(|peer_id: ()| String::new())
                .collect())
        } else {
            Err(format!("Not joined to topic: {}", topic).into())
        }
    }
    
    async fn broadcast_message(&self, topic: &str, message: &[u8]) -> Result<(), AppError> {
        let topics = self.topics.read().await;
        
        if let Some(_handle) = topics.get(topic) {
            // メッセージをブロードキャスト
            // Simplified - actual implementation needs proper API
            tracing::debug!("Broadcasting raw message to topic {}: {} bytes", topic, message.len());
            Ok(())
        } else {
            Err(format!("Not joined to topic: {}", topic).into())
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
            eprintln!(
                "skipping {} (ENABLE_P2P_INTEGRATION!=1)",
                test_name
            );
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
        let endpoint = Arc::new(Endpoint::builder().bind().await.unwrap());
        let service = IrohGossipService::new(endpoint).unwrap();

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
        let endpoint = Arc::new(Endpoint::builder().bind().await.unwrap());
        let service = IrohGossipService::new(endpoint).unwrap();

        let topic = "test-topic-leave";
        service.join_topic(topic, vec![]).await.unwrap();
        let result = service.leave_topic(topic).await;
        assert!(result.is_ok());
    }
}
