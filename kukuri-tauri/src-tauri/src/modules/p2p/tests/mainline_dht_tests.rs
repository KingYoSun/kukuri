#[cfg(test)]
mod tests {
    use crate::application::services::p2p_service::{P2PService, P2PServiceTrait};
    use crate::domain::entities::Event;
    use crate::infrastructure::p2p::{
        DiscoveryOptions, NetworkService, gossip_service::GossipService,
        iroh_gossip_service::IrohGossipService, iroh_network_service::IrohNetworkService,
    };
    use crate::shared::config::{AppConfig, NetworkConfig as AppNetworkConfig};
    use chrono::Utc;
    use iroh::SecretKey;
    use nostr_sdk::prelude::*;
    use rand::{RngCore, SeedableRng, rngs::StdRng};
    use tokio::time::{Duration, Instant, sleep, timeout};

    macro_rules! log_step {
        ($($arg:tt)*) => {{
            eprintln!("[mainline_dht_tests] {}", format!($($arg)*));
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
        network_config.enable_local = false;

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

    fn nostr_to_domain(ev: &nostr_sdk::Event) -> Event {
        let created_at =
            chrono::DateTime::<chrono::Utc>::from_timestamp(ev.created_at.as_u64() as i64, 0)
                .unwrap_or_else(|| chrono::Utc::now());
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

    async fn wait_for_bootstrap_peer(
        service: &IrohNetworkService,
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
        service: &IrohGossipService,
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
        service: &IrohGossipService,
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_mainline_handshake_routing() {
        let Some(ctx) = prepare_context("test_mainline_handshake_routing") else {
            return;
        };
        log_step!(
            "bootstrap hints resolved: {}",
            ctx.hints
                .iter()
                .map(|h| h.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );

        let MainlineContext {
            hints,
            network_config,
        } = ctx;

        let discovery = DiscoveryOptions::new(false, true, false);

        let mut rng = StdRng::from_entropy();
        let secret_a = random_secret(&mut rng);
        let secret_b = random_secret(&mut rng);

        let stack_a = P2PService::builder(secret_a, network_config.clone())
            .with_discovery_options(discovery)
            .build()
            .await
            .expect("build stack A");
        let stack_b = P2PService::builder(secret_b, network_config)
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
            wait_for_bootstrap_peer(&stack_a.network_service, 1, Duration::from_secs(12)).await,
            "A failed to discover bootstrap peer via mainline DHT"
        );
        assert!(
            wait_for_bootstrap_peer(&stack_b.network_service, 1, Duration::from_secs(12)).await,
            "B failed to discover bootstrap peer via mainline DHT"
        );

        let hint_a =
            wait_for_local_peer_hint(&stack_a.gossip_service, Duration::from_secs(5)).await;
        let hint_b =
            wait_for_local_peer_hint(&stack_b.gossip_service, Duration::from_secs(5)).await;

        let mut hints_for_a = hints.clone();
        let mut hints_for_b = hints.clone();
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

        let mut rx_b = stack_b
            .gossip_service
            .subscribe(&topic_id)
            .await
            .expect("subscribe on B");

        assert!(
            wait_for_topic_membership(&stack_a.gossip_service, &topic_id, Duration::from_secs(15))
                .await,
            "A did not observe topic membership via DHT routing"
        );
        assert!(
            wait_for_topic_membership(&stack_b.gossip_service, &topic_id, Duration::from_secs(15))
                .await,
            "B did not observe topic membership via DHT routing"
        );

        sleep(Duration::from_millis(750)).await;

        let keys = Keys::generate();
        let payload = format!("mainline-handshake-{}", keys.public_key());
        let nostr_event = EventBuilder::text_note(&payload)
            .sign_with_keys(&keys)
            .expect("sign nostr event");
        let event = nostr_to_domain(&nostr_event);

        stack_a
            .gossip_service
            .broadcast(&topic_id, &event)
            .await
            .expect("broadcast from A");

        let delivered = timeout(Duration::from_secs(20), async { rx_b.recv().await })
            .await
            .expect("B receive timeout");
        let delivered = delivered.expect("B subscription closed unexpectedly");
        assert_eq!(delivered.content, payload);

        let stats_a = stack_a
            .network_service
            .get_stats()
            .await
            .expect("fetch stats from A");
        assert!(
            stats_a.connected_peers >= 1,
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
    }
}
