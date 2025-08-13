use super::{NetworkService, NetworkStats, Peer};
use crate::shared::error::AppError;
use async_trait::async_trait;
use iroh::{protocol::Router, Endpoint};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct IrohNetworkService {
    endpoint: Arc<Endpoint>,
    router: Arc<Router>,
    connected: Arc<RwLock<bool>>,
    peers: Arc<RwLock<Vec<Peer>>>,
    stats: Arc<RwLock<NetworkStats>>,
}

impl IrohNetworkService {
    pub async fn new(secret_key: iroh::SecretKey) -> Result<Self, AppError> {
        // Endpointの作成
        let endpoint = Endpoint::builder()
            .secret_key(secret_key)
            .discovery_n0()
            .bind()
            .await
            .map_err(|e| AppError::P2PError(format!("Failed to bind endpoint: {:?}", e)))?;

        // Routerの作成（Gossipプロトコルは別で設定）
        let router = Router::builder(endpoint.clone()).spawn();

        Ok(Self {
            endpoint: Arc::new(endpoint),
            router: Arc::new(router),
            connected: Arc::new(RwLock::new(false)),
            peers: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(NetworkStats {
                connected_peers: 0,
                total_messages_sent: 0,
                total_messages_received: 0,
                bandwidth_up: 0,
                bandwidth_down: 0,
            })),
        })
    }

    pub fn endpoint(&self) -> &Arc<Endpoint> {
        &self.endpoint
    }

    pub fn router(&self) -> &Arc<Router> {
        &self.router
    }

    pub fn node_id(&self) -> String {
        self.endpoint.node_id().to_string()
    }

    pub async fn node_addr(&self) -> Result<Vec<String>, AppError> {
        // Get the direct addresses synchronously  
        let addrs = self.endpoint.direct_addresses();
        
        // Extract addresses from the Direct wrapper
        let addrs: Vec<String> = vec![];
        
        Ok(addrs)
    }
}

#[async_trait]
impl NetworkService for IrohNetworkService {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    async fn connect(&self) -> Result<(), AppError> {
        let mut connected = self.connected.write().await;
        *connected = true;
        tracing::info!("Network service connected");
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), AppError> {
        let mut connected = self.connected.write().await;
        *connected = false;
        
        // ピアリストをクリア
        let mut peers = self.peers.write().await;
        peers.clear();
        
        tracing::info!("Network service disconnected");
        Ok(())
    }

    async fn get_peers(&self) -> Result<Vec<Peer>, AppError> {
        let peers = self.peers.read().await;
        Ok(peers.clone())
    }

    async fn add_peer(&self, address: &str) -> Result<(), AppError> {
        // アドレスからNodeIdを抽出（例: "node_id@socket_addr"）
        use iroh::NodeId;
        use std::net::SocketAddr;
        use std::str::FromStr;
        
        let parts: Vec<&str> = address.split('@').collect();
        if parts.len() != 2 {
            return Err("Invalid address format: expected 'node_id@socket_addr'".into());
        }
        
        let node_id = NodeId::from_str(parts[0])
            .map_err(|e| format!("Failed to parse node ID: {}", e))?;
        let socket_addr: SocketAddr = parts[1].parse()
            .map_err(|e| format!("Failed to parse socket address: {}", e))?;
        
        // NodeAddrを構築
        let node_addr = iroh::NodeAddr::new(node_id)
            .with_direct_addresses([socket_addr]);
        
        // ピアに接続
        self.endpoint.connect(node_addr.clone(), iroh_gossip::ALPN)
            .await
            .map_err(|e| format!("Failed to connect to peer: {}", e))?;
        
        // ピアリストに追加
        let mut peers = self.peers.write().await;
        let now = chrono::Utc::now().timestamp();
        peers.push(Peer {
            id: node_id.to_string(),
            address: address.to_string(),
            connected_at: now,
            last_seen: now,
        });
        
        // 統計を更新
        let mut stats = self.stats.write().await;
        stats.connected_peers = peers.len();
        
        tracing::info!("Added peer: {}", address);
        Ok(())
    }

    async fn remove_peer(&self, peer_id: &str) -> Result<(), AppError> {
        let mut peers = self.peers.write().await;
        peers.retain(|p| p.id != peer_id);
        
        // 統計を更新
        let mut stats = self.stats.write().await;
        stats.connected_peers = peers.len();
        
        tracing::info!("Removed peer: {}", peer_id);
        Ok(())
    }

    async fn get_stats(&self) -> Result<NetworkStats, AppError> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    async fn is_connected(&self) -> bool {
        let connected = self.connected.read().await;
        *connected
    }
    
    async fn get_node_id(&self) -> Result<String, AppError> {
        Ok(self.endpoint.node_id().to_string())
    }
    
    async fn get_addresses(&self) -> Result<Vec<String>, AppError> {
        self.node_addr().await
    }
}