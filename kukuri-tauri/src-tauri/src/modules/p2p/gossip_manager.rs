use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, Mutex};
use iroh::{Endpoint, protocol::Router, Watcher};
use iroh_gossip::{net::Gossip, ALPN as GOSSIP_ALPN_BYTES};
use iroh_gossip::proto::TopicId;
use iroh_gossip::api::{Event, GossipTopic, GossipSender};
use futures::StreamExt;

use crate::modules::p2p::error::{P2PError, Result as P2PResult};
use crate::modules::p2p::message::GossipMessage;


pub struct GossipManager {
    endpoint: Endpoint,
    gossip: Gossip,
    router: Router,
    topics: Arc<RwLock<HashMap<String, TopicHandle>>>,
    secret_key: secp256k1::SecretKey,
    event_tx: mpsc::UnboundedSender<P2PEvent>,
}

pub struct TopicHandle {
    topic_id: String,
    iroh_topic_id: TopicId,
    // gossip APIハンドルを保持
    sender: Arc<Mutex<GossipSender>>,
    receiver_task: tokio::task::JoinHandle<()>,
    mesh: Arc<crate::modules::p2p::topic_mesh::TopicMesh>,
}

#[derive(Clone, Debug)]
pub enum P2PEvent {
    MessageReceived {
        topic_id: String,
        message: GossipMessage,
        from_peer: Vec<u8>,
    },
    PeerJoined {
        topic_id: String,
        peer_id: Vec<u8>,
    },
    PeerLeft {
        topic_id: String,
        peer_id: Vec<u8>,
    },
}

impl GossipManager {
    /// 新しいGossipManagerを作成
    pub async fn new(iroh_secret_key: iroh::SecretKey, secp_secret_key: secp256k1::SecretKey, event_tx: mpsc::UnboundedSender<P2PEvent>) -> P2PResult<Self> {
        // Endpointの作成
        let endpoint = Endpoint::builder()
            .secret_key(iroh_secret_key)
            .discovery_n0()
            .bind()
            .await
            .map_err(|e| P2PError::EndpointInit(e.to_string()))?;
        
        // Gossipインスタンスの作成
        let gossip = Gossip::builder()
            .spawn(endpoint.clone());
        
        // Routerの作成
        let router = Router::builder(endpoint.clone())
            .accept(GOSSIP_ALPN_BYTES.to_vec(), gossip.clone())
            .spawn();
        
        Ok(Self {
            endpoint,
            gossip,
            router,
            topics: Arc::new(RwLock::new(HashMap::new())),
            secret_key: secp_secret_key,
            event_tx,
        })
    }
    
    /// 自身のNodeIDを取得
    pub fn node_id(&self) -> String {
        self.endpoint.node_id().to_string()
    }
    
    /// 自身のアドレス情報を取得
    pub async fn node_addr(&self) -> P2PResult<Vec<String>> {
        let node_addr = self.endpoint.node_addr();
        let addrs = match node_addr.get() {
            Ok(Some(addr)) => addr,
            Ok(None) => return Err(P2PError::Internal("Node address not available".to_string())),
            Err(e) => return Err(P2PError::Internal(format!("Failed to get node address: {}", e))),
        };
        
        Ok(addrs
            .direct_addresses()
            .map(|addr| addr.to_string())
            .collect())
    }
    
