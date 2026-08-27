//! WP-S6 T8: live_sessions(viewer_count = live_presence 集計)と game_rooms の
//! sqlite/memory 突き合わせ。
//! live_presence は「複数 topic × 同一 (channel_id, session_id, author_pubkey)」を
//! 必ずシナリオに含める(memory 側キーの topic_id 欠落 divergence を突く — T7 の fix 対象)。

use super::*;

const TOPIC_A: &str = "kukuri:topic:parity-live-a";
const TOPIC_B: &str = "kukuri:topic:parity-live-b";

#[derive(Debug, PartialEq)]
struct LiveSessionScenarioResult {
    topic_a_initial: Vec<LiveSessionProjectionRow>,
    topic_b_initial: Vec<LiveSessionProjectionRow>,
    topic_a_after_expire: Vec<LiveSessionProjectionRow>,
    topic_a_after_clear_topic_b: Vec<LiveSessionProjectionRow>,
    topic_b_after_clear_topic_b: Vec<LiveSessionProjectionRow>,
}

async fn live_session_scenario<S: Store + ProjectionStore>(store: &S) -> LiveSessionScenarioResult {
    let alice = "a".repeat(64);
    let bob = "b".repeat(64);
    let carol = "c".repeat(64);

    let mut ended = parity_live_session(
        "sess-ended",
        TOPIC_A,
        "ch-alt",
        LiveSessionStatus::Ended,
        90,
    );
    ended.ended_at = Some(95);
    for row in [
        parity_live_session(
            "sess-live",
            TOPIC_A,
            "ch-main",
            LiveSessionStatus::Live,
            100,
        ),
        parity_live_session(
            "sess-sched",
            TOPIC_A,
            "ch-main",
            LiveSessionStatus::Scheduled,
            100,
        ),
        ended,
        parity_live_session(
            "sess-other",
            TOPIC_B,
            "ch-main",
            LiveSessionStatus::Live,
            50,
        ),
    ] {
        LiveGameProjectionStore::upsert_live_session_cache(store, row)
            .await
            .expect("LiveGameProjectionStore::upsert_live_session_cache");
    }

    // presence の投入順が重要: alice の同一キー更新(2 回目)の後で、
    // topic-b から同一 (channel_id, session_id, author_pubkey) を upsert する。
    // sqlite は (topic, channel, session, author) 単位で 2 行を保持するが、
    // topic_id 欠落キーの memory 実装は topic-a の alice を上書きしてしまう。
    let presence = [
        (
            TOPIC_A,
            "ch-main",
            "sess-live",
            alice.as_str(),
            1_000_i64,
            10_i64,
        ),
        (TOPIC_A, "ch-main", "sess-live", alice.as_str(), 2_000, 20), // 同一キー更新
        (TOPIC_A, "ch-main", "sess-live", bob.as_str(), 1_000, 21),
        (TOPIC_A, "ch-main", "sess-live", carol.as_str(), 500, 22),
        (TOPIC_B, "ch-main", "sess-live", alice.as_str(), 1_000, 23), // 複数 topic × 同一 (channel, session, author)
        (TOPIC_A, "ch-main", "sess-sched", alice.as_str(), 1_000, 24),
        (TOPIC_A, "ch-alt", "sess-ended", alice.as_str(), 1_000, 25),
    ];
    for (topic_id, channel_id, session_id, author, expires_at, updated_at) in presence {
        LiveGameProjectionStore::upsert_live_presence(
            store, topic_id, channel_id, session_id, author, expires_at, updated_at,
        )
        .await
        .expect("LiveGameProjectionStore::upsert_live_presence");
    }

    let topic_a_initial = LiveGameProjectionStore::list_topic_live_sessions(store, TOPIC_A)
        .await
        .expect("list topic-a sessions initial");
    let topic_b_initial = LiveGameProjectionStore::list_topic_live_sessions(store, TOPIC_B)
        .await
        .expect("list topic-b sessions initial");

    // expires_at <= 500 を掃除(境界値 500 ちょうどの carol が消える)
    LiveGameProjectionStore::clear_expired_live_presence(store, 500)
        .await
        .expect("clear expired presence");
    let topic_a_after_expire = LiveGameProjectionStore::list_topic_live_sessions(store, TOPIC_A)
        .await
        .expect("list topic-a sessions after expire");

    // topic-b の presence だけ消える(topic-a の viewer_count は不変)
    LiveGameProjectionStore::clear_topic_live_presence(store, TOPIC_B)
        .await
        .expect("clear topic-b presence");

    LiveSessionScenarioResult {
        topic_a_initial,
        topic_b_initial,
        topic_a_after_expire,
        topic_a_after_clear_topic_b: LiveGameProjectionStore::list_topic_live_sessions(
            store, TOPIC_A,
        )
        .await
        .expect("list topic-a sessions after clear"),
        topic_b_after_clear_topic_b: LiveGameProjectionStore::list_topic_live_sessions(
            store, TOPIC_B,
        )
        .await
        .expect("list topic-b sessions after clear"),
    }
}

