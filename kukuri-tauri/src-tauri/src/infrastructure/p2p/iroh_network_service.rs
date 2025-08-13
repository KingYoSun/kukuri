use super::{NetworkService, NetworkStats, Peer};
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
    pub async fn new(secret_key: iroh::SecretKey) -> Result<Self, Box<dyn std::error::Error>> {
        // Endpointの作成
        let endpoint = Endpoint::builder()
            .secret_key(secret_key)
            .discovery_n0()
            .bind()
            .await?;

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

    pub async fn node_addr(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let node_addr = self.endpoint.node_addr();
        let addrs = match node_addr.get() {
            Ok(Some(addr)) => addr
                .direct_addresses()
                .map(|addr| addr.to_string())
                .collect(),
            Ok(None) => {
                tracing::warn!("No direct addresses available");
                vec![]
            }
            Err(e) => {
                tracing::error!("Failed to get node address: {}", e);
                return Err(Box::new(e));
            }
        };
        Ok(addrs)
    }
}

#[async_trait]
impl NetworkService for IrohNetworkService {
    async fn connect(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut connected = self.connected.write().await;
        *connected = true;
        tracing::info!("Network service connected");
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut connected = self.connected.write().await;
        *connected = false;
        
        // ピアリストをクリア
        let mut peers = self.peers.write().await;
        peers.clear();
        
        tracing::info!("Network service disconnected");
        Ok(())
    }

    async fn get_peers(&self) -> Result<Vec<Peer>, Box<dyn std::error::Error>> {
        let peers = self.peers.read().await;
        Ok(peers.clone())
    }

    async fn add_peer(&self, address: &str) -> Result<(), Box<dyn std::error::Error>> {
        // NodeAddrをパース
        let node_addr: iroh::NodeAddr = address.parse()
            .map_err(|e| format!("Failed to parse node address: {}", e))?;
        
        // ピアに接続
        self.endpoint.connect(node_addr.clone(), iroh_gossip::ALPN)
            .await
            .map_err(|e| format!("Failed to connect to peer: {}", e))?;
        
        // ピアリストに追加
        let mut peers = self.peers.write().await;
        let now = chrono::Utc::now().timestamp();
        peers.push(Peer {
            id: node_addr.node_id.to_string(),
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

    async fn remove_peer(&self, peer_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut peers = self.peers.write().await;
        peers.retain(|p| p.id != peer_id);
        
        // 統計を更新
        let mut stats = self.stats.write().await;
        stats.connected_peers = peers.len();
        
        tracing::info!("Removed peer: {}", peer_id);
        Ok(())
    }

    async fn get_stats(&self) -> Result<NetworkStats, Box<dyn std::error::Error>> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    async fn is_connected(&self) -> bool {
        let connected = self.connected.read().await;
        *connected
    }
}