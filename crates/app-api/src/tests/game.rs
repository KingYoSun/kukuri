use super::*;

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
