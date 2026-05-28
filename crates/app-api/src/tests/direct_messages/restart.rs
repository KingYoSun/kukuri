use super::super::*;

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
