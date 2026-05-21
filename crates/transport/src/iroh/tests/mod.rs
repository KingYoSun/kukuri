use super::*;

use n0_mainline::{DhtBuilder, Testnet};

async fn wait_for_endpoint_in_testnet(endpoint: &Endpoint, testnet: &Testnet) {
    let mut dht_builder = DhtBuilder::default();
    dht_builder.bootstrap(&testnet.bootstrap);
    let lookup = DhtAddressLookup::builder()
        .dht_builder(dht_builder)
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
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("resolve endpoint info from DHT");
}

struct HintRoundtripParticipant<'a, T> {
    transport: &'a T,
    stream: &'a mut HintStream,
    expected_source_peer: Option<&'a str>,
}

async fn wait_for_hint_roundtrip<T>(
    participant_a: HintRoundtripParticipant<'_, T>,
    participant_b: HintRoundtripParticipant<'_, T>,
    topic: &TopicId,
    step_timeout: Duration,
    label: &str,
) where
    T: Transport + HintTransport + Sync,
{
    let hint_from_a = GossipHint::TopicObjectsChanged {
        topic_id: topic.clone(),
        objects: vec![HintObjectRef {
            object_id: format!("{label}-from-a"),
            object_kind: "post".into(),
        }],
    };
    let hint_from_b = GossipHint::TopicObjectsChanged {
        topic_id: topic.clone(),
        objects: vec![HintObjectRef {
            object_id: format!("{label}-from-b"),
            object_kind: "post".into(),
        }],
    };
    match timeout(step_timeout, async {
        let mut received_on_a = false;
        let mut received_on_b = false;
        loop {
            if !received_on_a {
                participant_b
                    .transport
                    .publish_hint(topic, hint_from_b.clone())
                    .await
                    .expect("publish hint from b");
            }
            if !received_on_b {
                participant_a
                    .transport
                    .publish_hint(topic, hint_from_a.clone())
                    .await
                    .expect("publish hint from a");
            }
            if !received_on_a
                && let Ok(Some(envelope)) =
                    timeout(Duration::from_millis(500), participant_a.stream.next()).await
            {
                received_on_a = envelope.hint == hint_from_b
                    && participant_b
                        .expected_source_peer
                        .is_none_or(|peer_id| envelope.source_peer == peer_id);
            }
            if !received_on_b
                && let Ok(Some(envelope)) =
                    timeout(Duration::from_millis(500), participant_b.stream.next()).await
            {
                received_on_b = envelope.hint == hint_from_a
                    && participant_a
                        .expected_source_peer
                        .is_none_or(|peer_id| envelope.source_peer == peer_id);
            }
            if received_on_a && received_on_b {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    {
        Ok(()) => {}
        Err(_) => {
            let peers_a = participant_a.transport.peers().await.expect("peers a");
            let peers_b = participant_b.transport.peers().await.expect("peers b");
            panic!(
                "{label} hint roundtrip timeout: a={} b={}",
                format_peer_snapshot(&peers_a),
                format_peer_snapshot(&peers_b)
            );
        }
    }
}

fn format_peer_snapshot(snapshot: &PeerSnapshot) -> String {
    let topics = snapshot
            .topic_diagnostics
            .iter()
            .map(|topic| {
                format!(
                    "{}: joined={}, peer_count={}, connected_peers={:?}, missing_peer_ids={:?}, status_detail={}, last_error={:?}",
                    topic.topic,
                    topic.joined,
                    topic.peer_count,
                    topic.connected_peers,
                    topic.missing_peer_ids,
                    topic.status_detail,
                    topic.last_error
                )
            })
            .collect::<Vec<_>>();
    format!(
        "connected={}, peer_count={}, connected_peers={:?}, configured_peers={:?}, status_detail={}, last_error={:?}, topics={topics:?}",
        snapshot.connected,
        snapshot.peer_count,
        snapshot.connected_peers,
        snapshot.configured_peers,
        snapshot.status_detail,
        snapshot.last_error
    )
}

fn seed_peer_from_ticket(ticket: &str) -> SeedPeer {
    let (endpoint_id, addr_hint) = ticket.split_once('@').expect("ticket host");
    SeedPeer {
        endpoint_id: endpoint_id.to_string(),
        addr_hint: Some(addr_hint.to_string()),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transport_two_process_hint_roundtrip_static_peer() {
    if std::env::var_os("GITHUB_ACTIONS").is_some() {
        return;
    }
    let transport_a = IrohGossipTransport::bind_local()
        .await
        .expect("transport a");
    let transport_b = IrohGossipTransport::bind_local()
        .await
        .expect("transport b");
    let ticket_a = transport_a
        .export_ticket()
        .await
        .expect("ticket a")
        .expect("ticket a value");
    let ticket_b = transport_b
        .export_ticket()
        .await
        .expect("ticket b")
        .expect("ticket b value");
    transport_a
        .import_ticket(&ticket_b)
        .await
        .expect("import b");
    transport_b
        .import_ticket(&ticket_a)
        .await
        .expect("import a");
    let topic = TopicId::new("kukuri:topic:transport");
    let join_timeout = initial_topic_join_timeout();
    let peer_id_a = transport_a.endpoint.id().to_string();
    let peer_id_b = transport_b.endpoint.id().to_string();
    let (mut stream_a, mut stream_b) = tokio::try_join!(
        transport_a.subscribe_hints(&topic),
        transport_b.subscribe_hints(&topic)
    )
    .expect("subscribe both");
    wait_for_hint_roundtrip(
        HintRoundtripParticipant {
            transport: &transport_a,
            stream: &mut stream_a,
            expected_source_peer: Some(peer_id_a.as_str()),
        },
        HintRoundtripParticipant {
            transport: &transport_b,
            stream: &mut stream_b,
            expected_source_peer: Some(peer_id_b.as_str()),
        },
        &topic,
        join_timeout,
        "static-peer",
    )
    .await;

    match timeout(join_timeout, async {
        loop {
            let peers_a = transport_a.peers().await.expect("peers a");
            let peers_b = transport_b.peers().await.expect("peers b");
            let diag_a = peers_a
                .topic_diagnostics
                .iter()
                .find(|topic| topic.topic == "hint/kukuri:topic:transport");
            let diag_b = peers_b
                .topic_diagnostics
                .iter()
                .find(|topic| topic.topic == "hint/kukuri:topic:transport");
            if peers_a.peer_count >= 1
                && peers_b.peer_count >= 1
                && diag_a.is_some_and(|topic| topic.peer_count >= 1)
                && diag_b.is_some_and(|topic| topic.peer_count >= 1)
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    {
        Ok(()) => {}
        Err(_) => {
            let peers_a = transport_a.peers().await.expect("peers a");
            let peers_b = transport_b.peers().await.expect("peers b");
            panic!(
                "peer snapshot timeout: a={} b={}",
                format_peer_snapshot(&peers_a),
                format_peer_snapshot(&peers_b)
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transport_import_ticket_updates_existing_topic_subscription() {
    let transport_a = IrohGossipTransport::bind_local()
        .await
        .expect("transport a");
    let transport_b = IrohGossipTransport::bind_local()
        .await
        .expect("transport b");
    let topic = TopicId::new("kukuri:topic:import-update");
    let join_timeout = Duration::from_secs(10);
    let peer_id_a = transport_a.endpoint.id().to_string();
    let peer_id_b = transport_b.endpoint.id().to_string();
    let (mut stream_a, mut stream_b) = tokio::try_join!(
        transport_a.subscribe_hints(&topic),
        transport_b.subscribe_hints(&topic)
    )
    .expect("subscribe both before import");

    let ticket_a = transport_a
        .export_ticket()
        .await
        .expect("ticket a")
        .expect("ticket a value");
    let ticket_b = transport_b
        .export_ticket()
        .await
        .expect("ticket b")
        .expect("ticket b value");
    transport_a
        .import_ticket(&ticket_b)
        .await
        .expect("import b after subscribe");
    transport_b
        .import_ticket(&ticket_a)
        .await
        .expect("import a after subscribe");

    wait_for_hint_roundtrip(
        HintRoundtripParticipant {
            transport: &transport_a,
            stream: &mut stream_a,
            expected_source_peer: Some(peer_id_a.as_str()),
        },
        HintRoundtripParticipant {
            transport: &transport_b,
            stream: &mut stream_b,
            expected_source_peer: Some(peer_id_b.as_str()),
        },
        &topic,
        join_timeout,
        "import-update",
    )
    .await;

    timeout(join_timeout, async {
        loop {
            let peers_a = transport_a.peers().await.expect("peers a");
            let peers_b = transport_b.peers().await.expect("peers b");
            let direct_a = peers_a.topic_diagnostics.iter().any(|diag| {
                diag.topic == "hint/kukuri:topic:import-update" && !diag.connected_peers.is_empty()
            });
            let direct_b = peers_b.topic_diagnostics.iter().any(|diag| {
                diag.topic == "hint/kukuri:topic:import-update" && !diag.connected_peers.is_empty()
            });
            if direct_a && direct_b {
                return;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("direct topic update timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transport_seed_update_updates_existing_topic_subscription() {
    let transport_a = IrohGossipTransport::bind_local()
        .await
        .expect("transport a");
    let transport_b = IrohGossipTransport::bind_local()
        .await
        .expect("transport b");
    let topic = TopicId::new("kukuri:topic:seed-update");
    let join_timeout = Duration::from_secs(10);
    let peer_id_a = transport_a.endpoint.id().to_string();
    let peer_id_b = transport_b.endpoint.id().to_string();
    let (mut stream_a, mut stream_b) = tokio::try_join!(
        transport_a.subscribe_hints(&topic),
        transport_b.subscribe_hints(&topic)
    )
    .expect("subscribe both before seed update");

    let ticket_a = transport_a
        .export_ticket()
        .await
        .expect("ticket a")
        .expect("ticket a value");
    let ticket_b = transport_b
        .export_ticket()
        .await
        .expect("ticket b")
        .expect("ticket b value");
    transport_a
        .configure_discovery(
            DiscoveryMode::StaticPeer,
            false,
            vec![seed_peer_from_ticket(&ticket_b)],
            Vec::new(),
        )
        .await
        .expect("configure a after subscribe");
    transport_b
        .configure_discovery(
            DiscoveryMode::StaticPeer,
            false,
            vec![seed_peer_from_ticket(&ticket_a)],
            Vec::new(),
        )
        .await
        .expect("configure b after subscribe");

    wait_for_hint_roundtrip(
        HintRoundtripParticipant {
            transport: &transport_a,
            stream: &mut stream_a,
            expected_source_peer: Some(peer_id_a.as_str()),
        },
        HintRoundtripParticipant {
            transport: &transport_b,
            stream: &mut stream_b,
            expected_source_peer: Some(peer_id_b.as_str()),
        },
        &topic,
        join_timeout,
        "seed-update",
    )
    .await;

    timeout(join_timeout, async {
        loop {
            let peers_a = transport_a.peers().await.expect("peers a");
            let peers_b = transport_b.peers().await.expect("peers b");
            let direct_a = peers_a.topic_diagnostics.iter().any(|diag| {
                diag.topic == "hint/kukuri:topic:seed-update" && !diag.connected_peers.is_empty()
            });
            let direct_b = peers_b.topic_diagnostics.iter().any(|diag| {
                diag.topic == "hint/kukuri:topic:seed-update" && !diag.connected_peers.is_empty()
            });
            if direct_a && direct_b {
                return;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("direct seed update timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transport_resubscribe_recreates_timed_out_topic_state() {
    let transport_a = IrohGossipTransport::bind_local()
        .await
        .expect("transport a");
    let transport_b = IrohGossipTransport::bind_local()
        .await
        .expect("transport b");
    let ticket_b = transport_b
        .export_ticket()
        .await
        .expect("ticket b")
        .expect("ticket b value");
    transport_b.shutdown().await;

    transport_a
        .configure_discovery(
            DiscoveryMode::StaticPeer,
            false,
            vec![seed_peer_from_ticket(&ticket_b)],
            Vec::new(),
        )
        .await
        .expect("configure a");

    let topic = TopicId::new("kukuri:topic:timed-out-resubscribe");
    let _stream = transport_a
        .subscribe_hints(&topic)
        .await
        .expect("initial subscribe");
    let topic_key = "hint/kukuri:topic:timed-out-resubscribe";
    let initial_last_error = {
        let topics = transport_a.topic_states.lock().await;
        topics
            .get(topic_key)
            .expect("initial topic state")
            .last_error
            .clone()
    };
    *initial_last_error.lock().await = Some("timed out waiting for initial topic join".to_string());

    let _stream = transport_a
        .subscribe_hints(&topic)
        .await
        .expect("resubscribe after join timeout");
    let recreated_last_error = {
        let topics = transport_a.topic_states.lock().await;
        topics
            .get(topic_key)
            .expect("recreated topic state")
            .last_error
            .clone()
    };

    assert_eq!(
        *recreated_last_error.lock().await,
        None,
        "resubscribe should recreate timed-out topic state so future joins can retry cleanly"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transport_seeded_dht_can_connect_by_endpoint_id_without_ticket() {
    let testnet = Testnet::new(5).await.expect("testnet");
    let config = TransportNetworkConfig::loopback();
    let transport_a = IrohGossipTransport::bind_with_discovery(
        config.clone(),
        DhtDiscoveryOptions::with_bootstrap(&testnet.bootstrap),
    )
    .await
    .expect("transport a");
    let transport_b = IrohGossipTransport::bind_with_discovery(
        config,
        DhtDiscoveryOptions::with_bootstrap(&testnet.bootstrap),
    )
    .await
    .expect("transport b");
    let discovery_a = transport_a.discovery().await.expect("discovery a");
    let discovery_b = transport_b.discovery().await.expect("discovery b");
    wait_for_endpoint_in_testnet(&transport_a.endpoint, &testnet).await;
    wait_for_endpoint_in_testnet(&transport_b.endpoint, &testnet).await;

    transport_a
        .configure_discovery(
            DiscoveryMode::SeededDht,
            false,
            vec![SeedPeer {
                endpoint_id: discovery_b.local_endpoint_id.clone(),
                addr_hint: None,
            }],
            Vec::new(),
        )
        .await
        .expect("configure a");
    transport_b
        .configure_discovery(
            DiscoveryMode::SeededDht,
            false,
            vec![SeedPeer {
                endpoint_id: discovery_a.local_endpoint_id.clone(),
                addr_hint: None,
            }],
            Vec::new(),
        )
        .await
        .expect("configure b");

    let endpoint_b = EndpointId::from_str(&discovery_b.local_endpoint_id).expect("endpoint b");
    let connection = timeout(Duration::from_secs(20), async {
        loop {
            match transport_a
                .endpoint
                .connect(EndpointAddr::new(endpoint_b), GOSSIP_ALPN)
                .await
            {
                Ok(connection) => return connection,
                Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
            }
        }
    })
    .await
    .expect("seeded dht connect timeout");

    drop(connection);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transport_custom_relay_static_peer_seed_peers_connect_without_ticket_import() {
    let (_relay_map, relay_url, _guard) = iroh::test_utils::run_relay_server()
        .await
        .expect("relay server");
    let relay_config = TransportRelayConfig {
        iroh_relay_urls: vec![relay_url.to_string()],
    }
    .normalized();
    let config = TransportNetworkConfig::loopback();
    let transport_a = IrohGossipTransport::bind_with_options(
        config.clone(),
        DhtDiscoveryOptions::disabled(),
        relay_config.clone(),
    )
    .await
    .expect("transport a");
    let transport_b = IrohGossipTransport::bind_with_options(
        config,
        DhtDiscoveryOptions::disabled(),
        relay_config.clone(),
    )
    .await
    .expect("transport b");
    let discovery_a = transport_a.discovery().await.expect("discovery a");
    let discovery_b = transport_b.discovery().await.expect("discovery b");

    transport_a
        .configure_discovery(
            DiscoveryMode::StaticPeer,
            false,
            vec![SeedPeer {
                endpoint_id: discovery_b.local_endpoint_id.clone(),
                addr_hint: None,
            }],
            Vec::new(),
        )
        .await
        .expect("configure a");
    transport_b
        .configure_discovery(
            DiscoveryMode::StaticPeer,
            false,
            vec![SeedPeer {
                endpoint_id: discovery_a.local_endpoint_id.clone(),
                addr_hint: None,
            }],
            Vec::new(),
        )
        .await
        .expect("configure b");

    let endpoint_b = EndpointId::from_str(&discovery_b.local_endpoint_id).expect("endpoint b");
    let connection = timeout(Duration::from_secs(20), async {
        loop {
            match transport_a
                .endpoint
                .connect(EndpointAddr::new(endpoint_b), GOSSIP_ALPN)
                .await
            {
                Ok(connection) => return connection,
                Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
            }
        }
    })
    .await
    .expect("custom relay seed connect timeout");

    drop(connection);
}

mod relay_connectivity;
