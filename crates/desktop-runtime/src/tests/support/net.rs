use super::super::*;

pub(crate) async fn wait_for_seeded_dht_topic_ready(
    runtime_a: &DesktopRuntime,
    runtime_b: &DesktopRuntime,
    topic: &str,
) {
    match timeout(seeded_dht_runtime_ready_timeout(), async {
        let mut stable_ready_polls = 0usize;
        loop {
            let status_a = runtime_a.get_sync_status().await.expect("status a");
            let status_b = runtime_b.get_sync_status().await.expect("status b");
            let ready_a = topic_has_direct_peer(&status_a, topic, 1)
                || topic_has_durable_delivery(&status_a, topic);
            let ready_b = topic_has_direct_peer(&status_b, topic, 1)
                || topic_has_durable_delivery(&status_b, topic);
            if ready_a && ready_b {
                stable_ready_polls += 1;
                if stable_ready_polls >= 3 {
                    return;
                }
            } else {
                stable_ready_polls = 0;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    {
        Ok(()) => {}
        Err(_) => {
            let status_a = runtime_a.get_sync_status().await.expect("status a");
            let status_b = runtime_b.get_sync_status().await.expect("status b");
            panic!(
                "seeded dht topic readiness timeout for `{topic}`: status_a={status_a:?} status_b={status_b:?}"
            );
        }
    }
}
pub(crate) async fn wait_for_runtime_endpoint_in_testnet(
    runtime: &DesktopRuntime,
    testnet: &Testnet,
) {
    let endpoint = runtime.iroh_stack.endpoint().await;
    let mut builder = DhtBuilder::default();
    builder.bootstrap(&testnet.bootstrap);
    let lookup = DhtAddressLookup::builder()
        .dht_builder(builder)
        .no_publish()
        .addr_filter(AddrFilter::unfiltered())
        .build()
        .expect("dht lookup");
    timeout(Duration::from_secs(30), async {
        loop {
            if let Some(mut resolved) = lookup.resolve(endpoint.id())
                && let Some(Ok(item)) = resolved.next().await
                && item.endpoint_info().endpoint_id == endpoint.id()
            {
                return;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("resolve published endpoint info");
}

pub(crate) fn seeded_dht_config(seed_peers: Vec<SeedPeer>) -> DiscoveryConfig {
    DiscoveryConfig {
        mode: DiscoveryMode::SeededDht,
        connect_mode: ConnectMode::DirectOnly,
        env_locked: false,
        seed_peers,
    }
}
pub(crate) async fn new_seeded_dht_runtime_with_config(
    db_path: &Path,
    testnet: &Testnet,
    discovery_config: DiscoveryConfig,
) -> DesktopRuntime {
    let runtime = DesktopRuntime::new_with_config_and_identity_and_discovery(
        db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
        discovery_config,
        DhtDiscoveryOptions::with_bootstrap(&testnet.bootstrap),
        false,
    )
    .await
    .expect("seeded dht runtime");
    wait_for_runtime_endpoint_in_testnet(&runtime, testnet).await;
    runtime
}

pub(crate) async fn new_seeded_dht_runtime(db_path: &Path, testnet: &Testnet) -> DesktopRuntime {
    new_seeded_dht_runtime_with_config(db_path, testnet, seeded_dht_config(Vec::new())).await
}
