//! WP-S6 T4【contract】row_mapping round-trip 層 — live / game 系
//! (後続 WP-H1 の安全網)。
//!
//! 対象: live_session_cache(row_to_live_session_projection —
//! viewer_count はテーブル列ではなく live_presence_cache の COUNT を返す
//! SELECT 計算列。ended は常に 0)/ game_room_cache
//! (row_to_game_room_projection — scores_json / metaverse_json)。
//! viewer_count は必ず trait メソッド経由で読む(生 SQL で
//! live_session_cache を読まない)。
//! 分割元の全体説明は row_mapping_roundtrip.rs の冒頭を参照。

use super::*;
use kukuri_core::{
    GameRoomKind, GameRoomStatus, GameScoreEntry, LiveSessionStatus, MetaverseAssetKind,
    MetaverseAssetRef, MetaversePrimitive, MetaverseRoomChatMessageV1, MetaverseRoomSceneV1,
    MetaverseRoomSpawnV1, MetaverseRoomStateV1, SharedRoomObjectV1,
};

// ---------------------------------------------------------------------------
// live_session_cache(row_to_live_session_projection + viewer_count 計算列)
// ---------------------------------------------------------------------------

const LIVE_TOPIC: &str = "kukuri:topic:live-rt";

/// max fixture: ended_at=Some。viewer_count=99 は永続化されず、読み出しは
/// live_presence_cache の COUNT に置き換わる(このテストでは 2)。
fn live_session_max() -> LiveSessionProjectionRow {
    LiveSessionProjectionRow {
        session_id: "session-max".into(),
        topic_id: LIVE_TOPIC.into(),
        channel_id: "ch:live".into(),
        host_pubkey: "a".repeat(64),
        title: "配信タイトル 🎥".into(),
        description: "説明 description".into(),
        status: LiveSessionStatus::Live,
        started_at: 3_000,
        ended_at: Some(4_000),
        updated_at: 3_500,
        source_replica_id: ReplicaId::new("topic::kukuri:topic:live-rt"),
        source_key: "live/session-max/manifest".into(),
        manifest_blob_hash: BlobHash::new("a".repeat(64)),
        derived_at: 3_600,
        projection_version: 5,
        viewer_count: 99,
    }
}

/// min fixture: ended_at=None・presence なし → viewer_count=0。
fn live_session_min() -> LiveSessionProjectionRow {
    LiveSessionProjectionRow {
        session_id: "session-min".into(),
        topic_id: LIVE_TOPIC.into(),
        channel_id: "public".into(),
        host_pubkey: "b".repeat(64),
        title: String::new(),
        description: String::new(),
        status: LiveSessionStatus::Scheduled,
        started_at: 2_000,
        ended_at: None,
        updated_at: 2_000,
        source_replica_id: ReplicaId::new("topic::kukuri:topic:live-rt"),
        source_key: "live/session-min/manifest".into(),
        manifest_blob_hash: BlobHash::new("b".repeat(64)),
        derived_at: 2_100,
        projection_version: 1,
        viewer_count: 0,
    }
}

/// ended fixture: status=Ended は presence が居ても viewer_count=0。
fn live_session_ended() -> LiveSessionProjectionRow {
    LiveSessionProjectionRow {
        session_id: "session-ended".into(),
        topic_id: LIVE_TOPIC.into(),
        channel_id: "ch:live".into(),
        host_pubkey: "c".repeat(64),
        title: "終了済み".into(),
        description: "ended".into(),
        status: LiveSessionStatus::Ended,
        started_at: 1_000,
        ended_at: Some(1_500),
        updated_at: 1_500,
        source_replica_id: ReplicaId::new("topic::kukuri:topic:live-rt"),
        source_key: "live/session-ended/manifest".into(),
        manifest_blob_hash: BlobHash::new("c".repeat(64)),
        derived_at: 1_600,
        projection_version: 2,
        viewer_count: 0,
    }
}