    /// トピックに参加
    pub async fn join_topic(&self, topic_id: &str, _initial_peers: Vec<String>) -> P2PResult<()> {
        let mut topics = self.topics.write().await;
        
        if topics.contains_key(topic_id) {
            return Ok(()); // 既に参加済み
        }
        
        // トピックIDを作成
        let mut topic_bytes = [0u8; 32];
        let bytes = topic_id.as_bytes();
        let len = bytes.len().min(32);
        topic_bytes[..len].copy_from_slice(&bytes[..len]);
        let iroh_topic_id = TopicId::from_bytes(topic_bytes);
        
        // ピアアドレスをパース
        let bootstrap_peers: Vec<iroh::NodeId> = Vec::new(); // TODO: initial_peersからNodeIdをパース
        
        // トピックに参加
        let gossip_topic: GossipTopic = self.gossip.subscribe(iroh_topic_id, bootstrap_peers)
            .await
            .map_err(|e| P2PError::JoinTopicFailed(format!("Failed to subscribe to topic: {}", e)))?;
        
        // 送信と受信を分離
        let (sender, mut receiver) = gossip_topic.split();
        
        // TopicMeshを作成
        let mesh = Arc::new(crate::modules::p2p::topic_mesh::TopicMesh::new(topic_id.to_string()));
        
        // イベント受信タスクを起動
        let event_tx = self.event_tx.clone();
        let topic_id_clone = topic_id.to_string();
        let mesh_clone = mesh.clone();
        let receiver_task = tokio::spawn(async move {
            while let Some(event) = receiver.next().await {
                match event {
                    Ok(Event::Received(msg)) => {
                        if let Ok(message) = GossipMessage::from_bytes(&msg.content) {
                            // 署名を検証
                            match message.verify_signature() {
                                Ok(true) => {
                                    // TopicMeshでメッセージを処理
                                    if let Err(e) = mesh_clone.handle_message(message.clone()).await {
                                        tracing::error!("Failed to handle message in topic mesh: {:?}", e);
                                    }
                                    
                                    let _ = event_tx.send(P2PEvent::MessageReceived {
                                        topic_id: topic_id_clone.clone(),
                                        message,
                                        from_peer: msg.delivered_from.as_bytes().to_vec(),
                                    });
                                },
                                Ok(false) => {
                                    tracing::warn!("Received message with invalid signature from {:?}", msg.delivered_from);
                                },
                                Err(e) => {
                                    tracing::error!("Failed to verify message signature: {}", e);
                                }
                            }
                        }
                    },
                    Ok(Event::NeighborUp(peer)) => {
                        mesh_clone.update_peer_status(peer.as_bytes().to_vec(), true).await;
                        let _ = event_tx.send(P2PEvent::PeerJoined {
                            topic_id: topic_id_clone.clone(),
                            peer_id: peer.as_bytes().to_vec(),
                        });
                    },
                    Ok(Event::NeighborDown(peer)) => {
                        mesh_clone.update_peer_status(peer.as_bytes().to_vec(), false).await;
                        let _ = event_tx.send(P2PEvent::PeerLeft {
                            topic_id: topic_id_clone.clone(),
                            peer_id: peer.as_bytes().to_vec(),
                        });
                    },
                    Ok(Event::Lagged) => {
                        tracing::warn!("Gossip receiver lagged, some messages may have been dropped");
                    },
                    Err(e) => {
                        tracing::error!("Gossip receiver error: {:?}", e);
                    }
                }
            }
        });
        
        let handle = TopicHandle {
            topic_id: topic_id.to_string(),
            iroh_topic_id,
            sender: Arc::new(Mutex::new(sender)),
            receiver_task,
            mesh,
        };
        
        topics.insert(topic_id.to_string(), handle);
        
        tracing::info!("Joined topic: {}", topic_id);
        Ok(())
    }
    
    /// トピックから離脱
    pub async fn leave_topic(&self, topic_id: &str) -> P2PResult<()> {
        let mut topics = self.topics.write().await;
        
        if let Some(handle) = topics.remove(topic_id) {
            // 受信タスクをキャンセル
            handle.receiver_task.abort();
            
            // gossipからの離脱
            // 送信チャネルをクローズ
            drop(handle.sender);
            
            tracing::info!("Left topic: {}", topic_id);
            Ok(())
        } else {
            Err(P2PError::TopicNotFound(topic_id.to_string()))
        }
    }
    
