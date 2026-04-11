use super::*;

#[tokio::test]
async fn dm_send_requires_mutual_relationship() {
    let (app, _, _, _) = local_app_with_memory_services();
    let peer_keys = generate_keys();

    let error = app
        .send_direct_message(
            peer_keys.public_key_hex().as_str(),
            Some("hello"),
            None,
            Vec::new(),
        )
        .await
        .expect_err("direct message send should require mutual relationship");

    assert!(
        error
            .to_string()
            .contains("direct message requires a mutual relationship")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dm_status_stays_enabled_during_concurrent_relationship_rebuilds() {
    let _guard = iroh_integration_test_lock().lock_owned().await;
    let tempdir = tempdir().expect("tempdir");
    let database_path = tempdir.path().join("dm-status.db");
    let app_store = Arc::new(
        SqliteStore::connect_file(&database_path)
            .await
            .expect("connect app store"),
    );
    let writer_store = Arc::new(
        SqliteStore::connect_file(&database_path)
            .await
            .expect("connect writer store"),
    );
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let app = AppService::new_with_services(
        app_store.clone(),
        app_store.clone(),
        transport,
        Arc::new(NoopHintTransport),
        Arc::new(MemoryDocsSync::default()),
        Arc::new(MemoryBlobService::default()),
        generate_keys(),
    );
    let local_pubkey = app.current_author_pubkey();
    let peer_keys = generate_keys();
    let peer_pubkey = peer_keys.public_key_hex();

    app_store
        .put_envelope(
            build_follow_edge_envelope(
                app.keys.as_ref(),
                &Pubkey::from(peer_pubkey.as_str()),
                FollowEdgeStatus::Active,
            )
            .expect("build local->peer follow"),
        )
        .await
        .expect("seed local->peer follow");
    app_store
        .put_envelope(
            build_follow_edge_envelope(
                &peer_keys,
                &Pubkey::from(local_pubkey.as_str()),
                FollowEdgeStatus::Active,
            )
            .expect("build peer->local follow"),
        )
        .await
        .expect("seed peer->local follow");

    app.rebuild_author_relationships()
        .await
        .expect("seed relationship projection");
    let initial_status = app
        .direct_message_status_view(peer_pubkey.as_str())
        .await
        .expect("initial dm status");
    assert!(initial_status.send_enabled);
    assert!(initial_status.mutual);

    let mut rows = (0..512)
        .map(|index| AuthorRelationshipProjectionRow {
            local_author_pubkey: local_pubkey.clone(),
            author_pubkey: format!("{index:064x}"),
            following: true,
            followed_by: true,
            mutual: true,
            friend_of_friend: false,
            friend_of_friend_via_pubkeys: Vec::new(),
            derived_at: index,
        })
        .collect::<Vec<_>>();
    rows.push(AuthorRelationshipProjectionRow {
        local_author_pubkey: local_pubkey.clone(),
        author_pubkey: peer_pubkey.clone(),
        following: true,
        followed_by: true,
        mutual: true,
        friend_of_friend: false,
        friend_of_friend_via_pubkeys: Vec::new(),
        derived_at: 999,
    });

    let keep_running = Arc::new(AtomicBool::new(true));
    let keep_running_for_task = Arc::clone(&keep_running);
    let writer_rows = rows.clone();
    let local_pubkey_for_task = local_pubkey.clone();
    let writer_task = tokio::spawn(async move {
        while keep_running_for_task.load(Ordering::SeqCst) {
            ProjectionStore::rebuild_author_relationships(
                writer_store.as_ref(),
                local_pubkey_for_task.as_str(),
                writer_rows.clone(),
            )
            .await
            .expect("rebuild relationships");
        }
    });

    let mut saw_disabled = false;
    for _ in 0..64 {
        let status = app
            .direct_message_status_view(peer_pubkey.as_str())
            .await
            .expect("dm status during rebuild");
        if !status.send_enabled || !status.mutual {
            saw_disabled = true;
            break;
        }
    }

    keep_running.store(false, Ordering::SeqCst);
    writer_task.await.expect("writer task");
    assert!(
        !saw_disabled,
        "direct message status should remain mutual while concurrent rebuilds are in flight",
    );
}

#[tokio::test]
async fn dm_open_does_not_duplicate_active_subscription_when_mutual_auto_subscribe_is_enabled() {
    let transport = Arc::new(StaticTransport::new(PeerSnapshot {
        connected: true,
        peer_count: 1,
        connected_peers: vec!["peer-b".into()],
        configured_peers: vec!["peer-b".into()],
        subscribed_topics: Vec::new(),
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
async fn dm_first_message_appears_in_recipient_conversation_list_without_opening_dm() {
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let hint_transport = Arc::new(TrackingHintTransport::default());
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let store_a = Arc::new(MemoryStore::default());
    let store_b = Arc::new(MemoryStore::default());
    let keys_a = generate_keys();
    let keys_b = generate_keys();
    let a_pubkey = keys_a.public_key_hex();
    let b_pubkey = keys_b.public_key_hex();
    let follow_a_to_b = parse_follow_edge(
        &build_follow_edge_envelope(
            &keys_a,
            &Pubkey::from(b_pubkey.as_str()),
            FollowEdgeStatus::Active,
        )
        .expect("build follow edge a->b"),
    )
    .expect("parse follow edge a->b")
    .expect("follow edge a->b");
    let follow_b_to_a = parse_follow_edge(
        &build_follow_edge_envelope(
            &keys_b,
            &Pubkey::from(a_pubkey.as_str()),
            FollowEdgeStatus::Active,
        )
        .expect("build follow edge b->a"),
    )
    .expect("parse follow edge b->a")
    .expect("follow edge b->a");

    store_a
        .upsert_follow_edge(follow_a_to_b.clone())
        .await
        .expect("seed follow edge a->b in store a");
    store_a
        .upsert_follow_edge(follow_b_to_a.clone())
        .await
        .expect("seed follow edge b->a in store a");
    store_b
        .upsert_follow_edge(follow_a_to_b)
        .await
        .expect("seed follow edge a->b in store b");
    store_b
        .upsert_follow_edge(follow_b_to_a)
        .await
        .expect("seed follow edge b->a in store b");

    let app_a = AppService::new_with_services(
        store_a.clone(),
        store_a,
        transport.clone(),
        hint_transport.clone(),
        docs_sync.clone(),
        blob_service.clone(),
        keys_a.clone(),
    );
    let app_b = AppService::new_with_services(
        store_b.clone(),
        store_b,
        transport.clone(),
        hint_transport.clone(),
        docs_sync,
        blob_service,
        keys_b.clone(),
    );

    app_a
        .rebuild_author_relationships()
        .await
        .expect("rebuild relationships for app a");
    app_b
        .rebuild_author_relationships()
        .await
        .expect("rebuild relationships for app b");
    assert!(
        app_b
            .direct_message_subscriptions
            .lock()
            .await
            .contains_key(a_pubkey.as_str()),
        "recipient should subscribe to mutual dm topics before opening the dm",
    );

    let message_id = app_a
        .send_direct_message(b_pubkey.as_str(), Some("hello from a"), None, Vec::new())
        .await
        .expect("send direct message");

    let conversation = timeout(Duration::from_secs(10), async {
        loop {
            let conversations = app_b
                .list_direct_messages()
                .await
                .expect("list recipient direct messages");
            if let Some(conversation) = conversations.into_iter().find(|item| {
                item.peer_pubkey == a_pubkey
                    && item.last_message_id.as_deref() == Some(message_id.as_str())
            }) {
                break conversation;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("wait for recipient conversation list update");
    assert_eq!(
        conversation.last_message_preview.as_deref(),
        Some("hello from a")
    );

    let delivered = app_b
        .list_direct_message_messages(a_pubkey.as_str(), None, 20)
        .await
        .expect("list recipient direct message timeline");
    assert!(
        delivered
            .items
            .iter()
            .any(|message| message.message_id == message_id),
        "recipient should see the delivered message after the conversation appears",
    );
}

#[tokio::test]
async fn dm_outbox_retry_stops_when_mutual_is_lost_and_resumes_when_it_returns() {
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
    let follow_peer_to_local_active = parse_follow_edge(
        &build_follow_edge_envelope(
            &keys_peer,
            &Pubkey::from(local_pubkey.as_str()),
            FollowEdgeStatus::Active,
        )
        .expect("build follow edge peer->local active"),
    )
    .expect("parse follow edge peer->local active")
    .expect("follow edge peer->local active");
    let follow_peer_to_local_inactive = parse_follow_edge(
        &build_follow_edge_envelope(
            &keys_peer,
            &Pubkey::from(local_pubkey.as_str()),
            FollowEdgeStatus::Revoked,
        )
        .expect("build follow edge peer->local inactive"),
    )
    .expect("parse follow edge peer->local inactive")
    .expect("follow edge peer->local inactive");

    store
        .upsert_follow_edge(follow_local_to_peer)
        .await
        .expect("seed follow edge local->peer");
    store
        .upsert_follow_edge(follow_peer_to_local_active.clone())
        .await
        .expect("seed follow edge peer->local");

    let app = AppService::new_with_services(
        store.clone(),
        store.clone(),
        transport.clone(),
        hint_transport,
        docs_sync,
        blob_service,
        keys_local.clone(),
    );

    app.rebuild_author_relationships()
        .await
        .expect("seed relationship projection");
    assert!(
        app.direct_message_subscriptions
            .lock()
            .await
            .contains_key(peer_pubkey.as_str()),
        "mutual peer should start with an active dm subscription",
    );

    let topic = derive_direct_message_topic(&keys_local, &Pubkey::from(peer_pubkey.as_str()))
        .expect("derive dm topic");
    let message_id = app
        .send_direct_message(
            peer_pubkey.as_str(),
            Some("queued while disconnected"),
            None,
            Vec::new(),
        )
        .await
        .expect("queue direct message while disconnected");
    let queued_outbox = store
        .list_direct_message_outbox()
        .await
        .expect("list queued outbox");
    assert_eq!(queued_outbox.len(), 1);
    assert_eq!(queued_outbox[0].message_id, message_id);
    assert_eq!(queued_outbox[0].last_attempt_at, None);

    store
        .upsert_follow_edge(follow_peer_to_local_inactive)
        .await
        .expect("drop peer->local follow edge");
    app.rebuild_author_relationships()
        .await
        .expect("rebuild relationships after mutual loss");
    assert!(
        !app.direct_message_subscriptions
            .lock()
            .await
            .contains_key(peer_pubkey.as_str()),
        "subscription should stop when mutual relationship is lost",
    );
    let disabled_status = app
        .get_direct_message_status(peer_pubkey.as_str())
        .await
        .expect("status after mutual loss");
    assert!(!disabled_status.send_enabled);
    assert_eq!(disabled_status.pending_outbox_count, 1);

    {
        let mut snapshot = transport.peers.lock().await;
        snapshot.connected = true;
        snapshot.peer_count = 1;
        snapshot.connected_peers = vec!["peer-b".into()];
        snapshot.topic_diagnostics = vec![TopicPeerSnapshot {
            topic: format!("hint/{}", topic.as_str()),
            joined: true,
            peer_count: 1,
            connected_peers: vec!["peer-b".into()],
            configured_peer_ids: vec!["peer-b".into()],
            missing_peer_ids: Vec::new(),
            last_received_at: None,
            status_detail: "connected".into(),
            last_error: None,
        }];
    }
    sleep(Duration::from_millis(
        DIRECT_MESSAGE_RETRY_INTERVAL_MS + 250,
    ))
    .await;
    let stopped_outbox = store
        .list_direct_message_outbox()
        .await
        .expect("list outbox while retry is stopped");
    assert_eq!(stopped_outbox[0].last_attempt_at, None);

    let follow_peer_to_local_restored = parse_follow_edge(
        &build_follow_edge_envelope(
            &keys_peer,
            &Pubkey::from(local_pubkey.as_str()),
            FollowEdgeStatus::Active,
        )
        .expect("build follow edge peer->local restored"),
    )
    .expect("parse follow edge peer->local restored")
    .expect("follow edge peer->local restored");
    store
        .upsert_follow_edge(follow_peer_to_local_restored)
        .await
        .expect("restore peer->local follow edge");
    app.rebuild_author_relationships()
        .await
        .expect("rebuild relationships after mutual restore");
    assert!(
        app.direct_message_subscriptions
            .lock()
            .await
            .contains_key(peer_pubkey.as_str()),
        "subscription should resume when mutual relationship returns",
    );
    let restored_status = app
        .get_direct_message_status(peer_pubkey.as_str())
        .await
        .expect("status after mutual restore");
    assert!(restored_status.send_enabled);

    timeout(Duration::from_secs(10), async {
        loop {
            let outbox = store
                .list_direct_message_outbox()
                .await
                .expect("list outbox after mutual restore");
            if outbox
                .iter()
                .any(|row| row.message_id == message_id && row.last_attempt_at.is_some())
            {
                break;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("wait for queued retry to resume after mutual restore");
}

#[tokio::test]
async fn dm_restart_resumes_pending_outbox_and_local_delete_prevents_duplicate_reinsert() {
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let hint_transport = Arc::new(TrackingHintTransport::default());
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let store_a = Arc::new(MemoryStore::default());
    let store_b = Arc::new(MemoryStore::default());
    let keys_a = generate_keys();
    let keys_b = generate_keys();
    let a_pubkey = keys_a.public_key_hex();
    let b_pubkey = keys_b.public_key_hex();
    let follow_a_to_b = parse_follow_edge(
        &build_follow_edge_envelope(
            &keys_a,
            &Pubkey::from(b_pubkey.as_str()),
            FollowEdgeStatus::Active,
        )
        .expect("build follow edge a->b"),
    )
    .expect("parse follow edge a->b")
    .expect("follow edge a->b");
    let follow_b_to_a = parse_follow_edge(
        &build_follow_edge_envelope(
            &keys_b,
            &Pubkey::from(a_pubkey.as_str()),
            FollowEdgeStatus::Active,
        )
        .expect("build follow edge b->a"),
    )
    .expect("parse follow edge b->a")
    .expect("follow edge b->a");

    store_a
        .upsert_follow_edge(follow_a_to_b.clone())
        .await
        .expect("seed follow edge a->b in store a");
    store_a
        .upsert_follow_edge(follow_b_to_a.clone())
        .await
        .expect("seed follow edge b->a in store a");
    store_b
        .upsert_follow_edge(follow_a_to_b)
        .await
        .expect("seed follow edge a->b in store b");
    store_b
        .upsert_follow_edge(follow_b_to_a)
        .await
        .expect("seed follow edge b->a in store b");

    let app_a = AppService::new_with_services(
        store_a.clone(),
        store_a.clone(),
        transport.clone(),
        hint_transport.clone(),
        docs_sync.clone(),
        blob_service.clone(),
        keys_a.clone(),
    );
    let app_b = AppService::new_with_services(
        store_b.clone(),
        store_b.clone(),
        transport.clone(),
        hint_transport.clone(),
        docs_sync.clone(),
        blob_service.clone(),
        keys_b.clone(),
    );

    app_b
        .open_direct_message(a_pubkey.as_str())
        .await
        .expect("recipient opens direct message");

    let message_id = app_a
        .send_direct_message(
            b_pubkey.as_str(),
            None,
            None,
            vec![pending_image_attachment(
                "image/png",
                tiny_png_bytes().as_slice(),
            )],
        )
        .await
        .expect("queue direct message while offline");
    let queued_status = app_a
        .get_direct_message_status(b_pubkey.as_str())
        .await
        .expect("queued status");
    assert_eq!(queued_status.pending_outbox_count, 1);
    let queued_outbox = store_a
        .list_direct_message_outbox()
        .await
        .expect("list queued outbox");
    assert_eq!(queued_outbox.len(), 1);
    let queued_frame = queued_outbox[0].clone();

    let initial_timeline = app_b
        .list_direct_message_messages(a_pubkey.as_str(), None, 20)
        .await
        .expect("initial recipient timeline");
    assert!(initial_timeline.items.is_empty());

    drop(app_a);

    let reopened_app_a = AppService::new_with_services(
        store_a.clone(),
        store_a.clone(),
        transport.clone(),
        hint_transport.clone(),
        docs_sync.clone(),
        blob_service.clone(),
        keys_a.clone(),
    );
    reopened_app_a
        .resume_direct_message_state()
        .await
        .expect("resume direct message state");
    assert!(
        reopened_app_a
            .direct_message_subscriptions
            .lock()
            .await
            .contains_key(b_pubkey.as_str()),
        "resume should restore the direct message subscription",
    );

    let topic = derive_direct_message_topic(&keys_a, &Pubkey::from(b_pubkey.as_str()))
        .expect("derive dm topic");
    {
        let mut snapshot = transport.peers.lock().await;
        snapshot.connected = true;
        snapshot.peer_count = 1;
        snapshot.connected_peers = vec!["peer-b".into()];
        snapshot.topic_diagnostics = vec![TopicPeerSnapshot {
            topic: format!("hint/{}", topic.as_str()),
            joined: true,
            peer_count: 1,
            connected_peers: vec!["peer-b".into()],
            configured_peer_ids: vec!["peer-b".into()],
            missing_peer_ids: Vec::new(),
            last_received_at: None,
            status_detail: "connected".into(),
            last_error: None,
        }];
    }
    let _published = AppService::flush_direct_message_outbox_for_peer_with_services(
        store_a.as_ref(),
        hint_transport.as_ref(),
        transport.as_ref(),
        a_pubkey.as_str(),
        &keys_a,
        b_pubkey.as_str(),
    )
    .await
    .expect("flush queued direct message after restart");

    let delivered = timeout(Duration::from_secs(10), async {
        loop {
            let timeline = app_b
                .list_direct_message_messages(a_pubkey.as_str(), None, 20)
                .await
                .expect("recipient timeline");
            if let Some(message) = timeline
                .items
                .iter()
                .find(|item| item.message_id == message_id)
            {
                break message.clone();
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("wait for delivered direct message");
    assert_eq!(delivered.text, "");
    assert_eq!(delivered.attachments.len(), 1);
    assert_eq!(delivered.attachments[0].role, "image_original");
    assert_eq!(delivered.attachments[0].mime, "image/png");
    assert!(!delivered.outgoing);
    assert!(delivered.delivered);

    timeout(Duration::from_secs(10), async {
        loop {
            let status = reopened_app_a
                .get_direct_message_status(b_pubkey.as_str())
                .await
                .expect("sender status");
            if status.pending_outbox_count == 0 {
                break;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("ack clears outbox");

    app_b
        .delete_direct_message_message(a_pubkey.as_str(), message_id.as_str())
        .await
        .expect("delete direct message locally");
    let after_delete = app_b
        .list_direct_message_messages(a_pubkey.as_str(), None, 20)
        .await
        .expect("timeline after delete");
    assert!(after_delete.items.is_empty());

    hint_transport
        .publish_hint(
            &topic,
            GossipHint::DirectMessageFrame {
                topic_id: topic.clone(),
                dm_id: queued_frame.dm_id.clone(),
                message_id: queued_frame.message_id.clone(),
                frame_hash: queued_frame.frame_blob_hash.clone(),
            },
        )
        .await
        .expect("republish duplicate direct message frame");
    sleep(Duration::from_millis(200)).await;

    let after_duplicate = app_b
        .list_direct_message_messages(a_pubkey.as_str(), None, 20)
        .await
        .expect("timeline after duplicate frame");
    assert!(after_duplicate.items.is_empty());
}

#[tokio::test]
async fn dm_restart_surfaces_queued_message_in_conversation_list_without_reopening_dm() {
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let hint_transport = Arc::new(TrackingHintTransport::default());
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let store_a = Arc::new(MemoryStore::default());
    let store_b = Arc::new(MemoryStore::default());
    let keys_a = generate_keys();
    let keys_b = generate_keys();
    let a_pubkey = keys_a.public_key_hex();
    let b_pubkey = keys_b.public_key_hex();
    let follow_a_to_b = parse_follow_edge(
        &build_follow_edge_envelope(
            &keys_a,
            &Pubkey::from(b_pubkey.as_str()),
            FollowEdgeStatus::Active,
        )
        .expect("build follow edge a->b"),
    )
    .expect("parse follow edge a->b")
    .expect("follow edge a->b");
    let follow_b_to_a = parse_follow_edge(
        &build_follow_edge_envelope(
            &keys_b,
            &Pubkey::from(a_pubkey.as_str()),
            FollowEdgeStatus::Active,
        )
        .expect("build follow edge b->a"),
    )
    .expect("parse follow edge b->a")
    .expect("follow edge b->a");

    store_a
        .upsert_follow_edge(follow_a_to_b.clone())
        .await
        .expect("seed follow edge a->b in store a");
    store_a
        .upsert_follow_edge(follow_b_to_a.clone())
        .await
        .expect("seed follow edge b->a in store a");
    store_b
        .upsert_follow_edge(follow_a_to_b)
        .await
        .expect("seed follow edge a->b in store b");
    store_b
        .upsert_follow_edge(follow_b_to_a)
        .await
        .expect("seed follow edge b->a in store b");

    let app_a = AppService::new_with_services(
        store_a.clone(),
        store_a.clone(),
        transport.clone(),
        hint_transport.clone(),
        docs_sync.clone(),
        blob_service.clone(),
        keys_a.clone(),
    );
    let app_b = AppService::new_with_services(
        store_b.clone(),
        store_b.clone(),
        transport.clone(),
        hint_transport.clone(),
        docs_sync.clone(),
        blob_service.clone(),
        keys_b.clone(),
    );

    app_b
        .open_direct_message(a_pubkey.as_str())
        .await
        .expect("recipient opens direct message before restart");

    let message_id = app_a
        .send_direct_message(
            b_pubkey.as_str(),
            Some("offline video"),
            None,
            vec![
                pending_video_attachment(AssetRole::VideoManifest, "video/mp4", b"restart-video"),
                pending_video_attachment(
                    AssetRole::VideoPoster,
                    "image/jpeg",
                    b"restart-video-poster",
                ),
            ],
        )
        .await
        .expect("queue direct message while offline");

    drop(app_a);
    drop(app_b);

    let reopened_app_b = AppService::new_with_services(
        store_b.clone(),
        store_b.clone(),
        transport.clone(),
        hint_transport.clone(),
        docs_sync.clone(),
        blob_service.clone(),
        keys_b.clone(),
    );
    reopened_app_b
        .resume_direct_message_state()
        .await
        .expect("resume recipient direct message state");

    let reopened_app_a = AppService::new_with_services(
        store_a.clone(),
        store_a.clone(),
        transport.clone(),
        hint_transport.clone(),
        docs_sync.clone(),
        blob_service.clone(),
        keys_a.clone(),
    );
    reopened_app_a
        .resume_direct_message_state()
        .await
        .expect("resume sender direct message state");

    let topic = derive_direct_message_topic(&keys_a, &Pubkey::from(b_pubkey.as_str()))
        .expect("derive dm topic");
    {
        let mut snapshot = transport.peers.lock().await;
        snapshot.connected = true;
        snapshot.peer_count = 1;
        snapshot.connected_peers = vec!["peer-b".into()];
        snapshot.topic_diagnostics = vec![TopicPeerSnapshot {
            topic: format!("hint/{}", topic.as_str()),
            joined: true,
            peer_count: 1,
            connected_peers: vec!["peer-b".into()],
            configured_peer_ids: vec!["peer-b".into()],
            missing_peer_ids: Vec::new(),
            last_received_at: None,
            status_detail: "connected".into(),
            last_error: None,
        }];
    }

    let conversation = timeout(Duration::from_secs(10), async {
        loop {
            let conversations = reopened_app_b
                .list_direct_messages()
                .await
                .expect("list recipient direct messages after restart");
            if let Some(conversation) = conversations.into_iter().find(|item| {
                item.peer_pubkey == a_pubkey
                    && item.last_message_id.as_deref() == Some(message_id.as_str())
            }) {
                break conversation;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("wait for recipient conversation list update after restart");
    assert_eq!(
        conversation.last_message_preview.as_deref(),
        Some("offline video")
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