#[tokio::test]
async fn live_session_roundtrip_preserves_all_columns_and_computes_viewer_count() {
    let store = SqliteStore::connect_memory().await.expect("sqlite store");
    ProjectionStore::upsert_live_session_cache(&store, live_session_max())
        .await
        .expect("upsert max session");
    ProjectionStore::upsert_live_session_cache(&store, live_session_min())
        .await
        .expect("upsert min session");
    ProjectionStore::upsert_live_session_cache(&store, live_session_ended())
        .await
        .expect("upsert ended session");

    // viewer_count の分子は (topic_id, channel_id, session_id) 完全一致の
    // presence のみ。channel / topic 違いは数えない。
    ProjectionStore::upsert_live_presence(
        &store,
        LIVE_TOPIC,
        "ch:live",
        "session-max",
        "viewer-1",
        10_000,
        3_000,
    )
    .await
    .expect("presence viewer-1");
    ProjectionStore::upsert_live_presence(
        &store,
        LIVE_TOPIC,
        "ch:live",
        "session-max",
        "viewer-2",
        10_000,
        3_001,
    )
    .await
    .expect("presence viewer-2");
    // channel 不一致 → 数えない。
    ProjectionStore::upsert_live_presence(
        &store,
        LIVE_TOPIC,
        "ch:other",
        "session-max",
        "viewer-3",
        10_000,
        3_002,
    )
    .await
    .expect("presence viewer-3");
    // topic 不一致 → 数えない。
    ProjectionStore::upsert_live_presence(
        &store,
        "kukuri:topic:live-other",
        "ch:live",
        "session-max",
        "viewer-4",
        10_000,
        3_003,
    )
    .await
    .expect("presence viewer-4");
    // ended セッションは presence が居ても 0。
    ProjectionStore::upsert_live_presence(
        &store,
        LIVE_TOPIC,
        "ch:live",
        "session-ended",
        "viewer-5",
        10_000,
        3_004,
    )
    .await
    .expect("presence viewer-5");

    let listed = ProjectionStore::list_topic_live_sessions(&store, LIVE_TOPIC)
        .await
        .expect("list live sessions");

    // put 時の viewer_count=99 は破棄され、COUNT(=2)が返る。
    let mut expected_max = live_session_max();
    expected_max.viewer_count = 2;
    // 現挙動(観測値): ended_at(i64)は filter が無いため、None で put しても
    // Some(0) で読み出される。
    let mut expected_min = live_session_min();
    expected_min.ended_at = Some(0);
    // started_at DESC, session_id DESC の順序ごと全列固定。
    assert_eq!(
        listed,
        vec![expected_max, expected_min, live_session_ended()]
    );
}

// ---------------------------------------------------------------------------
// game_room_cache(row_to_game_room_projection)
// ---------------------------------------------------------------------------

const GAME_TOPIC: &str = "kukuri:topic:game-rt";

/// metaverse_json に入る非自明な入れ子 fixture(全 Option=Some・全 Vec 非空)。
fn metaverse_state() -> MetaverseRoomStateV1 {
    MetaverseRoomStateV1 {
        world_version: 7,
        max_peers: Some(16),
        scene: MetaverseRoomSceneV1 {
            ground: "grid".into(),
            shared_object: SharedRoomObjectV1 {
                object_id: "shared-1".into(),
                asset_ref: Some(MetaverseAssetRef {
                    kind: MetaverseAssetKind::Glb,
                    blob_hash: "c".repeat(64),
                    mime_type: Some("model/gltf-binary".into()),
                    size_bytes: Some(2_048),
                    name: Some("チェア".into()),
                }),
                primitive_fallback: MetaversePrimitive::Cube,
                position: [1, 2, 3],
                rotation: [0, 90, 0],
                scale: [1, 1, 1],
                updated_by: "d".repeat(64).into(),
                updated_at: 1_700_000_000_400,
            },
        },
        default_spawn: MetaverseRoomSpawnV1 {
            position: [0, 0, 5],
            rotation: [0, 180, 0],
        },
        asset_refs: vec![MetaverseAssetRef {
            kind: MetaverseAssetKind::Vrm,
            blob_hash: "e".repeat(64),
            mime_type: None,
            size_bytes: None,
            name: None,
        }],
        chat_history: vec![MetaverseRoomChatMessageV1 {
            room_id: "room-max".into(),
            message_id: "chat-1".into(),
            author_peer_id: "peer-1".into(),
            display_name: Some("ゲスト".into()),
            body: "こんにちは 🎮".into(),
            created_at: 1_700_000_000_100,
        }],
    }
}

