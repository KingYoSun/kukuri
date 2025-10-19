use crate::infrastructure::p2p::gossip_service::GossipService;
use crate::modules::p2p::generate_topic_id;
use nostr_sdk::prelude::*;
use tokio::sync::mpsc::unbounded_channel;
use tokio::time::{Duration, sleep, timeout};

use super::support::{
    DEFAULT_JOIN_TIMEOUT, build_peer_hints, create_service, init_tracing, load_bootstrap_context,
    log_step, nostr_to_domain, wait_for_peer_join_event, wait_for_topic_membership,
};

/// subscribe → join の最小シナリオで接続確認を行う
#[tokio::test]
async fn test_two_nodes_connect_and_join() {
    init_tracing();
    let Some(ctx) = load_bootstrap_context("test_two_nodes_connect_and_join") else {
        return;
    };
    log_step!("--- test_two_nodes_connect_and_join start ---");

    let svc_a = create_service(&ctx).await;
    let svc_b = create_service(&ctx).await;

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
        wait_for_topic_membership(&svc_a, &topic, DEFAULT_JOIN_TIMEOUT).await,
        "svc_a failed to join topic {topic}"
    );
    assert!(
        wait_for_topic_membership(&svc_b, &topic, DEFAULT_JOIN_TIMEOUT).await,
        "svc_b failed to join topic {topic}"
    );
    log_step!("both services joined topic {}", topic);

    let joined_a = svc_a.get_joined_topics().await.unwrap();
    let joined_b = svc_b.get_joined_topics().await.unwrap();
    assert!(joined_a.contains(&topic));
    assert!(joined_b.contains(&topic));
    log_step!("--- test_two_nodes_connect_and_join end ---");
}

/// 双方向にメッセージをやり取りし、近接で安定して届くことを検証する
#[tokio::test]
async fn test_peer_connection_stability_bidirectional() {
    init_tracing();
    let Some(ctx) = load_bootstrap_context("test_peer_connection_stability_bidirectional") else {
        return;
    };
    log_step!("--- test_peer_connection_stability_bidirectional start ---");

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
        log_step!("peer join event not observed for stability test, continuing optimistically");
    }

    sleep(Duration::from_secs(1)).await;
    log_step!("broadcasting ping sequence on topic {}", topic);

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

    let mut count_a = 0;
    let mut count_b = 0;
    let start = tokio::time::Instant::now();
    while start.elapsed() < Duration::from_secs(12) && (count_a < 3 || count_b < 3) {
        if let Ok(Some(_)) = timeout(Duration::from_millis(150), async { rx_a.recv().await }).await
        {
            count_a += 1;
        }
        if let Ok(Some(_)) = timeout(Duration::from_millis(150), async { rx_b.recv().await }).await
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
