use super::super::*;

#[tokio::test]
async fn dm_open_does_not_duplicate_active_subscription_when_mutual_auto_subscribe_is_enabled() {
    let transport = Arc::new(StaticTransport::new(PeerSnapshot {
        connected: true,
        peer_count: 1,
        connected_peers: vec!["peer-b".into()],
        configured_peers: vec!["peer-b".into()],
        subscribed_topics: Vec::new(),
        active_path: Default::default(),
        fallback_peer_ids: Vec::new(),
        pending_events: 0,
        status_detail: "connected".into(),
        last_error: None,
        topic_diagnostics: Vec::new(),
    }));
    let hint_transport = Arc::new(CountingClosingHintTransport::default());
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let store = Arc::new(MemoryStore::default());
    let keys_local = generate_keys();
    let keys_peer = generate_keys();
    let local_pubkey = keys_local.public_key_hex();
    let peer_pubkey = keys_peer.public_key_hex();
    let follow_local_to_peer = parse_follow_edge(
        &build_follow_edge_envelope(
            &keys_local,
            &Pubkey::from(peer_pubkey.as_str()),
            FollowEdgeStatus::Active,
        )
        .expect("build follow edge local->peer"),
    )
    .expect("parse follow edge local->peer")
    .expect("follow edge local->peer");
    let follow_peer_to_local = parse_follow_edge(
        &build_follow_edge_envelope(
            &keys_peer,
            &Pubkey::from(local_pubkey.as_str()),
            FollowEdgeStatus::Active,
        )
        .expect("build follow edge peer->local"),
    )
    .expect("parse follow edge peer->local")
    .expect("follow edge peer->local");
    store
        .upsert_follow_edge(follow_local_to_peer)
        .await
        .expect("seed local->peer follow edge");
    store
        .upsert_follow_edge(follow_peer_to_local)
        .await
        .expect("seed peer->local follow edge");

    let app = AppService::new_with_services(
        store.clone(),
        store.clone(),
        transport.clone(),
        hint_transport.clone(),
        docs_sync,
        blob_service,
        keys_local,
    );

    app.open_direct_message(peer_pubkey.as_str())
        .await
        .expect("open direct message first time");
    sleep(Duration::from_millis(50)).await;
    app.open_direct_message(peer_pubkey.as_str())
        .await
        .expect("open direct message second time");
    sleep(Duration::from_millis(50)).await;

    assert_eq!(*hint_transport.subscribe_count.lock().await, 1);
}

#[tokio::test]
async fn dm_list_does_not_restart_active_subscription_after_open() {
    let transport = Arc::new(StaticTransport::new(PeerSnapshot {
        connected: true,
        peer_count: 1,
        connected_peers: vec!["peer-b".into()],
        configured_peers: vec!["peer-b".into()],
        subscribed_topics: Vec::new(),
        active_path: Default::default(),
        fallback_peer_ids: Vec::new(),
        pending_events: 0,
        status_detail: "connected".into(),
        last_error: None,
        topic_diagnostics: Vec::new(),
    }));
    let hint_transport = Arc::new(CountingClosingHintTransport::default());
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let store = Arc::new(MemoryStore::default());
    let keys_local = generate_keys();
    let keys_peer = generate_keys();
    let local_pubkey = keys_local.public_key_hex();
    let peer_pubkey = keys_peer.public_key_hex();
    let follow_local_to_peer = parse_follow_edge(
        &build_follow_edge_envelope(
            &keys_local,
            &Pubkey::from(peer_pubkey.as_str()),
            FollowEdgeStatus::Active,
        )
        .expect("build follow edge local->peer"),
    )
    .expect("parse follow edge local->peer")
    .expect("follow edge local->peer");
    let follow_peer_to_local = parse_follow_edge(
        &build_follow_edge_envelope(
            &keys_peer,
            &Pubkey::from(local_pubkey.as_str()),
            FollowEdgeStatus::Active,
        )
        .expect("build follow edge peer->local"),
    )
    .expect("parse follow edge peer->local")
    .expect("follow edge peer->local");
    store
        .upsert_follow_edge(follow_local_to_peer)
        .await
        .expect("seed local->peer follow edge");
    store
        .upsert_follow_edge(follow_peer_to_local)
        .await
        .expect("seed peer->local follow edge");

    let app = AppService::new_with_services(
        store.clone(),
        store.clone(),
        transport,
        hint_transport.clone(),
        docs_sync,
        blob_service,
        keys_local,
    );

    app.open_direct_message(peer_pubkey.as_str())
        .await
        .expect("open direct message");
    sleep(Duration::from_millis(50)).await;
    app.list_direct_messages()
        .await
        .expect("list direct messages first time");
    app.list_direct_messages()
        .await
        .expect("list direct messages second time");

    assert_eq!(*hint_transport.subscribe_count.lock().await, 1);
}

