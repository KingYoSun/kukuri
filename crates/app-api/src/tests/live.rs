use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn late_joiner_backfills_live_session_manifest() {
    let _guard = iroh_integration_test_lock().lock_owned().await;
    let dir = tempdir().expect("tempdir");
    let stack_a = TestIrohStack::new(&dir.path().join("live-a")).await;
    let stack_b = TestIrohStack::new(&dir.path().join("live-b")).await;
    let store_a = Arc::new(MemoryStore::default());
    let store_b = Arc::new(MemoryStore::default());
    let app_a = app_with_iroh_services(store_a, &stack_a);
    let app_b = app_with_iroh_services(store_b, &stack_b);
    let topic = "kukuri:topic:live-late";

    let session_id = app_a
        .create_live_session(
            topic,
            CreateLiveSessionInput {
                title: "late live".into(),
                description: "watch along".into(),
            },
        )
        .await
        .expect("create live session");

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

    let received = timeout(Duration::from_secs(10), async {
        loop {
            let sessions = app_b
                .list_live_sessions(topic)
                .await
                .expect("list live sessions");
            if let Some(session) = sessions
                .into_iter()
                .find(|session| session.session_id == session_id)
            {
                return session;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("live session backfill timeout");

    assert_eq!(received.title, "late live");
    assert_eq!(received.status, LiveSessionStatus::Live);
}

#[tokio::test]
async fn live_presence_expires_without_heartbeat() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(FakeTransport::new("self", FakeNetwork::default()));
    let app = AppService::new(store.clone(), transport.clone());
    let topic = "kukuri:topic:presence-expiry";
    let session_id = app
        .create_live_session(
            topic,
            CreateLiveSessionInput {
                title: "presence".into(),
                description: "ttl".into(),
            },
        )
        .await
        .expect("create live session");

    let sessions = app
        .list_live_sessions(topic)
        .await
        .expect("list live sessions before presence");
    assert!(
        sessions
            .iter()
            .any(|session| session.session_id == session_id),
        "live session should be visible before presence is published"
    );

    transport
        .publish_hint(
            &TopicId::new(topic),
            GossipHint::LivePresence {
                topic_id: TopicId::new(topic),
                session_id: session_id.clone(),
                author: Pubkey::from("a".repeat(64)),
                ttl_ms: 100,
            },
        )
        .await
        .expect("publish live presence");

    timeout(Duration::from_secs(2), async {
        loop {
            let sessions = store
                .list_topic_live_sessions(topic)
                .await
                .expect("list cached live sessions");
            if sessions
                .iter()
                .any(|session| session.session_id == session_id && session.viewer_count == 1)
            {
                break;
            }
            sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("viewer count update timeout");

    sleep(Duration::from_millis(150)).await;
    let sessions = app
        .list_live_sessions(topic)
        .await
        .expect("list after expiry");
    let session = sessions
        .iter()
        .find(|session| session.session_id == session_id)
        .expect("session present");
    assert_eq!(session.viewer_count, 0);
}

#[tokio::test]
async fn ended_live_session_rejects_new_viewers() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(FakeTransport::new("self", FakeNetwork::default()));
    let app = AppService::new(store, transport);
    let topic = "kukuri:topic:ended-live";
    let session_id = app
        .create_live_session(
            topic,
            CreateLiveSessionInput {
                title: "ended".into(),
                description: "session".into(),
            },
        )
        .await
        .expect("create live session");
    app.end_live_session(topic, session_id.as_str())
        .await
        .expect("end live session");

    let error = app
        .join_live_session(topic, session_id.as_str())
        .await
        .expect_err("join should fail");
    assert!(error.to_string().contains("ended live session"));
}

#[tokio::test]
async fn muted_author_is_filtered_from_live_and_game_lists() {
    let (local_app, _local_keys, remote_app, remote_keys, _store, _docs_sync, _blob_service) =
        shared_apps_with_memory_services();
    let topic = "kukuri:topic:mute-live-game";
    let remote_pubkey = remote_keys.public_key_hex();

    let session_id = remote_app
        .create_live_session(
            topic,
            CreateLiveSessionInput {
                title: "muted live".into(),
                description: "hidden".into(),
            },
        )
        .await
        .expect("create live session");
    let room_id = remote_app
        .create_game_room(
            topic,
            CreateGameRoomInput {
                title: "muted room".into(),
                description: "hidden".into(),
                participants: vec!["Alice".into(), "Bob".into()],
            },
        )
        .await
        .expect("create game room");

    let live_before = local_app
        .list_live_sessions(topic)
        .await
        .expect("list live sessions before mute");
    let games_before = local_app
        .list_game_rooms(topic)
        .await
        .expect("list game rooms before mute");
    assert!(
        live_before
            .iter()
            .any(|session| session.session_id == session_id)
    );
    assert!(games_before.iter().any(|room| room.room_id == room_id));

    local_app
        .mute_author(remote_pubkey.as_str())
        .await
        .expect("mute live/game host");

    let live_after = local_app
        .list_live_sessions(topic)
        .await
        .expect("list live sessions after mute");
    let games_after = local_app
        .list_game_rooms(topic)
        .await
        .expect("list game rooms after mute");

    assert!(
        live_after
            .iter()
            .all(|session| session.session_id != session_id)
    );
    assert!(games_after.iter().all(|room| room.room_id != room_id));
}
