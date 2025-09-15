/// DHT基盤のブートストラップ実装
/// irohのビルトインDHTディスカバリーを使用した分散型ピア発見

use crate::shared::error::AppError;
use iroh::Endpoint;
use iroh_gossip::net::Gossip;
use std::sync::Arc;
use tracing::{debug, info};

/// DHT統合付きGossipサービス
pub struct DhtGossip {
    gossip: Gossip,
    endpoint: Arc<Endpoint>,
}

impl DhtGossip {
    /// DHT統合付きGossipを作成
    pub async fn new(endpoint: Arc<Endpoint>) -> Result<Self, AppError> {
        info!("Initializing DHT-integrated Gossip service");

        // iroh-gossipを作成
        let gossip = Gossip::builder().spawn(endpoint.as_ref().clone());

        info!("DHT-integrated Gossip initialized successfully");

        Ok(Self {
            gossip,
            endpoint,
        })
    }

    /// トピックに参加
    pub async fn join_topic(
        &self,
        topic: &[u8],
        neighbors: Vec<iroh::NodeAddr>,
    ) -> Result<(), AppError> {
        let topic_id = blake3::hash(topic);
        let topic_bytes = *topic_id.as_bytes();

        // subscribe には NodeAddrのリストではなく、NodeIdのリストが必要
        let peer_ids: Vec<_> = neighbors.iter().map(|addr| addr.node_id).collect();
        
        self.gossip
            .subscribe(topic_bytes.into(), peer_ids)
            .await
            .map_err(|e| AppError::P2PError(format!("Failed to join topic: {:?}", e)))?;

        info!("Joined DHT topic: {:?}", topic_id);
        Ok(())
    }

    /// トピックから離脱
    pub async fn leave_topic(&self, _topic: &[u8]) -> Result<(), AppError> {
        // TODO: iroh-gossipのquitメソッドが使用可能になったら実装
        info!("Leave topic not yet implemented");
        Ok(())
    }

    /// メッセージをブロードキャスト
    pub async fn broadcast(&self, _topic: &[u8], _message: Vec<u8>) -> Result<(), AppError> {
        // TODO: iroh-gossipのbroadcastメソッドが使用可能になったら実装
        debug!("Broadcast not yet implemented");
        Ok(())
    }

    /// Gossipインスタンスを取得
    pub fn gossip(&self) -> &Gossip {
        &self.gossip
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
        let mut rng = rand::thread_rng();
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
        let mut rng = rand::thread_rng();
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
    use iroh::NodeAddr;
    use std::str::FromStr;

    /// ハードコードされたブートストラップノード（将来的に設定ファイルから読み込み）
    /// 形式: "NodeId@Address" (例: "abc123...@192.168.1.1:11204")
    const FALLBACK_NODES: &[&str] = &[
        // 本番環境用のブートストラップノードをここに追加
        // 例: "NodeId@IP:Port"
    ];

    /// フォールバックノードに接続
    pub async fn connect_to_fallback(endpoint: &Endpoint) -> Result<Vec<NodeAddr>, AppError> {
        let mut connected_nodes = Vec::new();
        
        for node_str in FALLBACK_NODES {
            match parse_node_addr(node_str) {
                Ok(node_addr) => {
                    // ノードに接続を試みる
                    match endpoint.connect(node_addr.clone(), iroh_gossip::ALPN).await {
                        Ok(_) => {
                            info!("Connected to fallback node: {}", node_str);
                            connected_nodes.push(node_addr);
                        }
                        Err(e) => {
                            debug!("Failed to connect to fallback node {}: {:?}", node_str, e);
                        }
                    }
                }
                Err(e) => {
                    debug!("Failed to parse node address {}: {:?}", node_str, e);
                }
            }
        }
        
        if connected_nodes.is_empty() {
            return Err(AppError::P2PError("Failed to connect to any fallback nodes".to_string()));
        }
        
        Ok(connected_nodes)
    }

    /// 設定ファイル（bootstrap_nodes.json）から NodeId@Addr を読み込み接続
    pub async fn connect_from_config(endpoint: &Endpoint) -> Result<Vec<NodeAddr>, AppError> {
        let node_addrs = bootstrap_config::load_bootstrap_node_addrs()?;
        let mut connected = Vec::new();

        for node_addr in node_addrs {
            match endpoint.connect(node_addr.clone(), iroh_gossip::ALPN).await {
                Ok(_) => {
                    info!("Connected to config bootstrap node: {}", node_addr.node_id);
                    connected.push(node_addr);
                }
                Err(e) => {
                    debug!("Failed to connect to config bootstrap node: {:?}", e);
                }
            }
        }

        if connected.is_empty() {
            return Err(AppError::P2PError("Failed to connect to nodes from bootstrap_nodes.json".to_string()));
        }

        Ok(connected)
    }
    
    /// ノードアドレス文字列をパース
    fn parse_node_addr(node_str: &str) -> Result<NodeAddr, AppError> {
        // 形式: "NodeId@Address"
        let parts: Vec<&str> = node_str.split('@').collect();
        if parts.len() != 2 {
            return Err(AppError::P2PError(format!("Invalid node address format: {}", node_str)));
        }
        
        let node_id = iroh::NodeId::from_str(parts[0])
            .map_err(|e| AppError::P2PError(format!("Failed to parse node ID: {}", e)))?;
        
        let socket_addr = parts[1].parse()
            .map_err(|e| AppError::P2PError(format!("Failed to parse socket address: {}", e)))?;
        
        Ok(NodeAddr::new(node_id).with_direct_addresses([socket_addr]))
    }
}
