//! DHT基盤のブートストラップ実装
//! irohのビルトインDHTディスカバリーを使用した分散型ピア発見
use super::utils::parse_node_addr;
use crate::shared::config::BootstrapSource;
use crate::shared::error::AppError;
use iroh::{Endpoint, EndpointAddr};
use iroh_gossip::{
    api::{GossipSender, GossipTopic},
    net::Gossip,
    proto::TopicId,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex as TokioMutex, RwLock};
use tracing::{debug, info, warn};

const LOG_TARGET: &str = "kukuri::p2p::dht";
const METRICS_TARGET: &str = "kukuri::p2p::metrics";

/// DHT統合付きGossipサービス
pub struct DhtGossip {
    gossip: Gossip,
    senders: Arc<RwLock<HashMap<String, Arc<TokioMutex<GossipSender>>>>>,
}

impl DhtGossip {
    /// DHT統合付きGossipを作成
    pub async fn new(endpoint: Arc<Endpoint>) -> Result<Self, AppError> {
        info!(target: LOG_TARGET, "Initializing DHT-integrated Gossip service");

        // iroh-gossipを作成
        let gossip = Gossip::builder().spawn(endpoint.as_ref().clone());

        info!(target: LOG_TARGET, "DHT-integrated Gossip initialized successfully");

        Ok(Self {
            gossip,
            senders: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// トピックに参加
    pub async fn join_topic(
        &self,
        topic: &[u8],
        neighbors: Vec<EndpointAddr>,
    ) -> Result<(), AppError> {
        let topic_id = Self::make_topic_id(topic);
        let topic_key = Self::topic_key(&topic_id);

        // subscribe には EndpointAddrのリストではなく、EndpointIdのリストが必要
        let peer_ids: Vec<_> = neighbors.iter().map(|addr| addr.id).collect();
        let topic: GossipTopic = self
            .gossip
            .subscribe(topic_id, peer_ids)
            .await
            .map_err(|e| {
                super::metrics::record_join_failure();
                warn!(
                    target: LOG_TARGET,
                    topic = %Self::fmt_topic_id(&topic_id),
                    error = ?e,
                    "Failed to join DHT topic"
                );
                AppError::P2PError(format!("Failed to join topic: {e:?}"))
            })?;

        // Sender を保存（Receiver は破棄しても参加状態は維持される）
        let (sender, _receiver) = topic.split();
        let sender = Arc::new(TokioMutex::new(sender));
        let mut senders = self.senders.write().await;
        senders.insert(topic_key, sender);

        super::metrics::record_join_success();
        let snap = super::metrics::snapshot();
        info!(
            target: METRICS_TARGET,
            action = "join",
            topic = %Self::fmt_topic_id(&topic_id),
            joins = snap.joins,
            join_failures = snap.join_details.failures,
            leaves = snap.leaves,
            broadcasts = snap.broadcasts_sent,
            received = snap.messages_received,
            "Joined DHT topic"
        );
        Ok(())
    }

    /// トピックから離脱
    pub async fn leave_topic(&self, topic: &[u8]) -> Result<(), AppError> {
        let topic_id = Self::make_topic_id(topic);
        let topic_key = Self::topic_key(&topic_id);
        let mut senders = self.senders.write().await;
        if senders.remove(&topic_key).is_some() {
            super::metrics::record_leave_success();
            let snap = super::metrics::snapshot();
            info!(
                target: METRICS_TARGET,
                action = "leave",
                topic = %Self::fmt_topic_id(&topic_id),
                leaves = snap.leaves,
                leave_failures = snap.leave_details.failures,
                joins = snap.joins,
                broadcasts = snap.broadcasts_sent,
                received = snap.messages_received,
                "Left DHT topic"
            );
            Ok(())
        } else {
            super::metrics::record_leave_failure();
            debug!(
                target: LOG_TARGET,
                topic = %Self::fmt_topic_id(&topic_id),
                "Leave requested for non-joined topic"
            );
            Ok(())
        }
    }

    /// メッセージをブロードキャスト
    pub async fn broadcast(&self, topic: &[u8], message: Vec<u8>) -> Result<(), AppError> {
        let topic_id = Self::make_topic_id(topic);
        let topic_key = Self::topic_key(&topic_id);

        // 既存 Sender を探す。なければ参加して作成。
        let sender_opt = {
            let senders = self.senders.read().await;
            senders.get(&topic_key).cloned()
        };

        let sender = match sender_opt {
            Some(s) => s,
            None => {
                // 近傍指定なしで join（Receiver は破棄）
                let topic: GossipTopic =
                    self.gossip.subscribe(topic_id, vec![]).await.map_err(|e| {
                        super::metrics::record_broadcast_failure();
                        super::metrics::record_mainline_route_failure();
                        warn!(
                            target: LOG_TARGET,
                            topic = %Self::fmt_topic_id(&topic_id),
                            error = ?e,
                            "Failed to lazily subscribe before broadcast"
                        );
                        AppError::P2PError(format!("Failed to subscribe before broadcast: {e:?}"))
                    })?;
                let (sender, _receiver) = topic.split();
                let sender = Arc::new(TokioMutex::new(sender));
                let mut senders = self.senders.write().await;
                senders.insert(topic_key.clone(), sender.clone());
                sender
            }
        };

        // ブロードキャスト
        let guard = sender.lock().await;
        let res = guard.broadcast(message.into()).await;

        match res {
            Ok(()) => {
                super::metrics::record_broadcast_success();
                super::metrics::record_mainline_route_success();
                let snap = super::metrics::snapshot();
                debug!(
                    target: METRICS_TARGET,
                    action = "broadcast",
                    topic = %Self::fmt_topic_id(&topic_id),
                    broadcasts = snap.broadcasts_sent,
                    broadcast_failures = snap.broadcast_details.failures,
                    joins = snap.joins,
                    leaves = snap.leaves,
                    received = snap.messages_received,
                    "Broadcasted message on topic"
                );
                Ok(())
            }
            Err(e) => {
                super::metrics::record_broadcast_failure();
                super::metrics::record_mainline_route_failure();
                warn!(
                    target: LOG_TARGET,
                    topic = %Self::fmt_topic_id(&topic_id),
                    error = ?e,
                    "Failed to broadcast gossip message"
                );
                Err(AppError::P2PError(format!("Failed to broadcast: {e:?}")))
            }
        }
    }

    /// Gossipインスタンスを取得
    pub fn gossip(&self) -> &Gossip {
        &self.gossip
    }

    fn make_topic_id(topic: &[u8]) -> TopicId {
        let hash = blake3::hash(topic);
        TopicId::from_bytes(*hash.as_bytes())
    }

    fn topic_key(topic_id: &TopicId) -> String {
        use std::fmt::Write as _;
        let bytes = topic_id.as_bytes();
        let mut s = String::with_capacity(64);
        for b in bytes {
            let _ = write!(&mut s, "{b:02x}");
        }
        s
    }

    fn fmt_topic_id(topic_id: &TopicId) -> String {
        Self::topic_key(topic_id)
    }
}

/// 共有シークレット管理（シンプル版）
pub mod secret {
    use super::*;
    use keyring::Entry;
    use rand::Rng;

    const SERVICE_NAME: &str = "kukuri";
    const SECRET_KEY: &str = "dht_secret";

    /// シークレットを取得または生成
    pub async fn get_or_create_secret() -> Result<Vec<u8>, AppError> {
        // キーリングから取得を試みる
        if let Ok(entry) = Entry::new(SERVICE_NAME, SECRET_KEY) {
            if let Ok(secret_str) = entry.get_password() {
                use base64::prelude::*;
                if let Ok(secret) = BASE64_STANDARD.decode(secret_str) {
                    return Ok(secret);
                }
            }
        }

        // 新しいシークレットを生成
        let mut rng = rand::rng();
        let mut secret = vec![0u8; 32];
        rng.fill(&mut secret[..]);

        // キーリングに保存
        if let Ok(entry) = Entry::new(SERVICE_NAME, SECRET_KEY) {
            use base64::prelude::*;
            let secret_str = BASE64_STANDARD.encode(&secret);
            let _ = entry.set_password(&secret_str);
        }

        Ok(secret)
    }

    /// シークレットをローテーション
    pub async fn rotate_secret() -> Result<Vec<u8>, AppError> {
        let mut rng = rand::rng();
        let mut secret = vec![0u8; 32];
        rng.fill(&mut secret[..]);

        // キーリングに保存
        if let Ok(entry) = Entry::new(SERVICE_NAME, SECRET_KEY) {
            use base64::prelude::*;
            let secret_str = BASE64_STANDARD.encode(&secret);
            let _ = entry.set_password(&secret_str);
        }

        info!("DHT secret rotated");
        Ok(secret)
    }
}

/// フォールバック機構
pub mod fallback {
    use super::*;
    use crate::infrastructure::p2p::bootstrap_config;
    use crate::infrastructure::p2p::metrics;

    /// ハードコードされたブートストラップノード（将来的に設定ファイルから読み込み）
    /// 形式: "NodeId@Address" (例: "abc123...@192.168.1.1:11204")
    const FALLBACK_NODES: &[&str] = &[
        // 本番環境用のブートストラップノードをここに追加
        // 例: "NodeId@IP:Port"
    ];

    /// フォールバックノードに接続
    pub async fn connect_to_fallback(endpoint: &Endpoint) -> Result<Vec<EndpointAddr>, AppError> {
        let mut connected_nodes = Vec::new();

        for node_str in FALLBACK_NODES {
            match parse_node_addr(node_str) {
                Ok(node_addr) => {
                    // ノードに接続を試みる
                    match endpoint.connect(node_addr.clone(), iroh_gossip::ALPN).await {
                        Ok(_) => {
                            info!("Connected to fallback node: {}", node_str);
                            metrics::record_mainline_connection_success();
                            connected_nodes.push(node_addr);
                        }
                        Err(e) => {
                            metrics::record_mainline_connection_failure();
                            debug!("Failed to connect to fallback node {}: {:?}", node_str, e);
                        }
                    }
                }
                Err(e) => {
                    metrics::record_mainline_connection_failure();
                    debug!("Failed to parse node address {}: {:?}", node_str, e);
                }
            }
        }

        if connected_nodes.is_empty() {
            return Err(AppError::P2PError(
                "Failed to connect to any fallback nodes".to_string(),
            ));
        }

        metrics::record_bootstrap_source(BootstrapSource::Fallback);

        Ok(connected_nodes)
    }

    /// ユーザーUI設定 または 設定ファイル（bootstrap_nodes.json）から NodeId@Addr を読み込み接続
    pub async fn connect_from_config(endpoint: &Endpoint) -> Result<Vec<EndpointAddr>, AppError> {
        // 1) ユーザー設定を優先
        let mut node_addrs = bootstrap_config::load_user_bootstrap_node_addrs();
        // 2) ユーザー設定が空なら、プロジェクト同梱のJSONを利用
        if node_addrs.is_empty() {
            node_addrs = bootstrap_config::load_bootstrap_node_addrs()?;
        }
        let mut connected = Vec::new();

        for node_addr in node_addrs {
            match endpoint.connect(node_addr.clone(), iroh_gossip::ALPN).await {
                Ok(_) => {
                    info!("Connected to config bootstrap node: {}", node_addr.id);
                    metrics::record_mainline_connection_success();
                    connected.push(node_addr);
                }
                Err(e) => {
                    metrics::record_mainline_connection_failure();
                    debug!("Failed to connect to config bootstrap node: {:?}", e);
                }
            }
        }

        if connected.is_empty() {
            return Err(AppError::P2PError(
                "Failed to connect to nodes from bootstrap_nodes.json".to_string(),
            ));
        }

        metrics::record_bootstrap_source(BootstrapSource::Fallback);

        Ok(connected)
    }
}
