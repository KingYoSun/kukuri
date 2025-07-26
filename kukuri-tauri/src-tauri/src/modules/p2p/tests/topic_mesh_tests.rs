#[cfg(test)]
mod tests {
    use crate::modules::p2p::topic_mesh::*;
    use crate::modules::p2p::message::{GossipMessage, MessageType};
    
    fn create_test_mesh() -> TopicMesh {
        TopicMesh::new("test-topic".to_string())
    }
    
    fn create_test_message(id: u8) -> GossipMessage {
        let mut message = GossipMessage::new(
            MessageType::NostrEvent,
            vec![id],
            vec![id; 32],
        );
        // 一意のIDを設定
        message.id[0] = id;
        message
    }
    
    #[tokio::test]
    async fn test_topic_mesh_creation() {
        let mesh = create_test_mesh();
        let stats = mesh.get_stats().await;
        
        assert_eq!(stats.peer_count, 0);
        assert_eq!(stats.message_count, 0);
        assert_eq!(stats.last_activity, 0);
    }
    
    #[tokio::test]
    async fn test_message_handling() {
        let mesh = create_test_mesh();
        let message = create_test_message(1);
        
        // メッセージ処理
        let result = mesh.handle_message(message.clone()).await;
        assert!(result.is_ok());
        
        // 統計情報の確認
        let stats = mesh.get_stats().await;
        assert_eq!(stats.message_count, 1);
        assert_eq!(stats.peer_count, 1);
        assert!(stats.last_activity > 0);
    }
    
    #[tokio::test]
    async fn test_duplicate_detection() {
        let mesh = create_test_mesh();
        let message = create_test_message(2);
        
        // 最初のメッセージは重複ではない
        assert!(!mesh.is_duplicate(&message.id).await);
        
        // メッセージを処理
        mesh.handle_message(message.clone()).await.unwrap();
        
        // 同じメッセージは重複として検出される
        assert!(mesh.is_duplicate(&message.id).await);
    }
    
    #[tokio::test]
    async fn test_peer_management() {
        let mesh = create_test_mesh();
        let peer1 = vec![1; 32];
        let peer2 = vec![2; 32];
        
        // ピアの追加
        mesh.update_peer_status(peer1.clone(), true).await;
        mesh.update_peer_status(peer2.clone(), true).await;
        
        let peers = mesh.get_peers().await;
        assert_eq!(peers.len(), 2);
        assert!(peers.contains(&peer1));
        assert!(peers.contains(&peer2));
        
        // ピアの削除
        mesh.update_peer_status(peer1.clone(), false).await;
        
        let peers = mesh.get_peers().await;
        assert_eq!(peers.len(), 1);
        assert!(!peers.contains(&peer1));
        assert!(peers.contains(&peer2));
    }
    
    #[tokio::test]
    async fn test_recent_messages() {
        let mesh = create_test_mesh();
        
        // 複数のメッセージを追加
        for i in 0..5 {
            let message = create_test_message(i);
            mesh.handle_message(message).await.unwrap();
        }
        
        // 最新のメッセージを取得
        let recent = mesh.get_recent_messages(3).await;
        assert_eq!(recent.len(), 3);
        
        // タイムスタンプが降順であることを確認
        for i in 0..recent.len() - 1 {
            assert!(recent[i].timestamp >= recent[i + 1].timestamp);
        }
    }
    
    #[tokio::test]
    async fn test_cache_clear() {
        let mesh = create_test_mesh();
        
        // メッセージを追加
        for i in 0..3 {
            let message = create_test_message(i);
            mesh.handle_message(message).await.unwrap();
        }
        
        let stats = mesh.get_stats().await;
        assert_eq!(stats.message_count, 3);
        
        // キャッシュをクリア
        mesh.clear_cache().await;
        
        let stats = mesh.get_stats().await;
        assert_eq!(stats.message_count, 0);
    }
    
    #[tokio::test]
    async fn test_cache_limit() {
        let mesh = create_test_mesh();
        
        // キャッシュ制限（1000）を超えるメッセージを追加しようとする
        // 実際にはLRUキャッシュが古いメッセージを削除する
        for i in 0..1100 {
            let mut message = create_test_message((i % 256) as u8);
            // より一意なIDを設定
            message.id[0] = (i % 256) as u8;
            message.id[1] = ((i >> 8) % 256) as u8;
            mesh.handle_message(message).await.unwrap();
        }
        
        let stats = mesh.get_stats().await;
        // キャッシュサイズは1000以下
        assert!(stats.message_count <= 1000);
    }
}