#[tokio::test]
async fn dm_import_peer_ticket_restarts_active_mutual_subscription() {
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let hint_transport = Arc::new(TrackingHintTransport::default());
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let store = Arc::new(MemoryStore::default());
    let keys_local = generate_keys();
    let keys_peer = generate_keys();
    let local_pubkey = keys_local.public_key_hex();
    let peer_pubkey = keys_peer.public_key_hex();
    let follow_local_to_peer = parse_follow_edge(
        &build_follow_edge_envelope(
            &keys_local,
            &Pubkey::from(peer_pubkey.as_str()),
            FollowEdgeStatus::Active,
        )
        .expect("build follow edge local->peer"),
    )
    .expect("parse follow edge local->peer")
    .expect("follow edge local->peer");
    let follow_peer_to_local = parse_follow_edge(
        &build_follow_edge_envelope(
            &keys_peer,
            &Pubkey::from(local_pubkey.as_str()),
            FollowEdgeStatus::Active,
        )
        .expect("build follow edge peer->local"),
    )
    .expect("parse follow edge peer->local")
    .expect("follow edge peer->local");
    store
        .upsert_follow_edge(follow_local_to_peer)
        .await
        .expect("seed local->peer follow edge");
    store
        .upsert_follow_edge(follow_peer_to_local)
        .await
        .expect("seed peer->local follow edge");

    let app = AppService::new_with_services(
        store.clone(),
        store.clone(),
        transport.clone(),
        hint_transport.clone(),
        docs_sync,
        blob_service,
        keys_local,
    );

    app.open_direct_message(peer_pubkey.as_str())
        .await
        .expect("open direct message");
    sleep(Duration::from_millis(50)).await;

    let topic = derive_direct_message_topic(app.keys.as_ref(), &Pubkey::from(peer_pubkey.as_str()))
        .expect("derive dm topic");
    assert!(
        app.direct_message_subscriptions
            .lock()
            .await
            .contains_key(peer_pubkey.as_str()),
        "open should create an active direct message subscription"
    );
    assert_eq!(*hint_transport.subscribe_count.lock().await, 1);

    app.import_peer_ticket("peer-ticket")
        .await
        .expect("import peer ticket");
    sleep(Duration::from_millis(50)).await;

    assert_eq!(*hint_transport.subscribe_count.lock().await, 2);
    assert_eq!(
        hint_transport.unsubscribed_topics.lock().await.as_slice(),
        &[topic.as_str().to_string()]
    );
}

#[tokio::test]
async fn dm_status_restarts_mutual_subscription_when_handle_is_missing() {
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let hint_transport = Arc::new(TrackingHintTransport::default());
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let store = Arc::new(MemoryStore::default());
    let keys_local = generate_keys();
    let keys_peer = generate_keys();
    let local_pubkey = keys_local.public_key_hex();
    let peer_pubkey = keys_peer.public_key_hex();
    let follow_local_to_peer = parse_follow_edge(
        &build_follow_edge_envelope(
            &keys_local,
            &Pubkey::from(peer_pubkey.as_str()),
            FollowEdgeStatus::Active,
        )
        .expect("build follow edge local->peer"),
    )
    .expect("parse follow edge local->peer")
    .expect("follow edge local->peer");
    let follow_peer_to_local = parse_follow_edge(
        &build_follow_edge_envelope(
            &keys_peer,
            &Pubkey::from(local_pubkey.as_str()),
            FollowEdgeStatus::Active,
        )
        .expect("build follow edge peer->local"),
    )
    .expect("parse follow edge peer->local")
    .expect("follow edge peer->local");
    store
        .upsert_follow_edge(follow_local_to_peer)
        .await
        .expect("seed local->peer follow edge");
    store
        .upsert_follow_edge(follow_peer_to_local)
        .await
        .expect("seed peer->local follow edge");

    let app = AppService::new_with_services(
        store.clone(),
        store.clone(),
        transport.clone(),
        hint_transport.clone(),
        docs_sync,
        blob_service,
        keys_local,
    );

    app.open_direct_message(peer_pubkey.as_str())
        .await
        .expect("open direct message");
    assert_eq!(*hint_transport.subscribe_count.lock().await, 1);

    if let Some(handle) = app
        .direct_message_subscriptions
        .lock()
        .await
        .remove(peer_pubkey.as_str())
    {
        handle.abort();
    }

    let status = app
        .get_direct_message_status(peer_pubkey.as_str())
        .await
        .expect("status should rebuild subscription");

    assert!(status.mutual);
    assert_eq!(*hint_transport.subscribe_count.lock().await, 2);
    assert!(
        app.direct_message_subscriptions
            .lock()
            .await
            .contains_key(peer_pubkey.as_str()),
        "status poll should restore the missing mutual direct-message subscription"
    );
}

