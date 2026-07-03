//! WP-S6 T8: 順序付き list 系 + bool 系の sqlite/memory 突き合わせ。
//! notifications / bookmarked_posts / bookmarked_custom_reactions(search_key '' の
//! フォールバック含む)/ dm_outbox / dm_conversations / muted_authors / follow_edges /
//! reaction 3 種(for_target / for_targets / recent_by_author)。

use super::*;

use std::collections::HashMap;

use kukuri_core::FollowEdge;

#[derive(Debug, PartialEq)]
struct NotificationScenarioResult {
    inserted: Vec<bool>,
    unread_initial: usize,
    unread_after_single: usize,
    unread_after_all: usize,
    list_final: Vec<NotificationRow>,
}

async fn notification_scenario<S: Store + ProjectionStore>(
    store: &S,
) -> NotificationScenarioResult {
    let n1 = parity_notification("notif-1", 100, NotificationKind::Mention);
    let mut n2 = parity_notification("notif-2", 100, NotificationKind::Reply);
    n2.source_envelope_id = Some(EnvelopeId::from("env-src"));
    n2.source_replica_id = Some(ReplicaId::new("topic::parity-notifications"));
    n2.topic_id = Some("kukuri:topic:parity-notifications".into());
    n2.channel_id = Some("public".into());
    n2.object_id = Some(EnvelopeId::from("obj-1"));
    n2.preview_text = Some("reply preview".into());
    let mut n3 = parity_notification("notif-3", 90, NotificationKind::DirectMessage);
    n3.dm_id = Some("dm-parity".into());
    n3.message_id = Some("msg-9".into());
    n3.read_at = Some(95);
    // 同一 notification_id の再 put は無視され、初回の内容が残る
    let mut duplicate = n1.clone();
    duplicate.preview_text = Some("should be ignored".into());

    let mut inserted = Vec::new();
    for row in [n1, n2, n3, duplicate] {
        inserted.push(
            ProjectionStore::put_notification_if_absent(store, row)
                .await
                .expect("ProjectionStore::put_notification_if_absent"),
        );
    }
    let unread_initial = ProjectionStore::count_unread_notifications(store)
        .await
        .expect("count unread initial");
    // NOTE: read_at が NULL のままの行を含む list はここでは比較しない。
    // sqlite の NULL decode quirk により read_at NULL → Some(0) となり
    // memory(None)と一致しないため(mod.rs ヘッダ参照)、
    // list の比較は全件 read 済みになった最後に行う。

    ProjectionStore::mark_notification_read(store, "notif-1", 110)
        .await
        .expect("mark notif-1 read");
    ProjectionStore::mark_notification_read(store, "notif-1", 120)
        .await
        .expect("mark notif-1 read again (kept)");
    ProjectionStore::mark_notification_read(store, "notif-missing", 130)
        .await
        .expect("mark missing notification");
    let unread_after_single = ProjectionStore::count_unread_notifications(store)
        .await
        .expect("count unread after single");
    ProjectionStore::mark_all_notifications_read(store, 140)
        .await
        .expect("mark all read");
    let unread_after_all = ProjectionStore::count_unread_notifications(store)
        .await
        .expect("count unread after all");
    let list_final = ProjectionStore::list_notifications(store)
        .await
        .expect("list notifications final");

    NotificationScenarioResult {
        inserted,
        unread_initial,
        unread_after_single,
        unread_after_all,
        list_final,
    }
}

#[tokio::test]
async fn notifications_match_between_backends() {
    let sqlite = SqliteStore::connect_memory().await.expect("sqlite store");
    let memory = MemoryStore::default();
    let from_sqlite = notification_scenario(&sqlite).await;
    let from_memory = notification_scenario(&memory).await;
    assert_eq!(from_sqlite, from_memory);

    // sanity: sqlite 実測(received_at DESC, notification_id DESC。100 の tie は id 降順)
    assert_eq!(from_sqlite.inserted, vec![true, true, true, false]);
    assert_eq!(from_sqlite.unread_initial, 2);
    assert_eq!(
        from_sqlite
            .list_final
            .iter()
            .map(|row| row.notification_id.clone())
            .collect::<Vec<_>>(),
        vec![
            "notif-2".to_string(),
            "notif-1".to_string(),
            "notif-3".to_string(),
        ],
    );
    assert_eq!(from_sqlite.unread_after_single, 1);
    assert_eq!(from_sqlite.unread_after_all, 0);
    assert_eq!(
        from_sqlite
            .list_final
            .iter()
            .map(|row| row.read_at)
            .collect::<Vec<_>>(),
        vec![Some(140), Some(110), Some(95)],
    );
    // 重複 put は初回の内容が残る(notif-1 の preview_text は None のまま)
    assert_eq!(from_sqlite.list_final[1].preview_text, None);
}

