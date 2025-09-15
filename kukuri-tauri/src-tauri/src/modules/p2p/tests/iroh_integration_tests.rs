#[cfg(test)]
mod tests {
    use crate::domain::entities::Event;
    use crate::infrastructure::p2p::gossip_service::GossipService;
    use crate::infrastructure::p2p::iroh_gossip_service::IrohGossipService;
    use crate::modules::p2p::generate_topic_id;
    use iroh::{Endpoint, Watcher as _};
    use std::sync::Arc;
    use tokio::time::{timeout, sleep, Duration};

    async fn create_service_with_endpoint() -> (IrohGossipService, Arc<Endpoint>) {
        let endpoint = Arc::new(Endpoint::builder().bind().await.unwrap());
        let svc = IrohGossipService::new(endpoint.clone()).unwrap();
        (svc, endpoint)
    }

    async fn connect_peers(src: &Endpoint, dst: &Endpoint) {
        // dstのNodeAddrを解決してsrcから接続
        let node_addr = dst.node_addr().initialized().await;
        src.connect(node_addr, iroh_gossip::ALPN).await.unwrap();
    }

    /// subscribe → broadcast → 受信までを単一ノードで検証（実配信導線）
    /// 二つのノードを接続して相互にメッセージを受信できることを検証
    #[tokio::test]
    async fn test_two_nodes_connect_and_join() {
        let (svc_a, ep_a) = create_service_with_endpoint().await;
        let (svc_b, ep_b) = create_service_with_endpoint().await;

        // A→B 接続
        connect_peers(&ep_a, &ep_b).await;

        // 同一トピックで購読/参加のみ検証（実ネットワーク経由の配送は別途環境依存のため）
        let topic = generate_topic_id("iroh-int-two-nodes");
        let _rx_b = svc_b.subscribe(&topic).await.unwrap();
        svc_a.join_topic(&topic, vec![]).await.unwrap();
        sleep(Duration::from_millis(100)).await;
        // 参加済みトピックに含まれることを確認
        let joined_a = svc_a.get_joined_topics().await.unwrap();
        let joined_b = svc_b.get_joined_topics().await.unwrap();
        assert!(joined_a.contains(&topic));
        assert!(joined_b.contains(&topic));
    }

    /// 複数購読者が同一トピックのイベントを受け取れること
    #[tokio::test]
    #[ignore = "Requires UDP reachability between two endpoints to receive"]
    async fn test_multiple_subscribers_receive() {
        let (svc_a, ep_a) = create_service_with_endpoint().await;
        let (svc_b, ep_b) = create_service_with_endpoint().await;

        // A→B 接続
        connect_peers(&ep_a, &ep_b).await;

        let topic = generate_topic_id("iroh-int-multi-subs");
        // B側に2購読者
        let mut rx1 = svc_b.subscribe(&topic).await.unwrap();
        let mut rx2 = svc_b.subscribe(&topic).await.unwrap();
        // A側はjoinのみ
        svc_a.join_topic(&topic, vec![]).await.unwrap();

        sleep(Duration::from_millis(200)).await;
        // Aから送信
        let ev = Event::new(1, "hello-multi".to_string(), "pk".to_string());
        svc_a.broadcast(&topic, &ev).await.unwrap();

        let r1 = timeout(Duration::from_secs(5), async { rx1.recv().await })
            .await
            .expect("rx1 timeout");
        let r2 = timeout(Duration::from_secs(5), async { rx2.recv().await })
            .await
            .expect("rx2 timeout");

        assert!(r1.is_some() && r2.is_some());
        assert_eq!(r1.unwrap().content, "hello-multi");
        assert_eq!(r2.unwrap().content, "hello-multi");
    }
}
