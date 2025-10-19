use crate::infrastructure::p2p::gossip_service::GossipService;
use crate::modules::p2p::generate_topic_id;
use nostr_sdk::prelude::*;
use tokio::sync::mpsc::unbounded_channel;
use tokio::time::{Duration, sleep, timeout};

use super::support::{
    DEFAULT_EVENT_TIMEOUT, DEFAULT_JOIN_TIMEOUT, build_peer_hints, create_service, init_tracing,
    load_bootstrap_context, log_step, nostr_to_domain, wait_for_peer_join_event,
    wait_for_topic_membership,
};

/// 二ノード間でのブロードキャスト配送を検証する
#[tokio::test]
async fn test_two_nodes_broadcast_and_receive() {
    init_tracing();
    let Some(ctx) = load_bootstrap_context("test_two_nodes_broadcast_and_receive") else {
        return;
    };
    log_step!("--- test_two_nodes_broadcast_and_receive start ---");

    let mut svc_a = create_service(&ctx).await;
    let mut svc_b = create_service(&ctx).await;

    let (tx_a, mut rx_a_evt) = unbounded_channel();
    let (tx_b, mut rx_b_evt) = unbounded_channel();
    svc_a.set_event_sender(tx_a);
    svc_b.set_event_sender(tx_b);

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
        wait_for_topic_membership(&svc_a, &topic, DEFAULT_JOIN_TIMEOUT).await,
        "svc_a failed to join topic {topic}"
    );
    assert!(
        wait_for_topic_membership(&svc_b, &topic, DEFAULT_JOIN_TIMEOUT).await,
        "svc_b failed to join topic {topic}"
    );
    log_step!("services joined topic {}, waiting for peer events", topic);

    let mut event_receivers = [&mut rx_a_evt, &mut rx_b_evt];
    if !wait_for_peer_join_event(&mut event_receivers, Duration::from_secs(20)).await {
        log_step!("peer join event not observed, continuing after grace period");
    }

    sleep(Duration::from_secs(1)).await;
    log_step!("sending broadcast message on topic {}", topic);

    let keys = Keys::generate();
    let ne = EventBuilder::text_note("hello-int")
        .sign_with_keys(&keys)
        .unwrap();
    let ev = nostr_to_domain(&ne);
    svc_a.broadcast(&topic, &ev).await.unwrap();

    let r = timeout(DEFAULT_EVENT_TIMEOUT, async { rx_b.recv().await })
        .await
        .expect("receive timeout");
    assert!(r.is_some());
    assert_eq!(r.unwrap().content, "hello-int");
    log_step!("--- test_two_nodes_broadcast_and_receive end ---");
}