#[derive(Debug, PartialEq)]
struct BookmarkScenarioResult {
    posts_initial: Vec<BookmarkedPostRow>,
    posts_after_remove: Vec<BookmarkedPostRow>,
    reactions_initial: Vec<BookmarkedCustomReactionRow>,
    reactions_after_remove: Vec<BookmarkedCustomReactionRow>,
}

async fn bookmark_scenario<S: Store + ProjectionStore>(store: &S) -> BookmarkScenarioResult {
    for row in [
        parity_bookmarked_post_max("bp-max", 100),
        parity_bookmarked_post("bp-min", 100),
        parity_bookmarked_post("bp-old", 90),
    ] {
        ProjectionStore::put_bookmarked_post(store, row)
            .await
            .expect("ProjectionStore::put_bookmarked_post");
    }
    // 同一 source_object_id の再 put(upsert 更新経路)
    let mut updated = parity_bookmarked_post_max("bp-max", 100);
    updated.content = Some("content:bp-max:updated".into());
    ProjectionStore::put_bookmarked_post(store, updated)
        .await
        .expect("put bookmarked post update");

    for row in [
        parity_custom_reaction("asset-normal", "smile", 100),
        parity_custom_reaction("asset-empty", "", 100),
        parity_custom_reaction("asset-blank", "   ", 90),
    ] {
        ProjectionStore::put_bookmarked_custom_reaction(store, row)
            .await
            .expect("ProjectionStore::put_bookmarked_custom_reaction");
    }
    // 同一 asset_id の再 put(upsert 更新経路)
    ProjectionStore::put_bookmarked_custom_reaction(
        store,
        parity_custom_reaction("asset-normal", "grin", 100),
    )
    .await
    .expect("put custom reaction update");

    let posts_initial = ProjectionStore::list_bookmarked_posts(store)
        .await
        .expect("list bookmarked posts initial");
    let reactions_initial = ProjectionStore::list_bookmarked_custom_reactions(store)
        .await
        .expect("list custom reactions initial");

    ProjectionStore::remove_bookmarked_post(store, &EnvelopeId::from("bp-old"))
        .await
        .expect("remove bp-old");
    ProjectionStore::remove_bookmarked_post(store, &EnvelopeId::from("bp-missing"))
        .await
        .expect("remove missing bookmarked post");
    ProjectionStore::remove_bookmarked_custom_reaction(store, "asset-empty")
        .await
        .expect("remove asset-empty");

    BookmarkScenarioResult {
        posts_initial,
        posts_after_remove: ProjectionStore::list_bookmarked_posts(store)
            .await
            .expect("list bookmarked posts after remove"),
        reactions_initial,
        reactions_after_remove: ProjectionStore::list_bookmarked_custom_reactions(store)
            .await
            .expect("list custom reactions after remove"),
    }
}

