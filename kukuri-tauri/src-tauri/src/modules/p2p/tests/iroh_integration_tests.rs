#[cfg(test)]
mod tests {
    use crate::domain::entities::Event;
    use crate::infrastructure::p2p::gossip_service::GossipService;
    use crate::infrastructure::p2p::iroh_gossip_service::IrohGossipService;
    use crate::infrastructure::p2p::utils::parse_peer_hint;
    use crate::modules::p2p::P2PEvent;
    use crate::modules::p2p::generate_topic_id;
    use iroh::{Endpoint, NodeAddr};
    use nostr_sdk::prelude::*;
    use std::net::{Ipv4Addr, SocketAddrV4};
    use std::sync::Arc;
    use tokio::time::{Duration, sleep, timeout};

    macro_rules! log_step {
        ($($arg:tt)*) => {{
            eprintln!("[iroh_integration_tests] {}", format!($($arg)*));
        }};
    }

    struct BootstrapContext {
        hints: Vec<String>,
        node_addrs: Vec<NodeAddr>,
    }

    async fn create_service(ctx: &BootstrapContext) -> IrohGossipService {
        let bind_addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0);
        log_step!(
            "binding endpoint on {} and enabling DHT discovery (bootstrap hints: {})",
            bind_addr,
            ctx.hints.join(", ")
        );
        let endpoint = Arc::new(
            Endpoint::builder()
                .discovery_dht()
                .bind_addr_v4(bind_addr)
                .bind()
                .await
                .unwrap(),
        );
        endpoint.online().await;
        for addr in &ctx.node_addrs {
            log_step!("adding bootstrap node addr {}", addr.node_id);
            let _ = endpoint.add_node_addr_with_source(addr.clone(), "integration-bootstrap");
            if let Err(err) = endpoint.connect(addr.clone(), iroh_gossip::ALPN).await {
                eprintln!("failed to connect to bootstrap {}: {:?}", addr.node_id, err);
            } else {
                log_step!("connected to bootstrap {}", addr.node_id);
            }
        }
        log_step!("endpoint ready, building gossip service");
        sleep(Duration::from_millis(200)).await;
        IrohGossipService::new(endpoint).unwrap()
    }

    fn nostr_to_domain(ev: &nostr_sdk::Event) -> Event {
        let created_at =
            chrono::DateTime::<chrono::Utc>::from_timestamp(ev.created_at.as_u64() as i64, 0)
                .unwrap();
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

    fn bootstrap_context(test_name: &str) -> Option<BootstrapContext> {
        if std::env::var("ENABLE_P2P_INTEGRATION").unwrap_or_default() != "1" {
            eprintln!("skipping {test_name} (ENABLE_P2P_INTEGRATION!=1)");
            return None;
        }
        let raw = std::env::var("KUKURI_BOOTSTRAP_PEERS").unwrap_or_default();
        if raw.trim().is_empty() {
            eprintln!("skipping {test_name} (KUKURI_BOOTSTRAP_PEERS not set)");
            return None;
        }
        let mut hints = Vec::new();
        let mut addrs = Vec::new();
        for entry in raw.split(',') {
            let trimmed = entry.trim();
            if trimmed.is_empty() {
                continue;
            }
            hints.push(trimmed.to_string());
            match parse_peer_hint(trimmed) {
                Ok(parsed) => {
                    if let Some(addr) = parsed.node_addr {
                        addrs.push(addr);
                    } else {
                        eprintln!("bootstrap peer '{trimmed}' missing address; skipping");
                    }
                }
                Err(err) => {
                    eprintln!("failed to parse bootstrap peer '{trimmed}': {err:?}");
                    return None;
                }
            }
        }
        if addrs.is_empty() {
            eprintln!("skipping {test_name} (no usable bootstrap node addresses)");
            return None;
        }
        log_step!(
            "test {} using bootstrap peers: {}",
            test_name,
            hints.join(", ")
        );
        Some(BootstrapContext {
            hints,
            node_addrs: addrs,
        })
    }

    async fn wait_for_topic_membership(
        service: &IrohGossipService,
        topic: &str,
        timeout: Duration,
    ) -> bool {
        let target = topic.to_string();
        let start = tokio::time::Instant::now();
        while start.elapsed() < timeout {
            log_step!(
                "checking joined topics for {} (elapsed {:?}/{:?})",
                topic,
                start.elapsed(),
                timeout
            );
            if let Ok(joined) = service.get_joined_topics().await {
                log_step!("currently joined topics: {:?}", joined);
                if joined.iter().any(|t| t == &target) {
                    return true;
                }
            }
            sleep(Duration::from_millis(100)).await;
        }
        false
    }

    async fn wait_for_peer_join_event(
        receivers: &mut [&mut tokio::sync::mpsc::UnboundedReceiver<P2PEvent>],
        max_wait: Duration,
    ) -> bool {
        log_step!(
            "waiting up to {:?} for peer join events across {} receivers",
            max_wait,
            receivers.len()
        );
        let start = tokio::time::Instant::now();
        while start.elapsed() < max_wait {
            for rx in receivers.iter_mut() {
                if let Ok(Some(evt)) =
                    timeout(Duration::from_millis(150), async { rx.recv().await }).await
                {
                    if matches!(evt, P2PEvent::PeerJoined { .. }) {
                        log_step!("received PeerJoined event after {:?}", start.elapsed());
                        return true;
                    }
                }
            }
        }
        log_step!(
            "timed out waiting for peer join events after {:?}",
            max_wait
        );
        false
    }

    fn build_peer_hints(
        base: &[String],
        local_hints: &[Option<String>],
        self_idx: usize,
    ) -> Vec<String> {
        let mut result = base.to_vec();
        for (idx, hint) in local_hints.iter().enumerate() {
            if idx == self_idx {
                continue;
            }
            if let Some(h) = hint {
                if !result.contains(h) {
                    result.push(h.clone());
                }
            }
        }
        result
    }

    /// subscribe → broadcast → 受信までを単一ノードで検証（実配信導線）
    /// 二つのノードを接続して相互にメッセージを受信できることを検証
    #[tokio::test]
    async fn test_two_nodes_connect_and_join() {
        let Some(ctx) = bootstrap_context("test_two_nodes_connect_and_join") else {
            return;
        };
        log_step!("--- test_two_nodes_connect_and_join start ---");
        let svc_a = create_service(&ctx).await;
        let svc_b = create_service(&ctx).await;

        // 同一トピックで購読/参加のみ検証（実ネットワーク経由の配送は別途環境依存のため）
        let topic = generate_topic_id("iroh-int-two-nodes");
        log_step!("joining topic {} on both services", topic);
        let local_hints = vec![svc_a.local_peer_hint(), svc_b.local_peer_hint()];
        let hints_a = build_peer_hints(&ctx.hints, &local_hints, 0);
        let hints_b = build_peer_hints(&ctx.hints, &local_hints, 1);
        svc_a.join_topic(&topic, hints_a).await.unwrap();
        log_step!("svc_a joined topic {}", topic);
        svc_b.join_topic(&topic, hints_b).await.unwrap();
        log_step!("svc_b joined topic {}", topic);
        let _rx_b = svc_b.subscribe(&topic).await.unwrap();
        assert!(
            wait_for_topic_membership(&svc_a, &topic, Duration::from_secs(15)).await,
            "svc_a failed to join topic {topic}"
        );
        assert!(
            wait_for_topic_membership(&svc_b, &topic, Duration::from_secs(15)).await,
            "svc_b failed to join topic {topic}"
        );
        log_step!("both services joined topic {}", topic);
        // 参加済みトピックに含まれることを確認
        let joined_a = svc_a.get_joined_topics().await.unwrap();
        let joined_b = svc_b.get_joined_topics().await.unwrap();
        assert!(joined_a.contains(&topic));
        assert!(joined_b.contains(&topic));
        log_step!("--- test_two_nodes_connect_and_join end ---");
    }

    #[tokio::test]
    async fn test_two_nodes_broadcast_and_receive() {
        let Some(ctx) = bootstrap_context("test_two_nodes_broadcast_and_receive") else {
            return;
        };
        log_step!("--- test_two_nodes_broadcast_and_receive start ---");
        use tokio::sync::mpsc::unbounded_channel;

        let mut svc_a = create_service(&ctx).await;
        let mut svc_b = create_service(&ctx).await;

        // P2PEvent受信用にチャネル接続（NeighborUp確認用）
        let (tx_a, mut rx_a_evt) = unbounded_channel();
        let (tx_b, mut rx_b_evt) = unbounded_channel();
        svc_a.set_event_sender(tx_a);
        svc_b.set_event_sender(tx_b);

        // 同一トピックで先に購読を確立
        let topic = generate_topic_id("iroh-int-recv");
        let local_hints = vec![svc_a.local_peer_hint(), svc_b.local_peer_hint()];
        let hints_a = build_peer_hints(&ctx.hints, &local_hints, 0);
        let hints_b = build_peer_hints(&ctx.hints, &local_hints, 1);
        svc_a.join_topic(&topic, hints_a).await.unwrap();
        log_step!("svc_a joined {}", topic);
        svc_b.join_topic(&topic, hints_b).await.unwrap();
        log_step!("svc_b joined {}", topic);
        let _rx_a = svc_a.subscribe(&topic).await.unwrap();
        let mut rx_b = svc_b.subscribe(&topic).await.unwrap();
        assert!(
            wait_for_topic_membership(&svc_a, &topic, Duration::from_secs(15)).await,
            "svc_a failed to join topic {topic}"
        );
        assert!(
            wait_for_topic_membership(&svc_b, &topic, Duration::from_secs(15)).await,
            "svc_b failed to join topic {topic}"
        );
        log_step!("services joined topic {}, waiting for peer events", topic);

        // NeighborUpがどちらかで観測されるまで待機
        let mut event_receivers = [&mut rx_a_evt, &mut rx_b_evt];
        if !wait_for_peer_join_event(&mut event_receivers, Duration::from_secs(20)).await {
            eprintln!("peer join event not observed, continuing after grace period");
        }

        // 少し安定化
        sleep(Duration::from_secs(1)).await;
        log_step!("sending broadcast message on topic {}", topic);

        // Aから送信→Bが受信（NIP-01準拠のイベントを送信）
        let keys = Keys::generate();
        let ne = EventBuilder::text_note("hello-int")
            .sign_with_keys(&keys)
            .unwrap();
        let ev = nostr_to_domain(&ne);
        svc_a.broadcast(&topic, &ev).await.unwrap();
        let r = timeout(Duration::from_secs(15), async { rx_b.recv().await })
            .await
            .expect("receive timeout");
        assert!(r.is_some());
        assert_eq!(r.unwrap().content, "hello-int");
        log_step!("--- test_two_nodes_broadcast_and_receive end ---");
    }

    /// 複数購読者が同一トピックのイベントを受け取れること

    /// P2P経路のみで返信イベントを伝搬できることを検証
    #[tokio::test]
    async fn test_p2p_reply_flow() {
        let Some(ctx) = bootstrap_context("test_p2p_reply_flow") else {
            return;
        };
        log_step!("--- test_p2p_reply_flow start ---");
        use tokio::sync::mpsc::unbounded_channel;

        let mut svc_a = create_service(&ctx).await;
        let mut svc_b = create_service(&ctx).await;

        let topic = generate_topic_id("iroh-int-reply-flow");

        let local_hints = vec![svc_a.local_peer_hint(), svc_b.local_peer_hint()];
        let hints_a = build_peer_hints(&ctx.hints, &local_hints, 0);
        let hints_b = build_peer_hints(&ctx.hints, &local_hints, 1);
        svc_a.join_topic(&topic, hints_a).await.unwrap();
        log_step!("svc_a joined {}", topic);
        svc_b.join_topic(&topic, hints_b).await.unwrap();
        log_step!("svc_b joined {}", topic);

        let mut rx_a = svc_a.subscribe(&topic).await.unwrap();
        let mut rx_b = svc_b.subscribe(&topic).await.unwrap();
        assert!(
            wait_for_topic_membership(&svc_a, &topic, Duration::from_secs(15)).await,
            "svc_a failed to join topic {topic}"
        );
        assert!(
            wait_for_topic_membership(&svc_b, &topic, Duration::from_secs(15)).await,
            "svc_b failed to join topic {topic}"
        );

        let (tx_a_evt, mut rx_a_evt) = unbounded_channel();
        let (tx_b_evt, mut rx_b_evt) = unbounded_channel();
        svc_a.set_event_sender(tx_a_evt);
        svc_b.set_event_sender(tx_b_evt);

        let mut event_receivers = [&mut rx_a_evt, &mut rx_b_evt];
        if !wait_for_peer_join_event(&mut event_receivers, Duration::from_secs(20)).await {
            eprintln!("peer join event not observed for reply flow, continuing optimistically");
        }
        sleep(Duration::from_secs(1)).await;
        log_step!("broadcasting root event on topic {}", topic);

        let root_keys = Keys::generate();
        let root_note = EventBuilder::text_note("root-post")
            .sign_with_keys(&root_keys)
            .unwrap();
        let root_event = nostr_to_domain(&root_note);
        let root_id = root_event.id.clone();
        let root_pubkey = root_event.pubkey.clone();
        svc_a.broadcast(&topic, &root_event).await.unwrap();

        let received_root = timeout(Duration::from_secs(15), async { rx_b.recv().await })
            .await
            .expect("root receive timeout")
            .expect("root channel closed");
        assert_eq!(received_root.content, "root-post");
        log_step!("root event received, sending reply");

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

        let received_reply = timeout(Duration::from_secs(15), async { rx_a.recv().await })
            .await
            .expect("reply receive timeout")
            .expect("reply channel closed");
        assert_eq!(received_reply.content, "reply-post");

        let e_tag = received_reply
            .tags
            .iter()
            .find(|tag| tag.first().map(|s| s.as_str()) == Some("e"))
            .expect("reply event missing e tag");
        assert_eq!(e_tag.get(1).map(|s| s.as_str()), Some(root_id.as_str()));
        assert_eq!(e_tag.get(3).map(|s| s.as_str()), Some("reply"));

        let p_tag = received_reply
            .tags
            .iter()
            .find(|tag| tag.first().map(|s| s.as_str()) == Some("p"))
            .expect("reply event missing p tag");
        assert_eq!(p_tag.get(1).map(|s| s.as_str()), Some(root_pubkey.as_str()));
        log_step!("--- test_p2p_reply_flow end ---");
    }

    /// P2P経路のみで引用イベント（mention）が伝搬されることを検証
    #[tokio::test]
    async fn test_p2p_quote_flow() {
        let Some(ctx) = bootstrap_context("test_p2p_quote_flow") else {
            return;
        };
        log_step!("--- test_p2p_quote_flow start ---");
        use tokio::sync::mpsc::unbounded_channel;

        let mut svc_a = create_service(&ctx).await;
        let mut svc_b = create_service(&ctx).await;

        let topic = generate_topic_id("iroh-int-quote-flow");

        let local_hints = vec![svc_a.local_peer_hint(), svc_b.local_peer_hint()];
        let hints_a = build_peer_hints(&ctx.hints, &local_hints, 0);
        let hints_b = build_peer_hints(&ctx.hints, &local_hints, 1);
        svc_a.join_topic(&topic, hints_a).await.unwrap();
        log_step!("svc_a joined {}", topic);
        svc_b.join_topic(&topic, hints_b).await.unwrap();
        log_step!("svc_b joined {}", topic);

        let mut rx_a = svc_a.subscribe(&topic).await.unwrap();
        let mut rx_b = svc_b.subscribe(&topic).await.unwrap();
        assert!(
            wait_for_topic_membership(&svc_a, &topic, Duration::from_secs(15)).await,
            "svc_a failed to join topic {topic}"
        );
        assert!(
            wait_for_topic_membership(&svc_b, &topic, Duration::from_secs(15)).await,
            "svc_b failed to join topic {topic}"
        );

        let (tx_a_evt, mut rx_a_evt) = unbounded_channel();
        let (tx_b_evt, mut rx_b_evt) = unbounded_channel();
        svc_a.set_event_sender(tx_a_evt);
        svc_b.set_event_sender(tx_b_evt);

        let mut event_receivers = [&mut rx_a_evt, &mut rx_b_evt];
        if !wait_for_peer_join_event(&mut event_receivers, Duration::from_secs(20)).await {
            eprintln!("peer join event not observed for quote flow, continuing optimistically");
        }
        sleep(Duration::from_secs(1)).await;
        log_step!("broadcasting base event on topic {}", topic);

        let base_keys = Keys::generate();
        let base_note = EventBuilder::text_note("quote-root")
            .sign_with_keys(&base_keys)
            .unwrap();
        let base_event = nostr_to_domain(&base_note);
        let base_id = base_event.id.clone();
        let base_pubkey = base_event.pubkey.clone();
        svc_a.broadcast(&topic, &base_event).await.unwrap();

        let _ = timeout(Duration::from_secs(15), async { rx_b.recv().await })
            .await
            .expect("base receive timeout")
            .expect("base channel closed");
        log_step!("base event received, sending quote");

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

        let received_quote = timeout(Duration::from_secs(15), async { rx_a.recv().await })
            .await
            .expect("quote receive timeout")
            .expect("quote channel closed");
        assert_eq!(received_quote.content, "quote-post");

        let e_tag = received_quote
            .tags
            .iter()
            .find(|tag| tag.first().map(|s| s.as_str()) == Some("e"))
            .expect("quote event missing e tag");
        assert_eq!(e_tag.get(1).map(|s| s.as_str()), Some(base_id.as_str()));
        assert_eq!(e_tag.get(3).map(|s| s.as_str()), Some("mention"));

        let p_tag = received_quote
            .tags
            .iter()
            .find(|tag| tag.first().map(|s| s.as_str()) == Some("p"))
            .expect("quote event missing p tag");
        assert_eq!(p_tag.get(1).map(|s| s.as_str()), Some(base_pubkey.as_str()));
        log_step!("--- test_p2p_quote_flow end ---");
    }

    #[tokio::test]
    async fn test_multiple_subscribers_receive() {
        let Some(ctx) = bootstrap_context("test_multiple_subscribers_receive") else {
            return;
        };
        log_step!("--- test_multiple_subscribers_receive start ---");
        let svc_a = create_service(&ctx).await;
        let svc_b = create_service(&ctx).await;

        let topic = generate_topic_id("iroh-int-multi-subs");
        let local_hints = vec![svc_a.local_peer_hint(), svc_b.local_peer_hint()];
        let hints_a = build_peer_hints(&ctx.hints, &local_hints, 0);
        let hints_b = build_peer_hints(&ctx.hints, &local_hints, 1);
        svc_a.join_topic(&topic, hints_a).await.unwrap();
        log_step!("svc_a joined {}", topic);
        svc_b.join_topic(&topic, hints_b).await.unwrap();
        log_step!("svc_b joined {}", topic);

        // B側に2購読者
        let mut rx1 = svc_b.subscribe(&topic).await.unwrap();
        let mut rx2 = svc_b.subscribe(&topic).await.unwrap();
        assert!(
            wait_for_topic_membership(&svc_b, &topic, Duration::from_secs(15)).await,
            "svc_b failed to join topic {topic}"
        );
        assert!(
            wait_for_topic_membership(&svc_a, &topic, Duration::from_secs(15)).await,
            "svc_a failed to join topic {topic}"
        );

        sleep(Duration::from_secs(1)).await;
        log_step!("broadcasting multi-subscriber event on {}", topic);
        // Aから送信（NIP-01準拠）
        let keys = Keys::generate();
        let ne = EventBuilder::text_note("hello-multi")
            .sign_with_keys(&keys)
            .unwrap();
        let ev = nostr_to_domain(&ne);
        svc_a.broadcast(&topic, &ev).await.unwrap();

        let r1 = timeout(Duration::from_secs(15), async { rx1.recv().await })
            .await
            .expect("rx1 timeout");
        let r2 = timeout(Duration::from_secs(15), async { rx2.recv().await })
            .await
            .expect("rx2 timeout");

        assert!(r1.is_some() && r2.is_some());
        assert_eq!(r1.unwrap().content, "hello-multi");
        assert_eq!(r2.unwrap().content, "hello-multi");
        log_step!("--- test_multiple_subscribers_receive end ---");
    }

    /// 3ノード構成でA->(B,C)へブロードキャストが届くことを検証
    #[tokio::test]
    async fn test_multi_node_broadcast_three_nodes() {
        let Some(ctx) = bootstrap_context("test_multi_node_broadcast_three_nodes") else {
            return;
        };
        log_step!("--- test_multi_node_broadcast_three_nodes start ---");

        let svc_a = create_service(&ctx).await;
        let svc_b = create_service(&ctx).await;
        let svc_c = create_service(&ctx).await;

        let topic = generate_topic_id("iroh-int-multi-node");
        let local_hints = vec![
            svc_a.local_peer_hint(),
            svc_b.local_peer_hint(),
            svc_c.local_peer_hint(),
        ];
        let hints_a = build_peer_hints(&ctx.hints, &local_hints, 0);
        let hints_b = build_peer_hints(&ctx.hints, &local_hints, 1);
        let hints_c = build_peer_hints(&ctx.hints, &local_hints, 2);
        svc_a.join_topic(&topic, hints_a).await.unwrap();
        log_step!("svc_a joined {}", topic);
        svc_b.join_topic(&topic, hints_b).await.unwrap();
        log_step!("svc_b joined {}", topic);
        svc_c.join_topic(&topic, hints_c).await.unwrap();
        log_step!("svc_c joined {}", topic);
        // 受信側B,Cは購読（内部でjoin）
        let mut rx_b = svc_b.subscribe(&topic).await.unwrap();
        let mut rx_c = svc_c.subscribe(&topic).await.unwrap();
        assert!(
            wait_for_topic_membership(&svc_b, &topic, Duration::from_secs(15)).await,
            "svc_b failed to join topic {topic}"
        );
        assert!(
            wait_for_topic_membership(&svc_c, &topic, Duration::from_secs(15)).await,
            "svc_c failed to join topic {topic}"
        );
        // 送信側Aはjoinのみ
        assert!(
            wait_for_topic_membership(&svc_a, &topic, Duration::from_secs(15)).await,
            "svc_a failed to join topic {topic}"
        );
        log_step!("all nodes joined topic {}", topic);

        // 安定化待ち
        sleep(Duration::from_secs(1)).await;

        // Aから送信（NIP-01準拠）
        let keys = Keys::generate();
        let ne = EventBuilder::text_note("hello-3nodes")
            .sign_with_keys(&keys)
            .unwrap();
        let ev = nostr_to_domain(&ne);
        svc_a.broadcast(&topic, &ev).await.unwrap();

        // BとCで受信
        let r_b = timeout(Duration::from_secs(15), async { rx_b.recv().await })
            .await
            .expect("B receive timeout");
        let r_c = timeout(Duration::from_secs(15), async { rx_c.recv().await })
            .await
            .expect("C receive timeout");

        assert!(r_b.is_some() && r_c.is_some());
        assert_eq!(r_b.unwrap().content, "hello-3nodes");
        assert_eq!(r_c.unwrap().content, "hello-3nodes");
        log_step!("--- test_multi_node_broadcast_three_nodes end ---");
    }

    /// 双方向にメッセージをやり取りし、近接で安定して届くことを簡易確認
    #[tokio::test]
    async fn test_peer_connection_stability_bidirectional() {
        let Some(ctx) = bootstrap_context("test_peer_connection_stability_bidirectional") else {
            return;
        };
        log_step!("--- test_peer_connection_stability_bidirectional start ---");
        use tokio::sync::mpsc::unbounded_channel;

        let mut svc_a = create_service(&ctx).await;
        let mut svc_b = create_service(&ctx).await;

        let topic = generate_topic_id("iroh-int-stability");
        let local_hints = vec![svc_a.local_peer_hint(), svc_b.local_peer_hint()];
        let hints_a = build_peer_hints(&ctx.hints, &local_hints, 0);
        let hints_b = build_peer_hints(&ctx.hints, &local_hints, 1);
        svc_a.join_topic(&topic, hints_a).await.unwrap();
        log_step!("svc_a joined {}", topic);
        svc_b.join_topic(&topic, hints_b).await.unwrap();
        log_step!("svc_b joined {}", topic);
        let mut rx_a = svc_a.subscribe(&topic).await.unwrap();
        let mut rx_b = svc_b.subscribe(&topic).await.unwrap();
        assert!(
            wait_for_topic_membership(&svc_a, &topic, Duration::from_secs(15)).await,
            "svc_a failed to join topic {topic}"
        );
        assert!(
            wait_for_topic_membership(&svc_b, &topic, Duration::from_secs(15)).await,
            "svc_b failed to join topic {topic}"
        );

        let (tx_a_evt, mut rx_a_evt) = unbounded_channel();
        let (tx_b_evt, mut rx_b_evt) = unbounded_channel();
        svc_a.set_event_sender(tx_a_evt);
        svc_b.set_event_sender(tx_b_evt);

        let mut event_receivers = [&mut rx_a_evt, &mut rx_b_evt];
        if !wait_for_peer_join_event(&mut event_receivers, Duration::from_secs(20)).await {
            eprintln!("peer join event not observed for stability test, continuing optimistically");
        }

        // 安定化待ち
        sleep(Duration::from_secs(1)).await;
        log_step!("broadcasting ping sequence on topic {}", topic);

        // A→B, B→A 交互に送信
        for i in 0..5u32 {
            let keys = Keys::generate();
            let ne = EventBuilder::text_note(format!("ping-{i}"))
                .sign_with_keys(&keys)
                .unwrap();
            let ev = nostr_to_domain(&ne);
            if i % 2 == 0 {
                svc_a.broadcast(&topic, &ev).await.unwrap();
            } else {
                svc_b.broadcast(&topic, &ev).await.unwrap();
            }
        }

        // 少なくとも片側で3件以上受信できること（緩い安定性チェック）
        let mut count_a = 0;
        let mut count_b = 0;
        let start = tokio::time::Instant::now();
        while start.elapsed() < Duration::from_secs(12) && (count_a < 3 || count_b < 3) {
            if let Ok(Some(_)) =
                timeout(Duration::from_millis(150), async { rx_a.recv().await }).await
            {
                count_a += 1;
            }
            if let Ok(Some(_)) =
                timeout(Duration::from_millis(150), async { rx_b.recv().await }).await
            {
                count_b += 1;
            }
        }
        assert!(
            count_a >= 3 || count_b >= 3,
            "insufficient messages received: a={count_a}, b={count_b}"
        );
        log_step!(
            "--- test_peer_connection_stability_bidirectional end (counts a={}, b={}) ---",
            count_a,
            count_b
        );
    }
}
