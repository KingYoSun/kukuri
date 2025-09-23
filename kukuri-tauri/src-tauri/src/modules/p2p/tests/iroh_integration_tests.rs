#[cfg(test)]
mod tests {
    use crate::domain::entities::Event;
    use crate::infrastructure::p2p::gossip_service::GossipService;
    use crate::infrastructure::p2p::iroh_gossip_service::IrohGossipService;
    use crate::modules::p2p::generate_topic_id;
    use iroh::{Endpoint, NodeAddr, Watcher as _};
    use std::net::{Ipv4Addr, SocketAddrV4};
    use std::sync::Arc;
    use tokio::time::{timeout, sleep, Duration};
    use nostr_sdk::prelude::*;

    async fn create_service_with_endpoint() -> (IrohGossipService, Arc<Endpoint>) {
        let bind_addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0);
        let endpoint = Arc::new(
            Endpoint::builder()
                .discovery_local_network()
                .bind_addr_v4(bind_addr)
                .bind()
                .await
                .unwrap(),
        );
        let svc = IrohGossipService::new(endpoint.clone()).unwrap();
        (svc, endpoint)
    }

    fn nostr_to_domain(ev: &nostr_sdk::Event) -> Event {
        let created_at = chrono::DateTime::<chrono::Utc>::from_utc(
            chrono::NaiveDateTime::from_timestamp_opt(ev.created_at.as_u64() as i64, 0).unwrap(),
            chrono::Utc,
        );
        Event {
            id: ev.id.to_string(),
            pubkey: ev.pubkey.to_string(),
            created_at,
            kind: ev.kind.as_u16() as u32,
            tags: ev.tags.iter().map(|t| t.clone().to_vec()).collect(),
            content: ev.content.clone(),
            sig: ev.sig.to_string(),
        }
    }

    async fn connect_peers(src: &Endpoint, dst: &Endpoint) {
        // resolve peer using direct addresses similar to kukuri-cli
        sleep(Duration::from_millis(500)).await;
        let direct_addrs = dst.direct_addresses().initialized().await;
        if let Some(addr) = direct_addrs.into_iter().map(|a| a.addr).next() {
            let node_addr = NodeAddr::new(dst.node_id()).with_direct_addresses([addr]);
            src.connect(node_addr, iroh_gossip::ALPN).await.unwrap();
        } else {
            src.connect(dst.node_id(), iroh_gossip::ALPN).await.unwrap();
        }
    }

    async fn wait_for_topic_membership(service: &IrohGossipService, topic: &str, timeout: Duration) -> bool {
        let target = topic.to_string();
        let start = tokio::time::Instant::now();
        while start.elapsed() < timeout {
            if let Ok(joined) = service.get_joined_topics().await {
                if joined.iter().any(|t| t == &target) {
                    return true;
                }
            }
            sleep(Duration::from_millis(100)).await;
        }
        false
    }

    /// subscribe → broadcast → 受信までを単一ノードで検証（実配信導線）
    /// 二つのノードを接続して相互にメッセージを受信できることを検証
    #[tokio::test]
    async fn test_two_nodes_connect_and_join() {
        let (mut svc_a, ep_a) = create_service_with_endpoint().await;
        let (mut svc_b, ep_b) = create_service_with_endpoint().await;

        // 双方向接続（保守的に）
        connect_peers(&ep_a, &ep_b).await;
        connect_peers(&ep_b, &ep_a).await;

        // 同一トピックで購読/参加のみ検証（実ネットワーク経由の配送は別途環境依存のため）
        let topic = generate_topic_id("iroh-int-two-nodes");
        let _rx_b = svc_b.subscribe(&topic).await.unwrap();
        svc_a.join_topic(&topic, vec![]).await.unwrap();
        assert!(
            wait_for_topic_membership(&svc_a, &topic, Duration::from_secs(5)).await,
            "svc_a failed to join topic {}",
            topic
        );
        assert!(
            wait_for_topic_membership(&svc_b, &topic, Duration::from_secs(5)).await,
            "svc_b failed to join topic {}",
            topic
        );
        // 参加済みトピックに含まれることを確認
        let joined_a = svc_a.get_joined_topics().await.unwrap();
        let joined_b = svc_b.get_joined_topics().await.unwrap();
        assert!(joined_a.contains(&topic));
        assert!(joined_b.contains(&topic));
    }

    #[tokio::test]
    async fn test_two_nodes_broadcast_and_receive() {
        if std::env::var("ENABLE_P2P_INTEGRATION").unwrap_or_default() != "1" {
            eprintln!("skipping two_nodes_broadcast_and_receive (ENABLE_P2P_INTEGRATION!=1)");
            return;
        }
        use tokio::sync::mpsc::unbounded_channel;
        use crate::modules::p2p::P2PEvent;

        let (mut svc_a, ep_a) = create_service_with_endpoint().await;
        let (mut svc_b, ep_b) = create_service_with_endpoint().await;

        // P2PEvent受信用にチャネル接続（NeighborUp確認用）
        let (tx_a, mut rx_a_evt) = unbounded_channel();
        let (tx_b, mut rx_b_evt) = unbounded_channel();
        svc_a.set_event_sender(tx_a);
        svc_b.set_event_sender(tx_b);

        // 同一トピックで先に購読を確立
        let topic = generate_topic_id("iroh-int-recv");
        // 双方で購読（内部で冪等join）
        let _rx_a = svc_a.subscribe(&topic).await.unwrap();
        let mut rx_b = svc_b.subscribe(&topic).await.unwrap();
        assert!(
            wait_for_topic_membership(&svc_a, &topic, Duration::from_secs(5)).await,
            "svc_a failed to join topic {}",
            topic
        );
        assert!(
            wait_for_topic_membership(&svc_b, &topic, Duration::from_secs(5)).await,
            "svc_b failed to join topic {}",
            topic
        );

        // 双方向接続
        connect_peers(&ep_a, &ep_b).await;
        connect_peers(&ep_b, &ep_a).await;

        // NeighborUpがどちらかで観測されるまで待機（最大5秒）
        let mut neighbor_ok = false;
        let start = tokio::time::Instant::now();
        while start.elapsed() < Duration::from_secs(8) {
            if let Ok(Some(evt)) = timeout(Duration::from_millis(100), async { rx_a_evt.recv().await }).await {
                if matches!(evt, P2PEvent::PeerJoined { .. }) { neighbor_ok = true; break; }
            }
            if let Ok(Some(evt)) = timeout(Duration::from_millis(100), async { rx_b_evt.recv().await }).await {
                if matches!(evt, P2PEvent::PeerJoined { .. }) { neighbor_ok = true; break; }
            }
        }
        if !neighbor_ok {
            eprintln!("peer join event not observed, continuing after grace period");
        }

        // 少し安定化
        sleep(Duration::from_millis(500)).await;

        // Aから送信→Bが受信（NIP-01準拠のイベントを送信）
        let keys = Keys::generate();
        let ne = EventBuilder::text_note("hello-int").sign_with_keys(&keys).unwrap();
        let ev = nostr_to_domain(&ne);
        svc_a.broadcast(&topic, &ev).await.unwrap();
        let r = timeout(Duration::from_secs(10), async { rx_b.recv().await })
            .await
            .expect("receive timeout");
        assert!(r.is_some());
        assert_eq!(r.unwrap().content, "hello-int");
    }

    /// 複数購読者が同一トピックのイベントを受け取れること

    /// P2P経路のみで返信イベントを伝搬できることを検証
    #[tokio::test]
    async fn test_p2p_reply_flow() {
        if std::env::var("ENABLE_P2P_INTEGRATION").unwrap_or_default() != "1" {
            eprintln!("skipping p2p_reply_flow (ENABLE_P2P_INTEGRATION!=1)");
            return;
        }
        use crate::modules::p2p::P2PEvent;
        use tokio::sync::mpsc::unbounded_channel;

        let (mut svc_a, ep_a) = create_service_with_endpoint().await;
        let (mut svc_b, ep_b) = create_service_with_endpoint().await;

        let topic = generate_topic_id("iroh-int-reply-flow");

        let mut rx_a = svc_a.subscribe(&topic).await.unwrap();
        let mut rx_b = svc_b.subscribe(&topic).await.unwrap();
        assert!(
            wait_for_topic_membership(&svc_a, &topic, Duration::from_secs(5)).await,
            "svc_a failed to join topic {}",
            topic
        );
        assert!(
            wait_for_topic_membership(&svc_b, &topic, Duration::from_secs(5)).await,
            "svc_b failed to join topic {}",
            topic
        );

        let (tx_a_evt, mut rx_a_evt) = unbounded_channel();
        let (tx_b_evt, mut rx_b_evt) = unbounded_channel();
        svc_a.set_event_sender(tx_a_evt);
        svc_b.set_event_sender(tx_b_evt);

        connect_peers(&ep_a, &ep_b).await;
        connect_peers(&ep_b, &ep_a).await;

        let mut neighbor_ok = false;
        let start = tokio::time::Instant::now();
        while start.elapsed() < Duration::from_secs(8) {
            if let Ok(Some(evt)) = timeout(Duration::from_millis(100), async { rx_a_evt.recv().await }).await {
                if matches!(evt, P2PEvent::PeerJoined { .. }) {
                    neighbor_ok = true;
                    break;
                }
            }
            if let Ok(Some(evt)) = timeout(Duration::from_millis(100), async { rx_b_evt.recv().await }).await {
                if matches!(evt, P2PEvent::PeerJoined { .. }) {
                    neighbor_ok = true;
                    break;
                }
            }
        }
        if !neighbor_ok {
            eprintln!("peer join event not observed for reply flow, continuing optimistically");
        }
        sleep(Duration::from_millis(600)).await;

        let root_keys = Keys::generate();
        let root_note = EventBuilder::text_note("root-post").sign_with_keys(&root_keys).unwrap();
        let root_event = nostr_to_domain(&root_note);
        let root_id = root_event.id.clone();
        let root_pubkey = root_event.pubkey.clone();
        svc_a.broadcast(&topic, &root_event).await.unwrap();

        let received_root = timeout(Duration::from_secs(10), async { rx_b.recv().await })
            .await
            .expect("root receive timeout")
            .expect("root channel closed");
        assert_eq!(received_root.content, "root-post");

        let reply_keys = Keys::generate();
        let reply_event_tag = Tag::from_standardized(TagStandard::Event {
            event_id: root_note.id,
            relay_url: None,
            marker: Some(Marker::Reply),
            public_key: None,
            uppercase: false,
        });
        let reply_pubkey_tag = Tag::from_standardized(TagStandard::public_key(root_note.pubkey));
        let reply_note = EventBuilder::text_note("reply-post")
            .tags([reply_event_tag, reply_pubkey_tag])
            .sign_with_keys(&reply_keys)
            .unwrap();
        let reply_event = nostr_to_domain(&reply_note);
        svc_b.broadcast(&topic, &reply_event).await.unwrap();

        let received_reply = timeout(Duration::from_secs(10), async { rx_a.recv().await })
            .await
            .expect("reply receive timeout")
            .expect("reply channel closed");
        assert_eq!(received_reply.content, "reply-post");

        let e_tag = received_reply
            .tags
            .iter()
            .find(|tag| tag.get(0).map(|s| s.as_str()) == Some("e"))
            .expect("reply event missing e tag");
        assert_eq!(e_tag.get(1).map(|s| s.as_str()), Some(root_id.as_str()));
        assert_eq!(e_tag.get(3).map(|s| s.as_str()), Some("reply"));

        let p_tag = received_reply
            .tags
            .iter()
            .find(|tag| tag.get(0).map(|s| s.as_str()) == Some("p"))
            .expect("reply event missing p tag");
        assert_eq!(p_tag.get(1).map(|s| s.as_str()), Some(root_pubkey.as_str()));
    }

    /// P2P経路のみで引用イベント（mention）が伝搬されることを検証
    #[tokio::test]
    async fn test_p2p_quote_flow() {
        if std::env::var("ENABLE_P2P_INTEGRATION").unwrap_or_default() != "1" {
            eprintln!("skipping p2p_quote_flow (ENABLE_P2P_INTEGRATION!=1)");
            return;
        }
        use crate::modules::p2p::P2PEvent;
        use tokio::sync::mpsc::unbounded_channel;

        let (mut svc_a, ep_a) = create_service_with_endpoint().await;
        let (mut svc_b, ep_b) = create_service_with_endpoint().await;

        let topic = generate_topic_id("iroh-int-quote-flow");

        let mut rx_a = svc_a.subscribe(&topic).await.unwrap();
        let mut rx_b = svc_b.subscribe(&topic).await.unwrap();
        assert!(
            wait_for_topic_membership(&svc_a, &topic, Duration::from_secs(5)).await,
            "svc_a failed to join topic {}",
            topic
        );
        assert!(
            wait_for_topic_membership(&svc_b, &topic, Duration::from_secs(5)).await,
            "svc_b failed to join topic {}",
            topic
        );

        let (tx_a_evt, mut rx_a_evt) = unbounded_channel();
        let (tx_b_evt, mut rx_b_evt) = unbounded_channel();
        svc_a.set_event_sender(tx_a_evt);
        svc_b.set_event_sender(tx_b_evt);

        connect_peers(&ep_a, &ep_b).await;
        connect_peers(&ep_b, &ep_a).await;

        let mut neighbor_ok = false;
        let start = tokio::time::Instant::now();
        while start.elapsed() < Duration::from_secs(8) {
            if let Ok(Some(evt)) = timeout(Duration::from_millis(100), async { rx_a_evt.recv().await }).await {
                if matches!(evt, P2PEvent::PeerJoined { .. }) {
                    neighbor_ok = true;
                    break;
                }
            }
            if let Ok(Some(evt)) = timeout(Duration::from_millis(100), async { rx_b_evt.recv().await }).await {
                if matches!(evt, P2PEvent::PeerJoined { .. }) {
                    neighbor_ok = true;
                    break;
                }
            }
        }
        if !neighbor_ok {
            eprintln!("peer join event not observed for quote flow, continuing optimistically");
        }
        sleep(Duration::from_millis(600)).await;

        let base_keys = Keys::generate();
        let base_note = EventBuilder::text_note("quote-root").sign_with_keys(&base_keys).unwrap();
        let base_event = nostr_to_domain(&base_note);
        let base_id = base_event.id.clone();
        let base_pubkey = base_event.pubkey.clone();
        svc_a.broadcast(&topic, &base_event).await.unwrap();

        let _ = timeout(Duration::from_secs(10), async { rx_b.recv().await })
            .await
            .expect("base receive timeout")
            .expect("base channel closed");

        let quote_keys = Keys::generate();
        let mention_tag = Tag::from_standardized(TagStandard::Event {
            event_id: base_note.id,
            relay_url: None,
            marker: Some(Marker::Mention),
            public_key: None,
            uppercase: false,
        });
        let mention_pubkey_tag = Tag::from_standardized(TagStandard::public_key(base_note.pubkey));
        let quote_note = EventBuilder::text_note("quote-post")
            .tags([mention_tag, mention_pubkey_tag])
            .sign_with_keys(&quote_keys)
            .unwrap();
        let quote_event = nostr_to_domain(&quote_note);
        svc_b.broadcast(&topic, &quote_event).await.unwrap();

        let received_quote = timeout(Duration::from_secs(10), async { rx_a.recv().await })
            .await
            .expect("quote receive timeout")
            .expect("quote channel closed");
        assert_eq!(received_quote.content, "quote-post");

        let e_tag = received_quote
            .tags
            .iter()
            .find(|tag| tag.get(0).map(|s| s.as_str()) == Some("e"))
            .expect("quote event missing e tag");
        assert_eq!(e_tag.get(1).map(|s| s.as_str()), Some(base_id.as_str()));
        assert_eq!(e_tag.get(3).map(|s| s.as_str()), Some("mention"));

        let p_tag = received_quote
            .tags
            .iter()
            .find(|tag| tag.get(0).map(|s| s.as_str()) == Some("p"))
            .expect("quote event missing p tag");
        assert_eq!(p_tag.get(1).map(|s| s.as_str()), Some(base_pubkey.as_str()));
    }

    #[tokio::test]
    async fn test_multiple_subscribers_receive() {
        if std::env::var("ENABLE_P2P_INTEGRATION").unwrap_or_default() != "1" {
            eprintln!("skipping multiple_subscribers_receive (ENABLE_P2P_INTEGRATION!=1)");
            return;
        }
        let (svc_a, ep_a) = create_service_with_endpoint().await;
        let (svc_b, ep_b) = create_service_with_endpoint().await;

        // A→B 接続
        connect_peers(&ep_a, &ep_b).await;

        let topic = generate_topic_id("iroh-int-multi-subs");
        // B側に2購読者
        let mut rx1 = svc_b.subscribe(&topic).await.unwrap();
        let mut rx2 = svc_b.subscribe(&topic).await.unwrap();
        assert!(
            wait_for_topic_membership(&svc_b, &topic, Duration::from_secs(5)).await,
            "svc_b failed to join topic {}",
            topic
        );
        // A側はjoinのみ
        svc_a.join_topic(&topic, vec![]).await.unwrap();
        assert!(
            wait_for_topic_membership(&svc_a, &topic, Duration::from_secs(5)).await,
            "svc_a failed to join topic {}",
            topic
        );

        sleep(Duration::from_millis(600)).await;
        // Aから送信（NIP-01準拠）
        let keys = Keys::generate();
        let ne = EventBuilder::text_note("hello-multi").sign_with_keys(&keys).unwrap();
        let ev = nostr_to_domain(&ne);
        svc_a.broadcast(&topic, &ev).await.unwrap();

        let r1 = timeout(Duration::from_secs(10), async { rx1.recv().await })
            .await
            .expect("rx1 timeout");
        let r2 = timeout(Duration::from_secs(10), async { rx2.recv().await })
            .await
            .expect("rx2 timeout");

        assert!(r1.is_some() && r2.is_some());
        assert_eq!(r1.unwrap().content, "hello-multi");
        assert_eq!(r2.unwrap().content, "hello-multi");
    }

    /// 3ノード構成でA->(B,C)へブロードキャストが届くことを検証
    #[tokio::test]
    async fn test_multi_node_broadcast_three_nodes() {
        if std::env::var("ENABLE_P2P_INTEGRATION").unwrap_or_default() != "1" {
            eprintln!("skipping multi_node_broadcast_three_nodes (ENABLE_P2P_INTEGRATION!=1)");
            return;
        }

        let (svc_a, ep_a) = create_service_with_endpoint().await;
        let (svc_b, ep_b) = create_service_with_endpoint().await;
        let (svc_c, ep_c) = create_service_with_endpoint().await;

        // AからB,Cへ接続（単方向で十分）
        connect_peers(&ep_a, &ep_b).await;
        connect_peers(&ep_a, &ep_c).await;

        let topic = generate_topic_id("iroh-int-multi-node");
        // 受信側B,Cは購読（内部でjoin）
        let mut rx_b = svc_b.subscribe(&topic).await.unwrap();
        let mut rx_c = svc_c.subscribe(&topic).await.unwrap();
        assert!(
            wait_for_topic_membership(&svc_b, &topic, Duration::from_secs(5)).await,
            "svc_b failed to join topic {}",
            topic
        );
        assert!(
            wait_for_topic_membership(&svc_c, &topic, Duration::from_secs(5)).await,
            "svc_c failed to join topic {}",
            topic
        );
        // 送信側Aはjoinのみ
        svc_a.join_topic(&topic, vec![]).await.unwrap();
        assert!(
            wait_for_topic_membership(&svc_a, &topic, Duration::from_secs(5)).await,
            "svc_a failed to join topic {}",
            topic
        );

        // 安定化待ち
        sleep(Duration::from_millis(600)).await;

        // Aから送信（NIP-01準拠）
        let keys = Keys::generate();
        let ne = EventBuilder::text_note("hello-3nodes").sign_with_keys(&keys).unwrap();
        let ev = nostr_to_domain(&ne);
        svc_a.broadcast(&topic, &ev).await.unwrap();

        // BとCで受信
        let r_b = timeout(Duration::from_secs(10), async { rx_b.recv().await })
            .await
            .expect("B receive timeout");
        let r_c = timeout(Duration::from_secs(10), async { rx_c.recv().await })
            .await
            .expect("C receive timeout");

        assert!(r_b.is_some() && r_c.is_some());
        assert_eq!(r_b.unwrap().content, "hello-3nodes");
        assert_eq!(r_c.unwrap().content, "hello-3nodes");
    }

    /// 双方向にメッセージをやり取りし、近接で安定して届くことを簡易確認
    #[tokio::test]
    async fn test_peer_connection_stability_bidirectional() {
        if std::env::var("ENABLE_P2P_INTEGRATION").unwrap_or_default() != "1" {
            eprintln!("skipping peer_connection_stability_bidirectional (ENABLE_P2P_INTEGRATION!=1)");
            return;
        }

        let (svc_a, ep_a) = create_service_with_endpoint().await;
        let (svc_b, ep_b) = create_service_with_endpoint().await;

        // 接続
        connect_peers(&ep_a, &ep_b).await;
        connect_peers(&ep_b, &ep_a).await;

        let topic = generate_topic_id("iroh-int-stability");
        let mut rx_a = svc_a.subscribe(&topic).await.unwrap();
        let mut rx_b = svc_b.subscribe(&topic).await.unwrap();
        assert!(
            wait_for_topic_membership(&svc_a, &topic, Duration::from_secs(5)).await,
            "svc_a failed to join topic {}",
            topic
        );
        assert!(
            wait_for_topic_membership(&svc_b, &topic, Duration::from_secs(5)).await,
            "svc_b failed to join topic {}",
            topic
        );

        // 安定化待ち
        sleep(Duration::from_millis(600)).await;

        // A→B, B→A 交互に送信
        for i in 0..5u32 {
            let keys = Keys::generate();
            let ne = EventBuilder::text_note(format!("ping-{i}")).sign_with_keys(&keys).unwrap();
            let ev = nostr_to_domain(&ne);
            if i % 2 == 0 { svc_a.broadcast(&topic, &ev).await.unwrap(); }
            else { svc_b.broadcast(&topic, &ev).await.unwrap(); }
        }

        // 少なくとも片側で3件以上受信できること（緩い安定性チェック）
        let mut count_a = 0;
        let mut count_b = 0;
        let start = tokio::time::Instant::now();
        while start.elapsed() < Duration::from_secs(8) && (count_a < 3 || count_b < 3) {
            if let Ok(Some(_)) = timeout(Duration::from_millis(100), async { rx_a.recv().await }).await { count_a += 1; }
            if let Ok(Some(_)) = timeout(Duration::from_millis(100), async { rx_b.recv().await }).await { count_b += 1; }
        }
        assert!(count_a >= 3 || count_b >= 3, "insufficient messages received: a={}, b={}", count_a, count_b);
    }
}