#[tokio::test]
async fn bookmarks_match_between_backends() {
    let sqlite = SqliteStore::connect_memory().await.expect("sqlite store");
    let memory = MemoryStore::default();
    let from_sqlite = bookmark_scenario(&sqlite).await;
    let from_memory = bookmark_scenario(&memory).await;
    assert_eq!(from_sqlite, from_memory);

    // sanity: sqlite 実測(bookmarked_at DESC, source_object_id DESC)
    assert_eq!(
        from_sqlite
            .posts_initial
            .iter()
            .map(|row| row.source_object_id.as_str().to_string())
            .collect::<Vec<_>>(),
        vec![
            "bp-min".to_string(),
            "bp-max".to_string(),
            "bp-old".to_string(),
        ],
    );
    assert_eq!(
        from_sqlite.posts_initial[1].content.as_deref(),
        Some("content:bp-max:updated"),
    );
    assert_eq!(from_sqlite.posts_after_remove.len(), 2);
    // search_key '' / 空白のみ → asset_id フォールバック(sqlite 読み出しと同義)
    assert_eq!(
        from_sqlite
            .reactions_initial
            .iter()
            .map(|row| (row.asset_id.clone(), row.search_key.clone()))
            .collect::<Vec<_>>(),
        vec![
            ("asset-normal".to_string(), "grin".to_string()),
            ("asset-empty".to_string(), "asset-empty".to_string()),
            ("asset-blank".to_string(), "asset-blank".to_string()),
        ],
    );
    assert_eq!(
        from_sqlite
            .reactions_after_remove
            .iter()
            .map(|row| row.asset_id.clone())
            .collect::<Vec<_>>(),
        vec!["asset-normal".to_string(), "asset-blank".to_string()],
    );
}

#[derive(Debug, PartialEq)]
struct MutedScenarioResult {
    list_initial: Vec<MutedAuthorRow>,
    muted_hit: Option<MutedAuthorRow>,
    muted_miss: Option<MutedAuthorRow>,
    list_after_remove: Vec<MutedAuthorRow>,
}

async fn muted_scenario<S: Store + ProjectionStore>(store: &S) -> MutedScenarioResult {
    for (author, muted_at) in [("m-a", 100), ("m-b", 100), ("m-c", 90)] {
        ProjectionStore::put_muted_author(
            store,
            MutedAuthorRow {
                author_pubkey: author.into(),
                muted_at,
            },
        )
        .await
        .expect("ProjectionStore::put_muted_author");
    }
    // 再 put で muted_at 更新(upsert 更新経路)
    ProjectionStore::put_muted_author(
        store,
        MutedAuthorRow {
            author_pubkey: "m-c".into(),
            muted_at: 120,
        },
    )
    .await
    .expect("put muted author update");

    let list_initial = ProjectionStore::list_muted_authors(store)
        .await
        .expect("list muted authors initial");
    let muted_hit = ProjectionStore::get_muted_author(store, "m-a")
        .await
        .expect("get muted author hit");
    let muted_miss = ProjectionStore::get_muted_author(store, "m-missing")
        .await
        .expect("get muted author miss");
    ProjectionStore::remove_muted_author(store, "m-b")
        .await
        .expect("remove muted author");

    MutedScenarioResult {
        list_initial,
        muted_hit,
        muted_miss,
        list_after_remove: ProjectionStore::list_muted_authors(store)
            .await
            .expect("list muted authors after remove"),
    }
}

#[tokio::test]
async fn muted_authors_match_between_backends() {
    let sqlite = SqliteStore::connect_memory().await.expect("sqlite store");
    let memory = MemoryStore::default();
    let from_sqlite = muted_scenario(&sqlite).await;
    let from_memory = muted_scenario(&memory).await;
    assert_eq!(from_sqlite, from_memory);

    // sanity: sqlite 実測(muted_at DESC, author_pubkey ASC。100 の tie は author 昇順)
    assert_eq!(
        from_sqlite
            .list_initial
            .iter()
            .map(|row| (row.author_pubkey.clone(), row.muted_at))
            .collect::<Vec<_>>(),
        vec![
            ("m-c".to_string(), 120),
            ("m-a".to_string(), 100),
            ("m-b".to_string(), 100),
        ],
    );
    assert_eq!(
        from_sqlite.muted_hit.as_ref().map(|row| row.muted_at),
        Some(100)
    );
    assert!(from_sqlite.muted_miss.is_none());
    assert_eq!(
        from_sqlite
            .list_after_remove
            .iter()
            .map(|row| row.author_pubkey.clone())
            .collect::<Vec<_>>(),
        vec!["m-c".to_string(), "m-a".to_string()],
    );
}

#[derive(Debug, PartialEq)]
struct FollowEdgeScenarioResult {
    by_subject: Vec<FollowEdge>,
    by_target: Vec<FollowEdge>,
}