    /// メッセージをブロードキャスト
    pub async fn broadcast(&self, topic_id: &str, mut message: GossipMessage) -> P2PResult<()> {
        let topics = self.topics.read().await;
        
        if let Some(handle) = topics.get(topic_id) {
            // メッセージに署名
            message.sign(&self.secret_key)
                .map_err(|e| P2PError::SerializationError(format!("Failed to sign message: {}", e)))?;
            
            // メッセージをバイト列に変換
            let bytes = message.to_bytes()
                .map_err(|e| P2PError::SerializationError(e))?;
            
            // Arc<Mutex<GossipSender>>をクローンしてブロードキャスト
            let sender = handle.sender.clone();
            drop(topics); // RwLockを解放
            
            let mut sender_guard = sender.lock().await;
            sender_guard.broadcast(bytes.into())
                .await
                .map_err(|e| P2PError::BroadcastFailed(format!("Failed to broadcast message: {}", e)))?;
            
            tracing::debug!("Broadcast message to topic: {}", topic_id);
            Ok(())
        } else {
            Err(P2PError::TopicNotFound(topic_id.to_string()))
        }
    }
    
    /// アクティブなトピックのリストを取得
    pub async fn active_topics(&self) -> Vec<String> {
        let topics = self.topics.read().await;
        topics.keys().cloned().collect()
    }
    
    /// 特定のトピックのステータスを取得
    pub async fn get_topic_status(&self, topic_id: &str) -> Option<crate::modules::p2p::topic_mesh::TopicStats> {
        let topics = self.topics.read().await;
        
        if let Some(handle) = topics.get(topic_id) {
            Some(handle.mesh.get_stats().await)
        } else {
            None
        }
    }
    
    /// 全トピックのステータスを取得
    pub async fn get_all_topic_stats(&self) -> Vec<(String, crate::modules::p2p::topic_mesh::TopicStats)> {
        let topics = self.topics.read().await;
        let mut stats = Vec::new();
        
        for (topic_id, handle) in topics.iter() {
            stats.push((topic_id.clone(), handle.mesh.get_stats().await));
        }
        
        stats
    }
    
    /// シャットダウン
    pub async fn shutdown(&self) -> P2PResult<()> {
        // すべてのトピックから離脱
        let topic_ids: Vec<String> = {
            let topics = self.topics.read().await;
            topics.keys().cloned().collect()
        };
        
        for topic_id in topic_ids {
            let _ = self.leave_topic(&topic_id).await;
        }
        
        // Routerとエンドポイントのシャットダウン
        self.router.shutdown().await;
        self.endpoint.close().await;
        
        tracing::info!("GossipManager shutdown complete");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_gossip_manager_initialization() {
        let iroh_secret_key = iroh::SecretKey::generate(rand::thread_rng());
        let secp_secret_key = secp256k1::SecretKey::new(&mut rand::thread_rng());
        let (event_tx, _) = mpsc::unbounded_channel();
        let manager = GossipManager::new(iroh_secret_key, secp_secret_key, event_tx).await;
        assert!(manager.is_ok());
    }
    
    #[tokio::test]
    async fn test_topic_join_leave() {
        let iroh_secret_key = iroh::SecretKey::generate(rand::thread_rng());
        let secp_secret_key = secp256k1::SecretKey::new(&mut rand::thread_rng());
        let (event_tx, _) = mpsc::unbounded_channel();
        let manager = GossipManager::new(iroh_secret_key, secp_secret_key, event_tx).await.unwrap();
        
        let topic_id = "test-topic";
        
        // トピックに参加
        let result = manager.join_topic(topic_id, vec![]).await;
        assert!(result.is_ok());
        
        // 既に参加済みのトピックに再度参加
        let result = manager.join_topic(topic_id, vec![]).await;
        assert!(result.is_ok());
        
        // トピックから離脱
        let result = manager.leave_topic(topic_id).await;
        assert!(result.is_ok());
        
        // 存在しないトピックから離脱
        let result = manager.leave_topic(topic_id).await;
        assert!(result.is_err());
    }
}