fn session_viewer_counts(rows: &[LiveSessionProjectionRow]) -> Vec<(String, usize)> {
    rows.iter()
        .map(|row| (row.session_id.clone(), row.viewer_count))
        .collect()
}

#[tokio::test]
async fn live_sessions_and_presence_match_between_backends() {
    let sqlite = SqliteStore::connect_memory().await.expect("sqlite store");
    let memory = MemoryStore::default();
    let from_sqlite = live_session_scenario(&sqlite).await;
    let from_memory = live_session_scenario(&memory).await;
    assert_eq!(from_sqlite, from_memory);

    // sanity: sqlite 実測(started_at DESC, session_id DESC。100 の tie は id 降順)
    // sess-live の viewer は alice(更新済み)+ bob + carol の 3。ended は常に 0。
    assert_eq!(
        session_viewer_counts(&from_sqlite.topic_a_initial),
        vec![
            ("sess-sched".to_string(), 1),
            ("sess-live".to_string(), 3),
            ("sess-ended".to_string(), 0),
        ],
    );
    assert_eq!(
        session_viewer_counts(&from_sqlite.topic_b_initial),
        vec![("sess-other".to_string(), 0)],
    );
    // expires_at == 500 の carol は掃除される
    assert_eq!(
        session_viewer_counts(&from_sqlite.topic_a_after_expire),
        vec![
            ("sess-sched".to_string(), 1),
            ("sess-live".to_string(), 2),
            ("sess-ended".to_string(), 0),
        ],
    );
    // topic-b の掃除は topic-a の presence に影響しない
    assert_eq!(
        session_viewer_counts(&from_sqlite.topic_a_after_clear_topic_b),
        vec![
            ("sess-sched".to_string(), 1),
            ("sess-live".to_string(), 2),
            ("sess-ended".to_string(), 0),
        ],
    );
    assert_eq!(
        session_viewer_counts(&from_sqlite.topic_b_after_clear_topic_b),
        vec![("sess-other".to_string(), 0)],
    );
}

#[derive(Debug, PartialEq)]
struct GameRoomScenarioResult {
    topic_rooms: Vec<GameRoomProjectionRow>,
    other_topic_rooms: Vec<GameRoomProjectionRow>,
}

async fn game_room_scenario<S: Store + ProjectionStore>(store: &S) -> GameRoomScenarioResult {
    let topic = "kukuri:topic:parity-game";
    let other_topic = "kukuri:topic:parity-game-other";

    let mut beta = parity_game_room("room-beta", topic, GameRoomStatus::Running, 100);
    beta.phase_label = Some("round-2".into());
    beta.scores = parity_game_scores();
    let mut meta = parity_game_room("room-meta", topic, GameRoomStatus::Ended, 90);
    meta.room_kind = GameRoomKind::MetaverseRoom;
    meta.metaverse = Some(parity_metaverse_state());
    for row in [
        parity_game_room("room-alpha", topic, GameRoomStatus::Waiting, 100),
        beta,
        meta,
        parity_game_room("room-other", other_topic, GameRoomStatus::Waiting, 80),
    ] {
        LiveGameProjectionStore::upsert_game_room_cache(store, row)
            .await
            .expect("LiveGameProjectionStore::upsert_game_room_cache");
    }
    // 同一 room_id の再 upsert(status 更新経路)
    LiveGameProjectionStore::upsert_game_room_cache(
        store,
        parity_game_room("room-alpha", topic, GameRoomStatus::Paused, 100),
    )
    .await
    .expect("upsert game room update");

    GameRoomScenarioResult {
        topic_rooms: LiveGameProjectionStore::list_topic_game_rooms(store, topic)
            .await
            .expect("list topic game rooms"),
        other_topic_rooms: LiveGameProjectionStore::list_topic_game_rooms(store, other_topic)
            .await
            .expect("list other topic game rooms"),
    }
}

