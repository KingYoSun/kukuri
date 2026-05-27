use super::*;
use kukuri_core::{
    MetaverseAssetKind, MetaverseAvatarTransformV1, MetaverseRoomChatMessageV1,
    MetaverseRoomEventV1,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn game_room_score_update_replicates() {
    let _guard = iroh_integration_test_lock().lock_owned().await;
    let dir = tempdir().expect("tempdir");
    let stack_a = TestIrohStack::new(&dir.path().join("game-a")).await;
    let stack_b = TestIrohStack::new(&dir.path().join("game-b")).await;
    let store_a = Arc::new(MemoryStore::default());
    let store_b = Arc::new(MemoryStore::default());
    let app_a = app_with_iroh_services(store_a, &stack_a);
    let app_b = app_with_iroh_services(store_b, &stack_b);
    let topic = "kukuri:topic:game-sync";

    let ticket_a = app_a
        .peer_ticket()
        .await
        .expect("ticket a")
        .expect("ticket a value");
    let ticket_b = app_b
        .peer_ticket()
        .await
        .expect("ticket b")
        .expect("ticket b value");
    app_a.import_peer_ticket(&ticket_b).await.expect("import b");
    app_b.import_peer_ticket(&ticket_a).await.expect("import a");

    let room_id = app_a
        .create_game_room(
            topic,
            CreateGameRoomInput {
                title: "sync room".into(),
                description: "set".into(),
                participants: vec!["Alice".into(), "Bob".into()],
            },
        )
        .await
        .expect("create game room");
    app_a
        .update_game_room(
            topic,
            room_id.as_str(),
            UpdateGameRoomInput {
                status: GameRoomStatus::Running,
                phase_label: Some("Round 2".into()),
                scores: vec![
                    GameScoreView {
                        participant_id: "participant-1".into(),
                        label: "Alice".into(),
                        score: 2,
                    },
                    GameScoreView {
                        participant_id: "participant-2".into(),
                        label: "Bob".into(),
                        score: 1,
                    },
                ],
            },
        )
        .await
        .expect("update game room");

    let received = timeout(Duration::from_secs(60), async {
        loop {
            let rooms = app_b.list_game_rooms(topic).await.expect("list game rooms");
            if let Some(room) = rooms.into_iter().find(|room| room.room_id == room_id) {
                let alice_score = room
                    .scores
                    .iter()
                    .find(|score| score.label == "Alice")
                    .map(|score| score.score);
                if room.status == GameRoomStatus::Running
                    && room.phase_label.as_deref() == Some("Round 2")
                    && alice_score == Some(2)
                {
                    return room;
                }
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("game room replication timeout");

    assert_eq!(received.status, GameRoomStatus::Running);
    assert_eq!(received.phase_label.as_deref(), Some("Round 2"));
    assert_eq!(
        received
            .scores
            .iter()
            .find(|score| score.label == "Alice")
            .map(|score| score.score),
        Some(2)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn metaverse_room_events_replicate_between_iroh_peers() {
    let _guard = iroh_integration_test_lock().lock_owned().await;
    let dir = tempdir().expect("tempdir");
    let stack_a = TestIrohStack::new(&dir.path().join("meta-event-a")).await;
    let stack_b = TestIrohStack::new(&dir.path().join("meta-event-b")).await;
    let app_a = app_with_iroh_services(Arc::new(MemoryStore::default()), &stack_a);
    let app_b = app_with_iroh_services(Arc::new(MemoryStore::default()), &stack_b);
    let topic = "kukuri:topic:metaverse-iroh-events";

    let ticket_a = app_a
        .peer_ticket()
        .await
        .expect("ticket a")
        .expect("ticket a value");
    let ticket_b = app_b
        .peer_ticket()
        .await
        .expect("ticket b")
        .expect("ticket b value");
    app_a.import_peer_ticket(&ticket_b).await.expect("import b");
    app_b.import_peer_ticket(&ticket_a).await.expect("import a");

    let room_id = app_a
        .create_metaverse_room(
            topic,
            CreateMetaverseRoomInput {
                title: "iroh room".into(),
                description: "event transport".into(),
                max_peers: Some(4),
            },
        )
        .await
        .expect("create metaverse room");

    timeout(Duration::from_secs(60), async {
        loop {
            let rooms = app_b.list_game_rooms(topic).await.expect("list rooms");
            if rooms.iter().any(|room| room.room_id == room_id) {
                return;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("metaverse room discovery timeout");

    app_a
        .publish_metaverse_room_event(
            topic,
            PublishMetaverseRoomEventInput {
                room_id: room_id.clone(),
                peer_id: "peer-a".into(),
                seq: 11,
                event: MetaverseRoomEventV1::ChatMessage {
                    message: MetaverseRoomChatMessageV1 {
                        room_id: room_id.clone(),
                        message_id: "chat-iroh-1".into(),
                        author_peer_id: "peer-a".into(),
                        display_name: Some("Peer A".into()),
                        body: "hello over iroh".into(),
                        created_at: Utc::now().timestamp_millis(),
                    },
                },
            },
        )
        .await
        .expect("publish chat event");

    let received = timeout(Duration::from_secs(60), async {
        loop {
            let events = app_b
                .list_metaverse_room_events(topic, room_id.as_str(), None, Some(32))
                .await
                .expect("list events");
            if let Some(event) = events.into_iter().find(|event| event.content.seq == 11) {
                return event;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("metaverse event replication timeout");

    received.envelope.verify().expect("signed received event");
    assert!(matches!(
        received.content.event,
        MetaverseRoomEventV1::ChatMessage { .. }
    ));
}

#[tokio::test]
async fn finished_game_room_rejects_updates() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(FakeTransport::new("self", FakeNetwork::default()));
    let app = AppService::new(store, transport);
    let topic = "kukuri:topic:game-finished";
    let room_id = app
        .create_game_room(
            topic,
            CreateGameRoomInput {
                title: "finished room".into(),
                description: "set".into(),
                participants: vec!["Alice".into(), "Bob".into()],
            },
        )
        .await
        .expect("create game room");

    app.update_game_room(
        topic,
        room_id.as_str(),
        UpdateGameRoomInput {
            status: GameRoomStatus::Ended,
            phase_label: Some("Final".into()),
            scores: vec![
                GameScoreView {
                    participant_id: "participant-1".into(),
                    label: "Alice".into(),
                    score: 2,
                },
                GameScoreView {
                    participant_id: "participant-2".into(),
                    label: "Bob".into(),
                    score: 0,
                },
            ],
        },
    )
    .await
    .expect("finish room");

    let error = app
        .update_game_room(
            topic,
            room_id.as_str(),
            UpdateGameRoomInput {
                status: GameRoomStatus::Ended,
                phase_label: Some("After".into()),
                scores: vec![
                    GameScoreView {
                        participant_id: "participant-1".into(),
                        label: "Alice".into(),
                        score: 3,
                    },
                    GameScoreView {
                        participant_id: "participant-2".into(),
                        label: "Bob".into(),
                        score: 1,
                    },
                ],
            },
        )
        .await
        .expect_err("ended room update should fail");
    assert!(error.to_string().contains("ended game room"));
}

#[tokio::test]
async fn metaverse_room_uses_game_room_projection_without_scores() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(FakeTransport::new("self", FakeNetwork::default()));
    let app = AppService::new(store, transport);
    let topic = "kukuri:topic:metaverse";

    let room_id = app
        .create_metaverse_room(
            topic,
            CreateMetaverseRoomInput {
                title: "atrium".into(),
                description: "small social space".into(),
                max_peers: Some(8),
            },
        )
        .await
        .expect("create metaverse room");

    let rooms = app.list_game_rooms(topic).await.expect("list rooms");
    let room = rooms
        .into_iter()
        .find(|room| room.room_id == room_id)
        .expect("metaverse room in game projection");
    assert_eq!(room.room_kind, GameRoomKind::MetaverseRoom);
    assert!(room.scores.is_empty());
    assert_eq!(
        room.metaverse.as_ref().and_then(|state| state.max_peers),
        Some(8)
    );
    assert!(!room.manifest_blob_hash.trim().is_empty());

    app.update_metaverse_room(
        topic,
        room_id.as_str(),
        UpdateMetaverseRoomInput {
            status: GameRoomStatus::Running,
            shared_object_position: [50, 50, -240],
            shared_object_rotation: [0, 15, 0],
            shared_object_scale: [100, 100, 100],
        },
    )
    .await
    .expect("update metaverse object");

    let updated = app
        .list_game_rooms(topic)
        .await
        .expect("list updated rooms")
        .into_iter()
        .find(|room| room.room_id == room_id)
        .expect("updated metaverse room");
    assert_eq!(updated.status, GameRoomStatus::Running);
    assert_eq!(
        updated
            .metaverse
            .as_ref()
            .map(|state| state.scene.shared_object.position),
        Some([50, 50, -240])
    );
}

#[tokio::test]
async fn metaverse_room_events_are_signed_and_delivered_over_hint_transport() {
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let store_a = Arc::new(MemoryStore::default());
    let store_b = Arc::new(MemoryStore::default());
    let app_a = AppService::new(store_a, Arc::clone(&transport));
    let app_b = AppService::new(store_b, transport);
    let topic = "kukuri:topic:metaverse-events";

    let room_id = app_a
        .create_metaverse_room(
            topic,
            CreateMetaverseRoomInput {
                title: "events".into(),
                description: "transport".into(),
                max_peers: Some(4),
            },
        )
        .await
        .expect("create metaverse room");
    app_b
        .list_game_rooms(topic)
        .await
        .expect("start topic subscription");

    let local_event = app_a
        .publish_metaverse_room_event(
            topic,
            PublishMetaverseRoomEventInput {
                room_id: room_id.clone(),
                peer_id: "peer-a".into(),
                seq: 7,
                event: MetaverseRoomEventV1::AvatarTransform {
                    transform: MetaverseAvatarTransformV1 {
                        room_id: room_id.clone(),
                        peer_id: "peer-a".into(),
                        seq: 7,
                        position: [100, 0, -50],
                        rotation: [0, 90, 0],
                        animation: Some("walk".into()),
                        sent_at: Utc::now().timestamp_millis(),
                    },
                },
            },
        )
        .await
        .expect("publish metaverse event");
    local_event.envelope.verify().expect("signed local event");

    let received = timeout(Duration::from_secs(5), async {
        loop {
            let events = app_b
                .list_metaverse_room_events(topic, room_id.as_str(), None, Some(16))
                .await
                .expect("list metaverse events");
            if let Some(event) = events.into_iter().find(|event| event.content.seq == 7) {
                return event;
            }
            sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("metaverse event delivery timeout");

    received.envelope.verify().expect("signed received event");
    assert_eq!(received.content.room_id, room_id);
    assert_eq!(received.content.peer_id, "peer-a");
    assert!(matches!(
        received.content.event,
        MetaverseRoomEventV1::AvatarTransform { .. }
    ));
}

#[tokio::test]
async fn metaverse_room_event_rejects_mismatched_payload_identity() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(FakeTransport::new("self", FakeNetwork::default()));
    let app = AppService::new(store, transport);
    let topic = "kukuri:topic:metaverse-event-identity";

    let room_id = app
        .create_metaverse_room(
            topic,
            CreateMetaverseRoomInput {
                title: "identity".into(),
                description: "reject mismatch".into(),
                max_peers: Some(4),
            },
        )
        .await
        .expect("create metaverse room");

    let error = app
        .publish_metaverse_room_event(
            topic,
            PublishMetaverseRoomEventInput {
                room_id: room_id.clone(),
                peer_id: "peer-a".into(),
                seq: 1,
                event: MetaverseRoomEventV1::AvatarTransform {
                    transform: MetaverseAvatarTransformV1 {
                        room_id,
                        peer_id: "peer-b".into(),
                        seq: 1,
                        position: [0, 0, 0],
                        rotation: [0, 0, 0],
                        animation: Some("idle".into()),
                        sent_at: Utc::now().timestamp_millis(),
                    },
                },
            },
        )
        .await
        .expect_err("mismatched peer identity should be rejected");

    assert!(
        error
            .to_string()
            .contains("metaverse transform event identity")
    );
}

#[tokio::test]
async fn metaverse_room_shared_object_update_allows_non_owner_without_status_change() {
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let owner_store = Arc::new(MemoryStore::default());
    let peer_store = Arc::new(MemoryStore::default());
    let app_owner = AppService::new_with_services(
        owner_store.clone(),
        owner_store,
        transport.clone(),
        transport.clone(),
        docs_sync.clone(),
        blob_service.clone(),
        generate_keys(),
    );
    let app_peer = AppService::new_with_services(
        peer_store.clone(),
        peer_store,
        transport.clone(),
        transport,
        docs_sync,
        blob_service,
        generate_keys(),
    );
    let topic = "kukuri:topic:metaverse-object-non-owner";

    let room_id = app_owner
        .create_metaverse_room(
            topic,
            CreateMetaverseRoomInput {
                title: "shared object".into(),
                description: "participant updates".into(),
                max_peers: Some(4),
            },
        )
        .await
        .expect("create metaverse room");

    app_peer
        .update_metaverse_room(
            topic,
            room_id.as_str(),
            UpdateMetaverseRoomInput {
                status: GameRoomStatus::Waiting,
                shared_object_position: [75, 50, -120],
                shared_object_rotation: [0, 45, 0],
                shared_object_scale: [100, 100, 100],
            },
        )
        .await
        .expect("non-owner should update shared object");

    let updated = app_peer
        .list_game_rooms(topic)
        .await
        .expect("list updated rooms")
        .into_iter()
        .find(|room| room.room_id == room_id)
        .expect("updated metaverse room");
    assert_eq!(
        updated
            .metaverse
            .as_ref()
            .map(|state| state.scene.shared_object.position),
        Some([75, 50, -120])
    );

    let error = app_peer
        .update_metaverse_room(
            topic,
            room_id.as_str(),
            UpdateMetaverseRoomInput {
                status: GameRoomStatus::Running,
                shared_object_position: [100, 50, -120],
                shared_object_rotation: [0, 45, 0],
                shared_object_scale: [100, 100, 100],
            },
        )
        .await
        .expect_err("non-owner status change should fail");
    assert!(error.to_string().contains("owner can change"));
}

#[tokio::test]
async fn metaverse_avatar_asset_imports_to_blob_ref_without_event_bytes() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(FakeTransport::new("self", FakeNetwork::default()));
    let app = AppService::new(store, transport);
    let topic = "kukuri:topic:metaverse-asset";
    let room_id = app
        .create_metaverse_room(
            topic,
            CreateMetaverseRoomInput {
                title: "asset room".into(),
                description: "vrm".into(),
                max_peers: Some(4),
            },
        )
        .await
        .expect("create room");

    let asset = app
        .import_metaverse_room_asset(
            topic,
            ImportMetaverseRoomAssetInput {
                room_id: room_id.clone(),
                kind: MetaverseAssetKind::Vrm,
                mime_type: "model/vrm".into(),
                name: Some("avatar.vrm".into()),
                bytes: b"vrm-bytes".to_vec(),
            },
        )
        .await
        .expect("import avatar asset");

    assert_eq!(asset.kind, MetaverseAssetKind::Vrm);
    assert_eq!(asset.mime_type.as_deref(), Some("model/vrm"));
    assert_eq!(asset.size_bytes, Some(9));
    assert!(!asset.blob_hash.trim().is_empty());

    let payload = app
        .blob_media_payload(asset.blob_hash.as_str(), "model/vrm")
        .await
        .expect("blob payload")
        .expect("blob payload exists");
    assert_eq!(payload.mime, "model/vrm");
    assert!(!payload.bytes_base64.trim().is_empty());
}

#[tokio::test]
async fn metaverse_room_manifest_restores_after_restart_from_docs_and_blobs() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let keys = generate_keys();
    let app = AppService::new_with_services(
        store.clone(),
        store,
        transport.clone(),
        transport.clone(),
        docs_sync.clone(),
        blob_service.clone(),
        keys.clone(),
    );
    let topic = "kukuri:topic:metaverse-restart";
    let room_id = app
        .create_metaverse_room(
            topic,
            CreateMetaverseRoomInput {
                title: "restart room".into(),
                description: "restore".into(),
                max_peers: Some(6),
            },
        )
        .await
        .expect("create room");
    app.update_metaverse_room(
        topic,
        room_id.as_str(),
        UpdateMetaverseRoomInput {
            status: GameRoomStatus::Running,
            shared_object_position: [150, 50, -90],
            shared_object_rotation: [0, 30, 0],
            shared_object_scale: [120, 100, 80],
        },
    )
    .await
    .expect("update shared object");

    let restarted_store = Arc::new(MemoryStore::default());
    let restarted = AppService::new_with_services(
        restarted_store.clone(),
        restarted_store,
        transport,
        Arc::new(NoopHintTransport),
        docs_sync,
        blob_service,
        keys,
    );
    let restored = restarted
        .list_game_rooms(topic)
        .await
        .expect("list rooms after restart")
        .into_iter()
        .find(|room| room.room_id == room_id)
        .expect("restored metaverse room");

    assert_eq!(restored.room_kind, GameRoomKind::MetaverseRoom);
    assert_eq!(restored.status, GameRoomStatus::Running);
    assert_eq!(
        restored
            .metaverse
            .as_ref()
            .map(|state| state.scene.shared_object.position),
        Some([150, 50, -90])
    );
    assert!(!restored.manifest_blob_hash.trim().is_empty());
}
