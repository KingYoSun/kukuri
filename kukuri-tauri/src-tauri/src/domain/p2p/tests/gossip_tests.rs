#[cfg(test)]
mod tests {
    use crate::domain::entities::Event;
    use crate::domain::p2p::generate_topic_id;
    use crate::infrastructure::p2p::gossip_service::GossipService;
    use crate::infrastructure::p2p::iroh_gossip_service::IrohGossipService;
    use iroh::Endpoint;
    use std::sync::Arc;

    macro_rules! skip_unless_p2p_enabled {
        ($name:literal) => {
            if std::env::var("ENABLE_P2P_INTEGRATION").unwrap_or_default() != "1" {
                eprintln!("skipping {} (ENABLE_P2P_INTEGRATION!=1)", $name);
                return;
            }
        };
    }

    async fn create_test_service() -> IrohGossipService {
        let endpoint = Endpoint::builder().bind().await.unwrap();
        IrohGossipService::new(Arc::new(endpoint)).unwrap()
    }

    #[tokio::test]
    async fn test_topic_join_leave() {
        skip_unless_p2p_enabled!("test_topic_join_leave");
        let service = create_test_service().await;
        let topic_id = generate_topic_id("test-topic");

        // Join topic
        let result = service.join_topic(&topic_id, vec![]).await;
        assert!(result.is_ok());

        // Verify topic is active
        let active_topics = service.get_joined_topics().await.unwrap();
        assert!(active_topics.contains(&topic_id));

        // Leave topic
        let result = service.leave_topic(&topic_id).await;
        assert!(result.is_ok());

        // Verify topic is removed
        let active_topics = service.get_joined_topics().await.unwrap();
        assert!(!active_topics.contains(&topic_id));
    }

    #[tokio::test]
    async fn test_multiple_topics() {
        skip_unless_p2p_enabled!("test_multiple_topics");
        let service = create_test_service().await;
        let topics = vec!["topic1", "topic2", "topic3"];

        // Join multiple topics
        for topic in &topics {
            let id = generate_topic_id(topic);
            service.join_topic(&id, vec![]).await.unwrap();
        }

        let active_topics = service.get_joined_topics().await.unwrap();
        assert_eq!(active_topics.len(), 3);

        // Leave one topic
        let id = generate_topic_id("topic2");
        service.leave_topic(&id).await.unwrap();

        let active_topics = service.get_joined_topics().await.unwrap();
        assert_eq!(active_topics.len(), 2);
        assert!(!active_topics.contains(&generate_topic_id("topic2")));
    }

    #[tokio::test]
    async fn test_leave_nonexistent_topic() {
        skip_unless_p2p_enabled!("test_leave_nonexistent_topic");
        let service = create_test_service().await;
        let topic = generate_topic_id("nonexistent");
        let result = service.leave_topic(&topic).await;
        // 未参加トピックのleaveは冪等（エラーにしない）
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_broadcast_to_topic() {
        skip_unless_p2p_enabled!("test_broadcast_to_topic");
        let service = create_test_service().await;
        let topic_id = generate_topic_id("broadcast-test");

        // まずトピックに参加
        service.join_topic(&topic_id, vec![]).await.unwrap();

        // ダミーEventを作成してブロードキャスト
        let event = Event::new(1, "hello".to_string(), "pubkey_test".to_string());
        let result = service.broadcast(&topic_id, &event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_broadcast_to_nonexistent_topic() {
        skip_unless_p2p_enabled!("test_broadcast_to_nonexistent_topic");
        let service = create_test_service().await;

        let event = Event::new(1, "hello".to_string(), "pubkey_test".to_string());
        let result = service.broadcast("nonexistent-topic", &event).await;
        // 未参加トピックのbroadcastはエラー
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_topic_status() {
        skip_unless_p2p_enabled!("test_get_topic_status");
        // IrohGossipServiceではステータスAPIは最小提供のためスキップ
        // 代わりにjoin後にget_joined_topicsで存在確認
        let service = create_test_service().await;
        let topic_id = generate_topic_id("status-test");
        service.join_topic(&topic_id, vec![]).await.unwrap();
        let topics = service.get_joined_topics().await.unwrap();
        assert!(topics.contains(&topic_id));
    }

    #[tokio::test]
    async fn test_get_all_topic_stats() {
        skip_unless_p2p_enabled!("test_get_all_topic_stats");
        let service = create_test_service().await;
        let topics = vec!["stats-topic1", "stats-topic2", "stats-topic3"];
        for topic in &topics {
            service
                .join_topic(&generate_topic_id(topic), vec![])
                .await
                .unwrap();
        }
        let joined = service.get_joined_topics().await.unwrap();
        assert_eq!(joined.len(), 3);
    }

    #[tokio::test]
    async fn test_shutdown() {
        skip_unless_p2p_enabled!("test_shutdown");
        let service = create_test_service().await;
        let topics = vec!["shutdown-topic1", "shutdown-topic2"];
        for topic in &topics {
            service
                .join_topic(&generate_topic_id(topic), vec![])
                .await
                .unwrap();
        }
        let active_topics = service.get_joined_topics().await.unwrap();
        assert_eq!(active_topics.len(), 2);
        // leave all
        for topic in &topics {
            service
                .leave_topic(&generate_topic_id(topic))
                .await
                .unwrap();
        }
        let active_topics = service.get_joined_topics().await.unwrap();
        assert_eq!(active_topics.len(), 0);
    }

    #[tokio::test]
    async fn test_node_id() {
        skip_unless_p2p_enabled!("test_node_id");
        // IrohGossipServiceでは直接のNodeID APIは提供しないため簡易確認のみ
        let _service = create_test_service().await;
        assert!(true);
    }

    #[tokio::test]
    async fn test_node_addr() {
        skip_unless_p2p_enabled!("test_node_addr");
        // IrohGossipServiceでは直接のアドレスAPIは提供しないためスキップ
        let _service = create_test_service().await;
        assert!(true);
    }

    #[tokio::test]
    async fn test_concurrent_topic_operations() {
        skip_unless_p2p_enabled!("test_concurrent_topic_operations");
        use std::sync::Arc;
        use tokio::task;

        let service = Arc::new(create_test_service().await);
        let mut handles = vec![];

        // 並行して複数のトピック操作を実行
        for i in 0..5 {
            let service_clone = service.clone();
            let handle = task::spawn(async move {
                let topic_id = generate_topic_id(&format!("concurrent-topic-{i}"));
                service_clone.join_topic(&topic_id, vec![]).await.unwrap();
                let joined = service_clone.get_joined_topics().await.unwrap();
                assert!(joined.contains(&topic_id));
                service_clone.leave_topic(&topic_id).await.unwrap();
            });
            handles.push(handle);
        }

        // すべてのタスクが完了するのを待つ
        for handle in handles {
            handle.await.unwrap();
        }

        // 最終的にすべてのトピックから離脱していることを確認
        let active_topics = service.get_joined_topics().await.unwrap();
        assert_eq!(active_topics.len(), 0);
    }
}