/// 複数購読者が同一トピックのイベントを受け取れることを検証
#[tokio::test]
async fn test_multiple_subscribers_receive() {
    init_tracing();
    let Some(ctx) = load_bootstrap_context("test_multiple_subscribers_receive") else {
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

    let mut rx1 = svc_b.subscribe(&topic).await.unwrap();
    let mut rx2 = svc_b.subscribe(&topic).await.unwrap();

    assert!(
        wait_for_topic_membership(&svc_b, &topic, DEFAULT_JOIN_TIMEOUT).await,
        "svc_b failed to join topic {topic}"
    );
    assert!(
        wait_for_topic_membership(&svc_a, &topic, DEFAULT_JOIN_TIMEOUT).await,
        "svc_a failed to join topic {topic}"
    );

    sleep(Duration::from_secs(1)).await;
    log_step!("broadcasting multi-subscriber event on {}", topic);

    let keys = Keys::generate();
    let ne = EventBuilder::text_note("hello-multi")
        .sign_with_keys(&keys)
        .unwrap();
    let ev = nostr_to_domain(&ne);
    svc_a.broadcast(&topic, &ev).await.unwrap();

    let r1 = timeout(DEFAULT_EVENT_TIMEOUT, async { rx1.recv().await })
        .await
        .expect("rx1 timeout");
    let r2 = timeout(DEFAULT_EVENT_TIMEOUT, async { rx2.recv().await })
        .await
        .expect("rx2 timeout");

    assert!(r1.is_some() && r2.is_some());
    assert_eq!(r1.unwrap().content, "hello-multi");
    assert_eq!(r2.unwrap().content, "hello-multi");
    log_step!("--- test_multiple_subscribers_receive end ---");
}

/// P2P経路のみで返信イベントを伝搬できることを検証
#[tokio::test]
async fn test_p2p_reply_flow() {
    init_tracing();
    let Some(ctx) = load_bootstrap_context("test_p2p_reply_flow") else {
        return;
    };
    log_step!("--- test_p2p_reply_flow start ---");

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
        wait_for_topic_membership(&svc_a, &topic, DEFAULT_JOIN_TIMEOUT).await,
        "svc_a failed to join topic {topic}"
    );
    assert!(
        wait_for_topic_membership(&svc_b, &topic, DEFAULT_JOIN_TIMEOUT).await,
        "svc_b failed to join topic {topic}"
    );

    let (tx_a_evt, mut rx_a_evt) = unbounded_channel();
    let (tx_b_evt, mut rx_b_evt) = unbounded_channel();
    svc_a.set_event_sender(tx_a_evt);
    svc_b.set_event_sender(tx_b_evt);

    let mut event_receivers = [&mut rx_a_evt, &mut rx_b_evt];
    if !wait_for_peer_join_event(&mut event_receivers, Duration::from_secs(20)).await {
        log_step!("peer join event not observed for reply flow, continuing optimistically");
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

    let received_root = timeout(DEFAULT_EVENT_TIMEOUT, async { rx_b.recv().await })
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

    let received_reply = timeout(DEFAULT_EVENT_TIMEOUT, async { rx_a.recv().await })
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
    init_tracing();
    let Some(ctx) = load_bootstrap_context("test_p2p_quote_flow") else {
        return;
    };
    log_step!("--- test_p2p_quote_flow start ---");

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
        wait_for_topic_membership(&svc_a, &topic, DEFAULT_JOIN_TIMEOUT).await,
        "svc_a failed to join topic {topic}"
    );
    assert!(
        wait_for_topic_membership(&svc_b, &topic, DEFAULT_JOIN_TIMEOUT).await,
        "svc_b failed to join topic {topic}"
    );

    let (tx_a_evt, mut rx_a_evt) = unbounded_channel();
    let (tx_b_evt, mut rx_b_evt) = unbounded_channel();
    svc_a.set_event_sender(tx_a_evt);
    svc_b.set_event_sender(tx_b_evt);

    let mut event_receivers = [&mut rx_a_evt, &mut rx_b_evt];
    if !wait_for_peer_join_event(&mut event_receivers, Duration::from_secs(20)).await {
        log_step!("peer join event not observed for quote flow, continuing optimistically");
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

    let _ = timeout(DEFAULT_EVENT_TIMEOUT, async { rx_b.recv().await })
        .await
        .expect("base receive timeout")
        .expect("base channel closed");
    log_step!("base event received, sending quote");

    let mention_tag = Tag::from_standardized(TagStandard::Event {
        event_id: base_note.id,
        relay_url: None,
        marker: Some(Marker::Mention),
        public_key: None,
        uppercase: false,
    });
    let mention_pubkey_tag = Tag::from_standardized(TagStandard::public_key(base_note.pubkey));
    let quote_keys = Keys::generate();
    let quote_note = EventBuilder::text_note("quote-post")
        .tags([mention_tag, mention_pubkey_tag])
        .sign_with_keys(&quote_keys)
        .unwrap();
    let quote_event = nostr_to_domain(&quote_note);
    svc_b.broadcast(&topic, &quote_event).await.unwrap();

    let received_quote = timeout(DEFAULT_EVENT_TIMEOUT, async { rx_a.recv().await })
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