async fn follow_edge_scenario<S: Store + ProjectionStore>(store: &S) -> FollowEdgeScenarioResult {
    let subject_1 = "1".repeat(64);
    let subject_2 = "2".repeat(64);
    let target_a = "a".repeat(64);
    let target_b = "b".repeat(64);
    let target_c = "c".repeat(64);

    for edge in [
        parity_follow_edge(
            &subject_1,
            &target_a,
            FollowEdgeStatus::Active,
            100,
            "edge-1",
        ),
        parity_follow_edge(
            &subject_1,
            &target_b,
            FollowEdgeStatus::Revoked,
            100,
            "edge-2",
        ),
        parity_follow_edge(
            &subject_1,
            &target_c,
            FollowEdgeStatus::Active,
            90,
            "edge-3",
        ),
        // 古い更新は無視される(updated_at 50 < 100)
        parity_follow_edge(
            &subject_1,
            &target_a,
            FollowEdgeStatus::Revoked,
            50,
            "edge-4",
        ),
        // 新しい更新は勝つ(95 > 90)
        parity_follow_edge(
            &subject_1,
            &target_c,
            FollowEdgeStatus::Revoked,
            95,
            "edge-5",
        ),
        // 同時刻(100 == 100)は上書きされる
        parity_follow_edge(
            &subject_1,
            &target_b,
            FollowEdgeStatus::Active,
            100,
            "edge-6",
        ),
        parity_follow_edge(
            &subject_2,
            &target_a,
            FollowEdgeStatus::Active,
            120,
            "edge-7",
        ),
    ] {
        Store::upsert_follow_edge(store, edge)
            .await
            .expect("Store::upsert_follow_edge");
    }

    FollowEdgeScenarioResult {
        by_subject: Store::list_follow_edges_by_subject(store, subject_1.as_str())
            .await
            .expect("Store::list_follow_edges_by_subject"),
        by_target: Store::list_follow_edges_by_target(store, target_a.as_str())
            .await
            .expect("Store::list_follow_edges_by_target"),
    }
}

