#[cfg(test)]
mod tests {
    use crate::modules::p2p::*;
    use iroh::SecretKey;
    use tokio::sync::mpsc;

    async fn create_test_manager() -> GossipManager {
        let iroh_secret_key = SecretKey::generate(rand::thread_rng());
        let secp_secret_key = secp256k1::SecretKey::new(&mut rand::thread_rng());
        let (event_tx, _) = mpsc::unbounded_channel();
        GossipManager::new(iroh_secret_key, secp_secret_key, event_tx)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_topic_join_leave() {
        let manager = create_test_manager().await;
        let topic_id = "test-topic";

        // Join topic
        let result = manager.join_topic(topic_id, vec![]).await;
        assert!(result.is_ok());

        // Verify topic is active
        let active_topics = manager.active_topics().await;
        assert!(active_topics.contains(&topic_id.to_string()));

        // Leave topic
        let result = manager.leave_topic(topic_id).await;
        assert!(result.is_ok());

        // Verify topic is removed
        let active_topics = manager.active_topics().await;
        assert!(!active_topics.contains(&topic_id.to_string()));
    }

    #[tokio::test]
    async fn test_multiple_topics() {
        let manager = create_test_manager().await;
        let topics = vec!["topic1", "topic2", "topic3"];

        // Join multiple topics
        for topic in &topics {
            manager.join_topic(topic, vec![]).await.unwrap();
        }

        let active_topics = manager.active_topics().await;
        assert_eq!(active_topics.len(), 3);

        // Leave one topic
        manager.leave_topic("topic2").await.unwrap();

        let active_topics = manager.active_topics().await;
        assert_eq!(active_topics.len(), 2);
        assert!(!active_topics.contains(&"topic2".to_string()));
    }

    #[tokio::test]
    async fn test_leave_nonexistent_topic() {
        let manager = create_test_manager().await;

        let result = manager.leave_topic("nonexistent").await;
        assert!(result.is_err());

        match result.unwrap_err() {
            P2PError::TopicNotFound(topic) => assert_eq!(topic, "nonexistent"),
            _ => panic!("Expected TopicNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_broadcast_to_topic() {
        use crate::modules::p2p::message::{GossipMessage, MessageType};

        let manager = create_test_manager().await;
        let topic_id = "broadcast-test";

        // まずトピックに参加
        manager.join_topic(topic_id, vec![]).await.unwrap();

        // メッセージを作成してブロードキャスト
        let message = GossipMessage::new(MessageType::NostrEvent, vec![1, 2, 3], vec![0; 33]);

        let result = manager.broadcast(topic_id, message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_broadcast_to_nonexistent_topic() {
        use crate::modules::p2p::message::{GossipMessage, MessageType};

        let manager = create_test_manager().await;

        let message = GossipMessage::new(MessageType::NostrEvent, vec![1, 2, 3], vec![0; 33]);

        let result = manager.broadcast("nonexistent", message).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            P2PError::TopicNotFound(topic) => assert_eq!(topic, "nonexistent"),
            _ => panic!("Expected TopicNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_get_topic_status() {
        let manager = create_test_manager().await;
        let topic_id = "status-test";

        // トピックに参加する前
        let status = manager.get_topic_status(topic_id).await;
        assert!(status.is_none());

        // トピックに参加
        manager.join_topic(topic_id, vec![]).await.unwrap();

        // ステータスを取得
        let status = manager.get_topic_status(topic_id).await;
        assert!(status.is_some());

        let stats = status.unwrap();
        assert_eq!(stats.peer_count, 0);
        assert_eq!(stats.message_count, 0);
        assert_eq!(stats.last_activity, 0);
    }

    #[tokio::test]
    async fn test_get_all_topic_stats() {
        let manager = create_test_manager().await;
        let topics = vec!["stats-topic1", "stats-topic2", "stats-topic3"];

        // 複数のトピックに参加
        for topic in &topics {
            manager.join_topic(topic, vec![]).await.unwrap();
        }

        // 全トピックの統計情報を取得
        let all_stats = manager.get_all_topic_stats().await;
        assert_eq!(all_stats.len(), 3);

        // 各トピックの統計情報が含まれているか確認
        let topic_ids: Vec<String> = all_stats.iter().map(|(id, _)| id.clone()).collect();
        for topic in &topics {
            assert!(topic_ids.contains(&topic.to_string()));
        }
    }

    #[tokio::test]
    async fn test_shutdown() {
        let manager = create_test_manager().await;
        let topics = vec!["shutdown-topic1", "shutdown-topic2"];

        // 複数のトピックに参加
        for topic in &topics {
            manager.join_topic(topic, vec![]).await.unwrap();
        }

        // アクティブなトピックがあることを確認
        let active_topics = manager.active_topics().await;
        assert_eq!(active_topics.len(), 2);

        // シャットダウン
        let result = manager.shutdown().await;
        assert!(result.is_ok());

        // すべてのトピックから離脱していることを確認
        let active_topics = manager.active_topics().await;
        assert_eq!(active_topics.len(), 0);
    }

    #[tokio::test]
    async fn test_node_id() {
        let manager = create_test_manager().await;
        let node_id = manager.node_id();

        // NodeIDが空でないことを確認
        assert!(!node_id.is_empty());
        // NodeIDは base58エンコードされた文字列
        assert!(node_id.chars().all(|c| c.is_alphanumeric()));
    }

    #[tokio::test]
    async fn test_node_addr() {
        let manager = create_test_manager().await;
        let result = manager.node_addr().await;

        // ノードアドレスが取得できることを確認
        assert!(result.is_ok());

        // アドレスは環境により空の場合もあるため、エラーにならないことだけを確認
        let _addrs = result.unwrap();
    }

    #[tokio::test]
    async fn test_concurrent_topic_operations() {
        use std::sync::Arc;
        use tokio::task;

        let manager = Arc::new(create_test_manager().await);
        let mut handles = vec![];

        // 並行して複数のトピック操作を実行
        for i in 0..5 {
            let manager_clone = manager.clone();
            let handle = task::spawn(async move {
                let topic_id = format!("concurrent-topic-{i}");

                // Join
                manager_clone.join_topic(&topic_id, vec![]).await.unwrap();

                // Get status
                let status = manager_clone.get_topic_status(&topic_id).await;
                assert!(status.is_some());

                // Leave
                manager_clone.leave_topic(&topic_id).await.unwrap();
            });
            handles.push(handle);
        }

        // すべてのタスクが完了するのを待つ
        for handle in handles {
            handle.await.unwrap();
        }

        // 最終的にすべてのトピックから離脱していることを確認
        let active_topics = manager.active_topics().await;
        assert_eq!(active_topics.len(), 0);
    }
}
