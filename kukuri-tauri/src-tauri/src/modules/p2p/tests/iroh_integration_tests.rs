#[cfg(test)]
mod tests {
    use crate::domain::entities::Event;
    use crate::infrastructure::p2p::gossip_service::GossipService;
    use crate::infrastructure::p2p::iroh_gossip_service::IrohGossipService;
    use crate::modules::p2p::generate_topic_id;
    use iroh::Endpoint;
    use std::sync::Arc;
    use tokio::time::{timeout, Duration};

    async fn create_service() -> IrohGossipService {
        let endpoint = Endpoint::builder().bind().await.unwrap();
        IrohGossipService::new(Arc::new(endpoint)).unwrap()
    }

    /// subscribe → broadcast → 受信までを単一ノードで検証（実配信導線）
    #[tokio::test]
    #[ignore = "iroh-gossip does not echo self-broadcast locally; needs multi-node wiring"]
    async fn test_subscribe_and_receive_local_broadcast() {
        let service = create_service().await;
        let topic_id = generate_topic_id("iroh-int-local");

        // 購読を開始（内部で冪等join）
        let mut rx = service.subscribe(&topic_id).await.unwrap();

        // イベントを作成してブロードキャスト
        let event = Event::new(1, "hello-integration".to_string(), "pubkey_test".to_string());
        service.broadcast(&topic_id, &event).await.unwrap();

        // 受信を待機
        let received = timeout(Duration::from_secs(2), async move { rx.recv().await })
            .await
            .expect("timeout waiting for event");

        assert!(received.is_some());
        let e = received.unwrap();
        assert_eq!(e.content, "hello-integration");
        assert_eq!(e.kind, 1);
        assert_eq!(e.pubkey, "pubkey_test");
    }

    /// 複数購読者が同一トピックのイベントを受け取れること
    #[tokio::test]
    #[ignore = "requires multi-node or local echo; to be enabled after wiring"]
    async fn test_multiple_subscribers_receive() {
        let service = create_service().await;
        let topic_id = generate_topic_id("iroh-int-multi-subs");

        let mut rx1 = service.subscribe(&topic_id).await.unwrap();
        let mut rx2 = service.subscribe(&topic_id).await.unwrap();

        let event = Event::new(1, "multi".to_string(), "pk".to_string());
        service.broadcast(&topic_id, &event).await.unwrap();

        let r1 = timeout(Duration::from_secs(2), async { rx1.recv().await })
            .await
            .expect("rx1 timeout");
        let r2 = timeout(Duration::from_secs(2), async { rx2.recv().await })
            .await
            .expect("rx2 timeout");

        assert!(r1.is_some() && r2.is_some());
        assert_eq!(r1.unwrap().content, "multi");
        assert_eq!(r2.unwrap().content, "multi");
    }
}
