use std::sync::Arc;
use tokio::sync::RwLock;
use iroh::NodeAddr;

use crate::modules::p2p::error::{P2PError, Result as P2PResult};

pub struct PeerDiscovery {
    known_peers: Arc<RwLock<Vec<NodeAddr>>>,
    bootstrap_peers: Vec<NodeAddr>,
}

impl PeerDiscovery {
    /// 新しいPeerDiscoveryインスタンスを作成
    pub fn new(bootstrap_peers: Vec<NodeAddr>) -> Self {
        Self {
            known_peers: Arc::new(RwLock::new(Vec::new())),
            bootstrap_peers,
        }
    }
    
    /// ブートストラップピアを追加
    pub async fn add_bootstrap_peer(&mut self, peer: NodeAddr) {
        self.bootstrap_peers.push(peer.clone());
        self.add_peer(peer).await;
    }
    
    /// ピアを追加
    pub async fn add_peer(&self, peer: NodeAddr) {
        let mut peers = self.known_peers.write().await;
        if !peers.iter().any(|p| p.node_id == peer.node_id) {
            peers.push(peer);
        }
    }
    
    /// ピアを削除
    pub async fn remove_peer(&self, peer: &NodeAddr) {
        let mut peers = self.known_peers.write().await;
        peers.retain(|p| p.node_id != peer.node_id);
    }
    
    /// 既知のピアリストを取得
    pub async fn get_peers(&self) -> Vec<NodeAddr> {
        let peers = self.known_peers.read().await;
        peers.iter().cloned().collect()
    }
    
    /// トピック用の初期ピアを取得
    pub async fn get_initial_peers_for_topic(&self, _topic_id: &str) -> Vec<NodeAddr> {
        // 現時点では全ピアを返す
        // 将来的にはトピック別のピア管理を実装
        self.get_peers().await
    }
    
    /// ピア情報を文字列から解析
    pub fn parse_peer_addr(_addr_str: &str) -> P2PResult<NodeAddr> {
        // TODO: NodeAddrのパース実装
        Err(P2PError::InvalidPeerAddr("NodeAddr parsing not yet implemented".to_string()))
    }
    
    /// ピア交換メッセージを処理
    pub async fn handle_peer_exchange(&self, new_peers: Vec<NodeAddr>) {
        for peer in new_peers {
            self.add_peer(peer).await;
        }
    }
    
    /// アクティブなピア数を取得
    pub async fn peer_count(&self) -> usize {
        let peers = self.known_peers.read().await;
        peers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_peer_management() {
        let discovery = PeerDiscovery::new(vec![]);
        assert_eq!(discovery.peer_count().await, 0);
        
        // テスト用のNodeAddrは実際のアドレスパース実装後に追加
    }
}