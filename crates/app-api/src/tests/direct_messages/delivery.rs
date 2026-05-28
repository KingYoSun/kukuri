use super::super::*;

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