#[tokio::test]
async fn dm_status_restarts_stale_active_subscription_when_topic_is_unjoined() {
    let keys_local = generate_keys();
    let keys_peer = generate_keys();
    let peer_pubkey = keys_peer.public_key_hex();
    let topic = derive_direct_message_topic(&keys_local, &Pubkey::from(peer_pubkey.as_str()))
        .expect("derive dm topic");
    let transport = Arc::new(StaticTransport::new(PeerSnapshot {
        connected: true,
        peer_count: 1,
        connected_peers: vec!["peer-b".into()],
        configured_peers: vec!["peer-b".into()],
        subscribed_topics: Vec::new(),
        active_path: Default::default(),
        fallback_peer_ids: Vec::new(),
        pending_events: 0,
        status_detail: "connected".into(),
        last_error: None,
        topic_diagnostics: Vec::new(),
    }));
    let hint_transport = Arc::new(TrackingHintTransport::default());
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let store = Arc::new(MemoryStore::default());
    let local_pubkey = keys_local.public_key_hex();
    let follow_local_to_peer = parse_follow_edge(
        &build_follow_edge_envelope(
            &keys_local,
            &Pubkey::from(peer_pubkey.as_str()),
            FollowEdgeStatus::Active,
        )
        .expect("build follow edge local->peer"),
    )
    .expect("parse follow edge local->peer")
    .expect("follow edge local->peer");
    let follow_peer_to_local = parse_follow_edge(
        &build_follow_edge_envelope(
            &keys_peer,
            &Pubkey::from(local_pubkey.as_str()),
            FollowEdgeStatus::Active,
        )
        .expect("build follow edge peer->local"),
    )
    .expect("parse follow edge peer->local")
    .expect("follow edge peer->local");
    store
        .upsert_follow_edge(follow_local_to_peer)
        .await
        .expect("seed local->peer follow edge");
    store
        .upsert_follow_edge(follow_peer_to_local)
        .await
        .expect("seed peer->local follow edge");

    let app = AppService::new_with_services(
        store.clone(),
        store.clone(),
        transport.clone(),
        hint_transport.clone(),
        docs_sync,
        blob_service,
        keys_local,
    );

    app.open_direct_message(peer_pubkey.as_str())
        .await
        .expect("open direct message");
    sleep(Duration::from_millis(50)).await;
    assert_eq!(*hint_transport.subscribe_count.lock().await, 1);
    {
        let mut peers = transport.peers.lock().await;
        peers.subscribed_topics = vec![format!("hint/{}", topic.as_str())];
        peers.topic_diagnostics = vec![TopicPeerSnapshot {
            topic: format!("hint/{}", topic.as_str()),
            joined: false,
            peer_count: 0,
            connected_peers: Vec::new(),
            configured_peer_ids: vec!["peer-b".into()],
            missing_peer_ids: vec!["peer-b".into()],
            active_path: Default::default(),
            rendezvous_peer_ids: Vec::new(),
            fallback_peer_ids: Vec::new(),
            last_received_at: None,
            status_detail: "Waiting for configured peers to join this topic".into(),
            last_error: None,
        }];
    }

    let status = app
        .get_direct_message_status(peer_pubkey.as_str())
        .await
        .expect("status should restart stale subscription");

    assert!(status.mutual);
    assert_eq!(*hint_transport.subscribe_count.lock().await, 2);
    assert_eq!(
        hint_transport.unsubscribed_topics.lock().await.as_slice(),
        &[topic.as_str().to_string()]
    );
}

#[tokio::test]
async fn direct_message_peer_count_falls_back_to_connected_peers_when_topic_diagnostic_is_missing()
{
    let transport = StaticTransport::new(PeerSnapshot {
        connected: true,
        peer_count: 1,
        connected_peers: vec!["peer-a".into()],
        configured_peers: vec!["peer-a".into()],
        subscribed_topics: vec!["kukuri:topic:demo".into()],
        active_path: Default::default(),
        fallback_peer_ids: Vec::new(),
        pending_events: 0,
        status_detail: "Connected".into(),
        last_error: None,
        topic_diagnostics: Vec::new(),
    });
    let keys_a = generate_keys();
    let keys_b = generate_keys();
    let topic =
        derive_direct_message_topic(&keys_a, &keys_b.public_key()).expect("direct message topic");

    let peer_count = direct_message_topic_peer_count(&transport, &topic)
        .await
        .expect("direct message peer count");

    assert_eq!(peer_count, 1);
}
