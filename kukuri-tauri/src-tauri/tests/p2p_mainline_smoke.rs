use chrono::Utc;
use iroh::SecretKey;
use kukuri_lib::test_support::application::services::p2p_service::{P2PService, P2PServiceTrait};
use kukuri_lib::test_support::infrastructure::p2p::{
    DiscoveryOptions, NetworkService, gossip_service::GossipService,
};
use kukuri_lib::test_support::shared::config::{AppConfig, NetworkConfig as AppNetworkConfig};
use rand::{RngCore, SeedableRng, rngs::StdRng};
use std::sync::Arc;
use tokio::time::{Duration, Instant, sleep};

macro_rules! log_step {
    ($($arg:tt)*) => {{
        eprintln!("[p2p_mainline_smoke] {}", format!($($arg)*));
    }};
}

struct MainlineContext {
    hints: Vec<String>,
    network_config: AppNetworkConfig,
}

fn prepare_context(test_name: &str) -> Option<MainlineContext> {
    if std::env::var("ENABLE_P2P_INTEGRATION").unwrap_or_default() != "1" {
        eprintln!("skipping {test_name} (ENABLE_P2P_INTEGRATION!=1)");
        return None;
    }
    let raw = std::env::var("KUKURI_BOOTSTRAP_PEERS").unwrap_or_default();
    let hints: Vec<String> = raw
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if hints.is_empty() {
        eprintln!("skipping {test_name} (KUKURI_BOOTSTRAP_PEERS not set)");
        return None;
    }

    let app_cfg = AppConfig::from_env();
    let mut network_config = app_cfg.network.clone();
    if network_config.bootstrap_peers.is_empty() {
        network_config.bootstrap_peers = hints.clone();
    }
    network_config.enable_dht = true;
    network_config.enable_dns = false;
    network_config.enable_local = true;

    Some(MainlineContext {
        hints,
        network_config,
    })
}

fn random_secret(rng: &mut StdRng) -> SecretKey {
    let mut bytes = [0u8; 32];
    rng.fill_bytes(&mut bytes);
    SecretKey::from_bytes(&bytes)
}

async fn wait_for_bootstrap_peer(
    service: &Arc<dyn NetworkService>,
    min_peers: usize,
    deadline: Duration,
) -> bool {
    let start = Instant::now();
    while start.elapsed() < deadline {
        if let Ok(peers) = service.get_peers().await {
            if peers.len() >= min_peers {
                return true;
            }
        }
        sleep(Duration::from_millis(200)).await;
    }
    false
}

async fn wait_for_local_peer_hint(
    service: &Arc<dyn GossipService>,
    deadline: Duration,
) -> Option<String> {
    let start = Instant::now();
    while start.elapsed() < deadline {
        if let Some(hint) = service.local_peer_hint() {
            return Some(hint);
        }
        sleep(Duration::from_millis(150)).await;
    }
    None
}

async fn wait_for_topic_membership(
    service: &Arc<dyn GossipService>,
    topic: &str,
    deadline: Duration,
) -> bool {
    let start = Instant::now();
    while start.elapsed() < deadline {
        match service.get_joined_topics().await {
            Ok(joined) if joined.iter().any(|t| t == topic) => return true,
            Ok(_) => {}
            Err(err) => {
                log_step!("get_joined_topics error for {}: {:?}", topic, err);
            }
        }
        sleep(Duration::from_millis(200)).await;
    }
    false
}

#[tokio::test]
async fn test_mainline_dht_handshake_and_routing() {
    let Some(ctx) = prepare_context("test_mainline_dht_handshake_and_routing") else {
        return;
    };
    log_step!("--- mainline handshake start ---");

    let discovery = DiscoveryOptions::new(false, true, true);

    let mut rng = StdRng::from_entropy();
    let secret_a = random_secret(&mut rng);
    let secret_b = random_secret(&mut rng);

    let stack_a = P2PService::builder(secret_a, ctx.network_config.clone())
        .with_discovery_options(discovery)
        .build()
        .await
        .expect("build stack A");
    let stack_b = P2PService::builder(secret_b, ctx.network_config.clone())
        .with_discovery_options(discovery)
        .build()
        .await
        .expect("build stack B");

    stack_a
        .network_service
        .connect()
        .await
        .expect("connect network A");
    stack_b
        .network_service
        .connect()
        .await
        .expect("connect network B");

    assert!(
        wait_for_bootstrap_peer(&stack_a.network_service, 1, Duration::from_secs(20)).await,
        "A failed to discover bootstrap peer via mainline DHT"
    );
    assert!(
        wait_for_bootstrap_peer(&stack_b.network_service, 1, Duration::from_secs(20)).await,
        "B failed to discover bootstrap peer via mainline DHT"
    );

    let hint_a = wait_for_local_peer_hint(&stack_a.gossip_service, Duration::from_secs(5)).await;
    let hint_b = wait_for_local_peer_hint(&stack_b.gossip_service, Duration::from_secs(5)).await;

    let mut hints_for_a = ctx.hints.clone();
    let mut hints_for_b = ctx.hints.clone();
    if let Some(h) = &hint_b {
        hints_for_a.push(h.clone());
    }
    if let Some(h) = &hint_a {
        hints_for_b.push(h.clone());
    }
    hints_for_a.sort();
    hints_for_a.dedup();
    hints_for_b.sort();
    hints_for_b.dedup();

    let topic_seed = format!(
        "mainline-handshake-routing-{}",
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    let topic_id = stack_a.p2p_service.generate_topic_id(&topic_seed);

    stack_a
        .p2p_service
        .join_topic(&topic_id, hints_for_a.clone())
        .await
        .expect("join topic on A");
    stack_b
        .p2p_service
        .join_topic(&topic_id, hints_for_b.clone())
        .await
        .expect("join topic on B");

    assert!(
        wait_for_topic_membership(&stack_a.gossip_service, &topic_id, Duration::from_secs(45))
            .await,
        "A did not observe topic membership via DHT routing"
    );
    assert!(
        wait_for_topic_membership(&stack_b.gossip_service, &topic_id, Duration::from_secs(45))
            .await,
        "B did not observe topic membership via DHT routing"
    );

    let stats_a = stack_a
        .network_service
        .get_stats()
        .await
        .expect("fetch stats from A");
    assert!(
        stats_a.connected_peers >= 1,
        "expected at least one connected peer after handshake"
    );

    let stats_b = stack_b
        .network_service
        .get_stats()
        .await
        .expect("fetch stats from B");
    assert!(
        stats_b.connected_peers >= 1,
        "expected at least one connected peer after handshake"
    );

    stack_a
        .p2p_service
        .leave_topic(&topic_id)
        .await
        .expect("leave topic A");
    stack_b
        .p2p_service
        .leave_topic(&topic_id)
        .await
        .expect("leave topic B");

    log_step!("--- mainline handshake end ---");
}