#[tokio::test]
async fn follow_edges_match_between_backends() {
    let sqlite = SqliteStore::connect_memory().await.expect("sqlite store");
    let memory = MemoryStore::default();
    let from_sqlite = follow_edge_scenario(&sqlite).await;
    let from_memory = follow_edge_scenario(&memory).await;
    assert_eq!(from_sqlite, from_memory);

    // sanity: sqlite 実測(updated_at DESC, target ASC。stale 無視・同時刻上書き)
    assert_eq!(
        from_sqlite
            .by_subject
            .iter()
            .map(|edge| {
                (
                    edge.target_pubkey.as_str().to_string(),
                    edge.status.clone(),
                    edge.updated_at,
                    edge.envelope_id.as_str().to_string(),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (
                "a".repeat(64),
                FollowEdgeStatus::Active,
                100,
                "edge-1".to_string()
            ),
            (
                "b".repeat(64),
                FollowEdgeStatus::Active,
                100,
                "edge-6".to_string()
            ),
            (
                "c".repeat(64),
                FollowEdgeStatus::Revoked,
                95,
                "edge-5".to_string()
            ),
        ],
    );
    assert_eq!(
        from_sqlite
            .by_target
            .iter()
            .map(|edge| (edge.subject_pubkey.as_str().to_string(), edge.updated_at))
            .collect::<Vec<_>>(),
        vec![("2".repeat(64), 120), ("1".repeat(64), 100)],
    );
}

#[derive(Debug, PartialEq)]
struct ReactionScenarioResult {
    reaction_hit: Option<ReactionProjectionRow>,
    reaction_miss: Option<ReactionProjectionRow>,
    for_target: Vec<ReactionProjectionRow>,
    for_targets: HashMap<String, Vec<ReactionProjectionRow>>,
    for_targets_empty: HashMap<String, Vec<ReactionProjectionRow>>,
    recent_by_author: Vec<ReactionProjectionRow>,
}

async fn reaction_scenario<S: Store + ProjectionStore>(store: &S) -> ReactionScenarioResult {
    let replica = "topic::parity-reactions";
    let other_replica = "topic::parity-other";
    let alice = "a".repeat(64);
    let bob = "b".repeat(64);
    let carol = "c".repeat(64);
    let dave = "d".repeat(64);

    for row in [
        parity_reaction(replica, "obj-1", "react-1", &alice, "emoji:🔥", "🔥", 10),
        parity_reaction(replica, "obj-1", "react-2", &bob, "emoji:😀", "😀", 30),
        parity_reaction_custom(replica, "obj-1", "react-3", &alice, 20),
        // react-1 と同一 normalized_reaction_key(tie は reaction_id ASC)
        parity_reaction(replica, "obj-1", "react-0", &carol, "emoji:🔥", "🔥", 25),
        parity_reaction(replica, "obj-2", "react-9", &alice, "emoji:🎉", "🎉", 40),
        // 別 replica は R1 のクエリから除外される
        parity_reaction(
            other_replica,
            "obj-1",
            "react-8",
            &dave,
            "emoji:🔥",
            "🔥",
            50,
        ),
    ] {
        ProjectionStore::upsert_reaction_cache(store, row)
            .await
            .expect("ProjectionStore::upsert_reaction_cache");
    }
    // 同一キーの再 upsert(status / updated_at 更新経路)
    let mut updated = parity_reaction(replica, "obj-1", "react-1", &alice, "emoji:🔥", "🔥", 11);
    updated.status = ObjectStatus::Deleted;
    ProjectionStore::upsert_reaction_cache(store, updated)
        .await
        .expect("upsert reaction update");

    let replica_id = ReplicaId::new(replica);
    ReactionScenarioResult {
        reaction_hit: ProjectionStore::get_reaction_cache(
            store,
            &replica_id,
            &EnvelopeId::from("obj-1"),
            &EnvelopeId::from("react-1"),
        )
        .await
        .expect("get reaction hit"),
        reaction_miss: ProjectionStore::get_reaction_cache(
            store,
            &replica_id,
            &EnvelopeId::from("obj-1"),
            &EnvelopeId::from("react-missing"),
        )
        .await
        .expect("get reaction miss"),
        for_target: ProjectionStore::list_reaction_cache_for_target(
            store,
            &replica_id,
            &EnvelopeId::from("obj-1"),
        )
        .await
        .expect("list reactions for target"),
        for_targets: ProjectionStore::list_reaction_cache_for_targets(
            store,
            &replica_id,
            &[
                EnvelopeId::from("obj-1"),
                EnvelopeId::from("obj-2"),
                EnvelopeId::from("obj-3"),
            ],
        )
        .await
        .expect("list reactions for targets"),
        for_targets_empty: ProjectionStore::list_reaction_cache_for_targets(
            store,
            &replica_id,
            &[],
        )
        .await
        .expect("list reactions for empty targets"),
        recent_by_author: ProjectionStore::list_recent_reaction_cache_by_author(
            store,
            alice.as_str(),
        )
        .await
        .expect("list recent reactions by author"),
    }
}

#[tokio::test]
async fn reactions_match_between_backends() {
    let sqlite = SqliteStore::connect_memory().await.expect("sqlite store");
    let memory = MemoryStore::default();
    let from_sqlite = reaction_scenario(&sqlite).await;
    let from_memory = reaction_scenario(&memory).await;
    assert_eq!(from_sqlite, from_memory);

    // sanity: sqlite 実測
    assert_eq!(
        from_sqlite
            .reaction_hit
            .as_ref()
            .map(|row| row.status.clone()),
        Some(ObjectStatus::Deleted),
    );
    assert!(from_sqlite.reaction_miss.is_none());
    // normalized_reaction_key ASC, reaction_id ASC(custom:… < emoji:…、🔥 < 😀 は UTF-8 バイト順)
    assert_eq!(
        from_sqlite
            .for_target
            .iter()
            .map(|row| row.reaction_id.as_str().to_string())
            .collect::<Vec<_>>(),
        vec![
            "react-3".to_string(),
            "react-0".to_string(),
            "react-1".to_string(),
            "react-2".to_string(),
        ],
    );
    // 行のない obj-3 はキー自体が存在しない
    assert_eq!(from_sqlite.for_targets.len(), 2);
    assert_eq!(
        from_sqlite.for_targets.get("obj-2").map(|rows| rows.len()),
        Some(1),
    );
    assert!(from_sqlite.for_targets_empty.is_empty());
    // updated_at DESC, reaction_id DESC(react-1 は再 upsert で 11)
    assert_eq!(
        from_sqlite
            .recent_by_author
            .iter()
            .map(|row| row.reaction_id.as_str().to_string())
            .collect::<Vec<_>>(),
        vec![
            "react-9".to_string(),
            "react-3".to_string(),
            "react-1".to_string(),
        ],
    );
}

#[derive(Debug, PartialEq)]
struct OutboxScenarioResult {
    list_initial: Vec<DirectMessageOutboxRow>,
    list_after_update: Vec<DirectMessageOutboxRow>,
    outbox_hit: Option<DirectMessageOutboxRow>,
    outbox_miss: Option<DirectMessageOutboxRow>,
    list_after_remove: Vec<DirectMessageOutboxRow>,
}

async fn outbox_scenario<S: Store + ProjectionStore>(store: &S) -> OutboxScenarioResult {
    for row in [
        parity_outbox("dm-out", "om-1", 10),
        parity_outbox("dm-out", "om-2", 10),
        parity_outbox("dm-out2", "om-0", 5),
    ] {
        ProjectionStore::put_direct_message_outbox(store, row)
            .await
            .expect("ProjectionStore::put_direct_message_outbox");
    }
    let list_initial = ProjectionStore::list_direct_message_outbox(store)
        .await
        .expect("list outbox initial");

    ProjectionStore::touch_direct_message_outbox_attempt(store, "dm-out", "om-2", 99)
        .await
        .expect("touch outbox attempt");
    ProjectionStore::touch_direct_message_outbox_attempt(store, "dm-out", "om-missing", 99)
        .await
        .expect("touch missing outbox attempt");
    // 再 put で created_at 更新(upsert 更新経路。順序も変わる)
    ProjectionStore::put_direct_message_outbox(store, parity_outbox("dm-out", "om-1", 8))
        .await
        .expect("put outbox update");
    let list_after_update = ProjectionStore::list_direct_message_outbox(store)
        .await
        .expect("list outbox after update");

    let outbox_hit = ProjectionStore::get_direct_message_outbox(store, "dm-out", "om-2")
        .await
        .expect("get outbox hit");
    let outbox_miss = ProjectionStore::get_direct_message_outbox(store, "dm-out", "om-missing")
        .await
        .expect("get outbox miss");
    ProjectionStore::remove_direct_message_outbox(store, "dm-out2", "om-0")
        .await
        .expect("remove outbox");

    OutboxScenarioResult {
        list_initial,
        list_after_update,
        outbox_hit,
        outbox_miss,
        list_after_remove: ProjectionStore::list_direct_message_outbox(store)
            .await
            .expect("list outbox after remove"),
    }
}

#[tokio::test]
async fn direct_message_outbox_matches_between_backends() {
    let sqlite = SqliteStore::connect_memory().await.expect("sqlite store");
    let memory = MemoryStore::default();
    let from_sqlite = outbox_scenario(&sqlite).await;
    let from_memory = outbox_scenario(&memory).await;
    assert_eq!(from_sqlite, from_memory);

    // sanity: sqlite 実測(created_at ASC, message_id ASC。10 の tie は id 昇順)
    assert_eq!(
        from_sqlite
            .list_initial
            .iter()
            .map(|row| row.message_id.clone())
            .collect::<Vec<_>>(),
        vec!["om-0".to_string(), "om-1".to_string(), "om-2".to_string()],
    );
    assert_eq!(
        from_sqlite
            .list_after_update
            .iter()
            .map(|row| (row.message_id.clone(), row.created_at, row.last_attempt_at))
            .collect::<Vec<_>>(),
        vec![
            ("om-0".to_string(), 5, Some(0)),
            ("om-1".to_string(), 8, Some(0)),
            ("om-2".to_string(), 10, Some(99)),
        ],
    );
    assert_eq!(
        from_sqlite
            .outbox_hit
            .as_ref()
            .and_then(|row| row.last_attempt_at),
        Some(99),
    );
    assert!(from_sqlite.outbox_miss.is_none());
    assert_eq!(from_sqlite.list_after_remove.len(), 2);
}

#[derive(Debug, PartialEq)]
struct ConversationScenarioResult {
    list_initial: Vec<DirectMessageConversationRow>,
    peer_hit: Option<DirectMessageConversationRow>,
    peer_miss: Option<DirectMessageConversationRow>,
    dm_id_hit: Option<DirectMessageConversationRow>,
    dm_id_miss: Option<DirectMessageConversationRow>,
    list_after_update: Vec<DirectMessageConversationRow>,
    list_after_clear: Vec<DirectMessageConversationRow>,
}

async fn conversation_scenario<S: Store + ProjectionStore>(
    store: &S,
) -> ConversationScenarioResult {
    let peer_1 = "3".repeat(64);
    let peer_2 = "4".repeat(64);
    let peer_3 = "5".repeat(64);
    for row in [
        parity_conversation("dm-a", &peer_1, 100),
        parity_conversation("dm-b", &peer_2, 100),
        parity_conversation("dm-c", &peer_3, 90),
    ] {
        ProjectionStore::upsert_direct_message_conversation(store, row)
            .await
            .expect("ProjectionStore::upsert_direct_message_conversation");
    }
    let list_initial = ProjectionStore::list_direct_message_conversations(store)
        .await
        .expect("list conversations initial");
    let peer_hit = ProjectionStore::get_direct_message_conversation_by_peer(store, peer_2.as_str())
        .await
        .expect("get conversation by peer hit");
    let peer_miss =
        ProjectionStore::get_direct_message_conversation_by_peer(store, &"9".repeat(64))
            .await
            .expect("get conversation by peer miss");
    let dm_id_hit = ProjectionStore::get_direct_message_conversation_by_dm_id(store, "dm-a")
        .await
        .expect("get conversation by dm_id hit");
    let dm_id_miss = ProjectionStore::get_direct_message_conversation_by_dm_id(store, "dm-missing")
        .await
        .expect("get conversation by dm_id miss");

    // 再 upsert で updated_at 更新 → 順序が変わる
    ProjectionStore::upsert_direct_message_conversation(
        store,
        parity_conversation("dm-c", &peer_3, 120),
    )
    .await
    .expect("upsert conversation update");
    let list_after_update = ProjectionStore::list_direct_message_conversations(store)
        .await
        .expect("list conversations after update");

    ProjectionStore::clear_direct_message_local(store, "dm-a")
        .await
        .expect("clear dm-a");

    ConversationScenarioResult {
        list_initial,
        peer_hit,
        peer_miss,
        dm_id_hit,
        dm_id_miss,
        list_after_update,
        list_after_clear: ProjectionStore::list_direct_message_conversations(store)
            .await
            .expect("list conversations after clear"),
    }
}

#[tokio::test]
async fn direct_message_conversations_match_between_backends() {
    let sqlite = SqliteStore::connect_memory().await.expect("sqlite store");
    let memory = MemoryStore::default();
    let from_sqlite = conversation_scenario(&sqlite).await;
    let from_memory = conversation_scenario(&memory).await;
    assert_eq!(from_sqlite, from_memory);

    // sanity: sqlite 実測(updated_at DESC, dm_id DESC。100 の tie は dm_id 降順)
    assert_eq!(
        from_sqlite
            .list_initial
            .iter()
            .map(|row| row.dm_id.clone())
            .collect::<Vec<_>>(),
        vec!["dm-b".to_string(), "dm-a".to_string(), "dm-c".to_string()],
    );
    assert_eq!(
        from_sqlite.peer_hit.as_ref().map(|row| row.dm_id.clone()),
        Some("dm-b".to_string()),
    );
    assert!(from_sqlite.peer_miss.is_none());
    assert!(from_sqlite.dm_id_hit.is_some());
    assert!(from_sqlite.dm_id_miss.is_none());
    assert_eq!(
        from_sqlite
            .list_after_update
            .iter()
            .map(|row| row.dm_id.clone())
            .collect::<Vec<_>>(),
        vec!["dm-c".to_string(), "dm-b".to_string(), "dm-a".to_string()],
    );
    assert_eq!(
        from_sqlite
            .list_after_clear
            .iter()
            .map(|row| row.dm_id.clone())
            .collect::<Vec<_>>(),
        vec!["dm-c".to_string(), "dm-b".to_string()],
    );
}