#[tokio::test]
async fn game_rooms_match_between_backends() {
    let sqlite = SqliteStore::connect_memory().await.expect("sqlite store");
    let memory = MemoryStore::default();
    let from_sqlite = game_room_scenario(&sqlite).await;
    let from_memory = game_room_scenario(&memory).await;
    assert_eq!(from_sqlite, from_memory);

    // sanity: sqlite 実測(updated_at DESC, room_id DESC。100 の tie は id 降順)
    assert_eq!(
        from_sqlite
            .topic_rooms
            .iter()
            .map(|row| (
                row.room_id.clone(),
                row.status.clone(),
                row.room_kind.clone()
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                "room-beta".to_string(),
                GameRoomStatus::Running,
                GameRoomKind::ScoreGame,
            ),
            (
                "room-alpha".to_string(),
                GameRoomStatus::Paused,
                GameRoomKind::ScoreGame,
            ),
            (
                "room-meta".to_string(),
                GameRoomStatus::Ended,
                GameRoomKind::MetaverseRoom,
            ),
        ],
    );
    assert_eq!(from_sqlite.topic_rooms[0].scores, parity_game_scores());
    assert_eq!(
        from_sqlite.topic_rooms[2].metaverse,
        Some(parity_metaverse_state()),
    );
    // sanity: serde 写像の生リテラル固定(fixture ビルダーを経由しない直値。
    // scores_json / metaverse_json ⇄ struct の対応が変われば fixture 側と
    // 同時にズレてもここが割れる)
    let scores = &from_sqlite.topic_rooms[0].scores;
    assert_eq!(
        (
            scores[0].participant_id.as_str(),
            scores[0].label.as_str(),
            scores[0].score,
        ),
        ("player-1", "Player One", 120),
    );
    assert_eq!(
        (scores[1].participant_id.as_str(), scores[1].score),
        ("player-2", -5),
    );
    let metaverse = from_sqlite.topic_rooms[2]
        .metaverse
        .as_ref()
        .expect("metaverse state");
    assert_eq!(metaverse.world_version, 3);
    assert_eq!(
        metaverse.dome.customization.persistent_props[0].prop_id,
        "shared-cube"
    );
    assert_eq!(
        metaverse.dome.customization.persistent_props[0].position,
        [1, 2, 3]
    );
    assert_eq!(metaverse.chat_history[0].body, "hello");
    assert_eq!(
        from_sqlite
            .other_topic_rooms
            .iter()
            .map(|row| row.room_id.clone())
            .collect::<Vec<_>>(),
        vec!["room-other".to_string()],
    );
}

async fn dome_connection_projection_scenario<S: Store + ProjectionStore>(
    store: &S,
) -> Option<DomeConnectionProjectionRow> {
    let first = DomeConnectionProjectionRow {
        context_id: "topic:kukuri:topic:dome-parity".into(),
        topic_id: "kukuri:topic:dome-parity".into(),
        channel_id: String::new(),
        snapshot_json: r#"{"components":[]}"#.into(),
        topology_digest: "digest-1".into(),
        derived_at: 10,
        projection_version: 1,
    };
    LiveGameProjectionStore::upsert_dome_connection_projection(store, first)
        .await
        .expect("upsert initial Dome Connection projection");
    let updated = DomeConnectionProjectionRow {
        snapshot_json: r#"{"components":["dome-a"]}"#.into(),
        topology_digest: "digest-2".into(),
        derived_at: 20,
        ..LiveGameProjectionStore::get_dome_connection_projection(
            store,
            "topic:kukuri:topic:dome-parity",
        )
        .await
        .expect("get initial Dome Connection projection")
        .expect("initial Dome Connection projection")
    };
    LiveGameProjectionStore::upsert_dome_connection_projection(store, updated)
        .await
        .expect("upsert updated Dome Connection projection");
    LiveGameProjectionStore::get_dome_connection_projection(store, "topic:kukuri:topic:dome-parity")
        .await
        .expect("get updated Dome Connection projection")
}

#[tokio::test]
async fn dome_connection_projection_matches_between_backends() {
    let sqlite = SqliteStore::connect_memory().await.expect("sqlite store");
    let memory = MemoryStore::default();
    let from_sqlite = dome_connection_projection_scenario(&sqlite).await;
    let from_memory = dome_connection_projection_scenario(&memory).await;
    assert_eq!(from_sqlite, from_memory);
    let projection = from_sqlite.expect("Dome Connection projection");
    assert_eq!(projection.topology_digest, "digest-2");
    assert_eq!(projection.derived_at, 20);
}
