#[cfg(test)]
mod tests {
    use crate::modules::p2p::peer_discovery::*;
    use crate::modules::p2p::P2PError;
    use iroh::NodeAddr;

    fn create_test_node_addr(id: u8) -> NodeAddr {
        // テスト用のNodeAddrを作成
        // 有効なNodeIdを生成するため、シードから生成
        use iroh::SecretKey;
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        let mut seed = [0u8; 32];
        seed[0] = id;
        let mut rng = StdRng::from_seed(seed);
        let secret_key = SecretKey::generate(&mut rng);
        let node_id = secret_key.public();

        NodeAddr::new(node_id)
    }

    #[tokio::test]
    async fn test_peer_discovery_creation() {
        let bootstrap_peers = vec![create_test_node_addr(1), create_test_node_addr(2)];

        let discovery = PeerDiscovery::new(bootstrap_peers.clone());

        // 初期状態では既知のピアは空（ブートストラップピアは別管理）
        let peers = discovery.get_peers().await;
        assert_eq!(peers.len(), 0);
    }

    #[tokio::test]
    async fn test_add_peer() {
        let discovery = PeerDiscovery::new(vec![]);
        let peer1 = create_test_node_addr(1);
        let peer2 = create_test_node_addr(2);

        // ピアの追加
        discovery.add_peer(peer1.clone()).await;
        discovery.add_peer(peer2.clone()).await;

        let peers = discovery.get_peers().await;
        assert_eq!(peers.len(), 2);

        // 同じピアを再度追加しても重複しない
        discovery.add_peer(peer1.clone()).await;
        let peers = discovery.get_peers().await;
        assert_eq!(peers.len(), 2);
    }

    #[tokio::test]
    async fn test_remove_peer() {
        let discovery = PeerDiscovery::new(vec![]);
        let peer1 = create_test_node_addr(1);
        let peer2 = create_test_node_addr(2);

        // ピアの追加
        discovery.add_peer(peer1.clone()).await;
        discovery.add_peer(peer2.clone()).await;

        // ピアの削除
        discovery.remove_peer(&peer1).await;

        let peers = discovery.get_peers().await;
        assert_eq!(peers.len(), 1);
        assert!(peers.iter().any(|p| p.node_id == peer2.node_id));
        assert!(!peers.iter().any(|p| p.node_id == peer1.node_id));
    }

    #[tokio::test]
    async fn test_add_bootstrap_peer() {
        let mut discovery = PeerDiscovery::new(vec![]);
        let bootstrap_peer = create_test_node_addr(1);

        // ブートストラップピアの追加
        discovery.add_bootstrap_peer(bootstrap_peer.clone()).await;

        // ブートストラップピアは通常のピアリストにも追加される
        let peers = discovery.get_peers().await;
        assert_eq!(peers.len(), 1);
    }

    #[tokio::test]
    async fn test_get_initial_peers_for_topic() {
        let discovery = PeerDiscovery::new(vec![]);
        let peer1 = create_test_node_addr(1);
        let peer2 = create_test_node_addr(2);

        discovery.add_peer(peer1).await;
        discovery.add_peer(peer2).await;

        // 現時点ではトピック別管理は未実装なので、全ピアが返される
        let topic_peers = discovery.get_initial_peers_for_topic("test-topic").await;
        assert_eq!(topic_peers.len(), 2);
    }

    #[tokio::test]
    async fn test_parse_peer_addr() {
        // 現時点では未実装
        let result = PeerDiscovery::parse_peer_addr("test-addr");
        assert!(result.is_err());

        match result.unwrap_err() {
            P2PError::InvalidPeerAddr(msg) => {
                assert!(msg.contains("not yet implemented"));
            }
            _ => panic!("Expected InvalidPeerAddr error"),
        }
    }

    #[tokio::test]
    async fn test_handle_peer_exchange() {
        let discovery = PeerDiscovery::new(vec![]);
        let new_peers = vec![
            create_test_node_addr(1),
            create_test_node_addr(2),
            create_test_node_addr(3),
        ];

        // ピア交換処理
        discovery.handle_peer_exchange(new_peers.clone()).await;

        let peers = discovery.get_peers().await;
        assert_eq!(peers.len(), 3);
    }

    #[tokio::test]
    async fn test_peer_count() {
        let discovery = PeerDiscovery::new(vec![]);

        assert_eq!(discovery.peer_count().await, 0);

        for i in 0..5 {
            discovery.add_peer(create_test_node_addr(i)).await;
        }

        assert_eq!(discovery.peer_count().await, 5);
    }
}
