use super::GossipService;
use crate::shared::error::AppError;
use crate::domain::entities::Event;
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
use std::sync::Mutex as StdMutex;

use crate::modules::p2p::events::P2PEvent;
use crate::modules::p2p::message::GossipMessage;

pub struct IrohGossipService {
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
            gossip: Arc::new(gossip),
            router: Arc::new(router),
            topics: Arc::new(RwLock::new(HashMap::new())),
            event_tx: None,
        })
    }

    pub fn set_event_sender(&mut self, tx: UnboundedSender<P2PEvent>) {
        self.event_tx = Some(Arc::new(StdMutex::new(tx)));
    }

    fn create_topic_id(topic: &str) -> TopicId {
        // トピック名からTopicIdを生成
        use blake3::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(topic.as_bytes());
        let hash = hasher.finalize();
        TopicId::from_bytes(*hash.as_bytes())
    }
}

#[async_trait]
impl GossipService for IrohGossipService {
    async fn join_topic(&self, topic: &str, _initial_peers: Vec<String>) -> Result<(), AppError> {
        let mut topics = self.topics.write().await;

        // 既に参加済みの場合はスキップ
        if topics.contains_key(topic) {
            tracing::debug!("Already joined topic: {}", topic);
            return Ok(());
        }

        let topic_id = Self::create_topic_id(topic);

        // Gossip APIを使用してトピックに参加し、Sender/Receiverに分離
        let gossip_topic: GossipTopic = self
            .gossip
            .subscribe(topic_id, vec![])
            .await
            .map_err(|e| AppError::P2PError(format!("Failed to subscribe to topic: {:?}", e)))?;

        let (sender, mut receiver) = gossip_topic.split();

        // 受信タスクを起動（UI配信用にサブスクライバへ配布 & 任意でP2PEventを送出）
        let topic_clone = topic.to_string();
        let event_tx_clone = self.event_tx.clone();
        let subscribers: Arc<RwLock<Vec<mpsc::Sender<Event>>>> = Arc::new(RwLock::new(Vec::new()));
        let subscribers_for_task = subscribers.clone();
        let receiver_task = tokio::spawn(async move {
            while let Some(event) = receiver.next().await {
                match event {
                    Ok(GossipApiEvent::Received(msg)) => {
                        super::metrics::inc_received();
                        // 1) P2PEvent (GossipMessage) 経路（互換用途）
                        if let Some(tx) = &event_tx_clone {
                            if let Ok(message) = GossipMessage::from_bytes(&msg.content) {
                                let _ = tx.lock().unwrap().send(P2PEvent::MessageReceived {
                                    topic_id: topic_clone.clone(),
                                    message,
                                    _from_peer: msg.delivered_from.as_bytes().to_vec(),
                                });
                            }
                        }

                        // 2) UI向けサブスクライバ（domain::entities::Event）
                        if let Ok(domain_event) = serde_json::from_slice::<Event>(&msg.content) {
                            // NIP-01/10/19 に準拠しているか検証（不正は破棄）
                            if let Err(e) = domain_event.validate_nip01().and_then(|_| domain_event.validate_nip10_19()) {
                                tracing::warn!("Drop invalid Nostr event (NIP-01): {}", e);
                            } else {
                                let subs = subscribers_for_task.read().await;
                                for s in subs.iter() {
                                    let _ = s.send(domain_event.clone()).await;
                                }
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
            sender: Arc::new(TokioMutex::new(sender)),
            receiver_task,
            subscribers,
        };

        topics.insert(topic.to_string(), handle);
        tracing::info!("Joined gossip topic: {}", topic);

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

    #[tokio::test]
    async fn test_join_and_broadcast_without_peers() {
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
        let endpoint = Arc::new(Endpoint::builder().bind().await.unwrap());
        let service = IrohGossipService::new(endpoint).unwrap();

        let topic = "test-topic-leave";
        service.join_topic(topic, vec![]).await.unwrap();
        let result = service.leave_topic(topic).await;
        assert!(result.is_ok());
    }
}
