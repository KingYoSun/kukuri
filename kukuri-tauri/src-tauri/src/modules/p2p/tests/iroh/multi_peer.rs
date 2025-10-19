use crate::infrastructure::p2p::gossip_service::GossipService;
use crate::modules::p2p::generate_topic_id;
use nostr_sdk::prelude::*;
use tokio::time::{Duration, sleep, timeout};

use super::support::{
    DEFAULT_EVENT_TIMEOUT, DEFAULT_JOIN_TIMEOUT, build_peer_hints, create_service, init_tracing,
    load_bootstrap_context, log_step, nostr_to_domain, wait_for_topic_membership,
};

/// 3ノード構成でA->(B,C)へブロードキャストが届くことを検証
#[tokio::test]
async fn test_multi_node_broadcast_three_nodes() {
    init_tracing();
    let Some(ctx) = load_bootstrap_context("test_multi_node_broadcast_three_nodes") else {
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

    let mut rx_b = svc_b.subscribe(&topic).await.unwrap();
    let mut rx_c = svc_c.subscribe(&topic).await.unwrap();

    assert!(
        wait_for_topic_membership(&svc_b, &topic, DEFAULT_JOIN_TIMEOUT).await,
        "svc_b failed to join topic {topic}"
    );
    assert!(
        wait_for_topic_membership(&svc_c, &topic, DEFAULT_JOIN_TIMEOUT).await,
        "svc_c failed to join topic {topic}"
    );
    assert!(
        wait_for_topic_membership(&svc_a, &topic, DEFAULT_JOIN_TIMEOUT).await,
        "svc_a failed to join topic {topic}"
    );
    log_step!("all nodes joined topic {}", topic);

    sleep(Duration::from_secs(1)).await;

    let keys = Keys::generate();
    let ne = EventBuilder::text_note("hello-3nodes")
        .sign_with_keys(&keys)
        .unwrap();
    let ev = nostr_to_domain(&ne);
    svc_a.broadcast(&topic, &ev).await.unwrap();

    let r_b = timeout(DEFAULT_EVENT_TIMEOUT, async { rx_b.recv().await })
        .await
        .expect("B receive timeout");
    let r_c = timeout(DEFAULT_EVENT_TIMEOUT, async { rx_c.recv().await })
        .await
        .expect("C receive timeout");

    assert!(r_b.is_some() && r_c.is_some());
    assert_eq!(r_b.unwrap().content, "hello-3nodes");
    assert_eq!(r_c.unwrap().content, "hello-3nodes");
    log_step!("--- test_multi_node_broadcast_three_nodes end ---");
}