/// max fixture: phase_label=Some・scores 2 要素(負値含む)・
/// room_kind=MetaverseRoom・metaverse=Some。
fn game_room_max() -> GameRoomProjectionRow {
    GameRoomProjectionRow {
        room_id: "room-max".into(),
        topic_id: GAME_TOPIC.into(),
        channel_id: "ch:game".into(),
        host_pubkey: "b".repeat(64),
        title: "ゲーム部屋 🎮".into(),
        description: "対戦募集".into(),
        status: GameRoomStatus::Running,
        phase_label: Some("第2ラウンド".into()),
        scores: vec![
            GameScoreEntry {
                participant_id: "p1".into(),
                label: "プレイヤー1".into(),
                score: 1_200,
            },
            GameScoreEntry {
                participant_id: "p2".into(),
                label: "プレイヤー2".into(),
                score: -50,
            },
        ],
        room_kind: GameRoomKind::MetaverseRoom,
        metaverse: Some(metaverse_state()),
        updated_at: 6_000,
        source_replica_id: ReplicaId::new("topic::kukuri:topic:game-rt"),
        source_key: "game/room-max/manifest".into(),
        manifest_blob_hash: BlobHash::new("f".repeat(64)),
        derived_at: 6_100,
        projection_version: 6,
    }
}

/// min fixture: phase_label=None・scores 空 Vec・room_kind=ScoreGame・
/// metaverse=None。
fn game_room_min() -> GameRoomProjectionRow {
    GameRoomProjectionRow {
        room_id: "room-min".into(),
        topic_id: GAME_TOPIC.into(),
        channel_id: "public".into(),
        host_pubkey: "c".repeat(64),
        title: String::new(),
        description: String::new(),
        status: GameRoomStatus::Waiting,
        phase_label: None,
        scores: Vec::new(),
        room_kind: GameRoomKind::ScoreGame,
        metaverse: None,
        updated_at: 5_000,
        source_replica_id: ReplicaId::new("topic::kukuri:topic:game-rt"),
        source_key: "game/room-min/manifest".into(),
        manifest_blob_hash: BlobHash::new("0".repeat(64)),
        derived_at: 5_100,
        projection_version: 1,
    }
}

#[tokio::test]
async fn game_room_roundtrip_preserves_all_columns() {
    let store = SqliteStore::connect_memory().await.expect("sqlite store");
    let max = game_room_max();
    let min = game_room_min();
    ProjectionStore::upsert_game_room_cache(&store, min.clone())
        .await
        .expect("upsert min room");
    ProjectionStore::upsert_game_room_cache(&store, max.clone())
        .await
        .expect("upsert max room");

    // 現挙動(観測値): phase_label は filter が無いため、None で put しても
    // Some("") で読み出される(metaverse_json は filter で None のまま)。
    let mut expected_min = min.clone();
    expected_min.phase_label = Some(String::new());

    // 読み出しは list のみ。主キー updated_at DESC の順序ごと全列固定
    // (副キー room_id DESC の tie-break はここでは未行使 —
    // T6 の pagination テストと T8 の backend_parity が担保)。
    assert_eq!(
        ProjectionStore::list_topic_game_rooms(&store, GAME_TOPIC)
            .await
            .expect("list game rooms"),
        vec![max, expected_min]
    );
}
