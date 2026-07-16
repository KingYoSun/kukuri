//! WP-S6 T4【contract】row_mapping round-trip 層 — DM / notifications 系
//! (後続 WP-H1 の安全網)。
//!
//! 対象: dm_conversations / dm_messages(outgoing の i64→bool 変換と
//! acked_at の無条件上書き)/ dm_outbox / dm_message_tombstones /
//! notifications(kind 6 値の永続文字列と read_at COALESCE = 既読を上書きしない)。
//! 分割元の全体説明は row_mapping_roundtrip.rs の冒頭を参照。

use super::*;
use kukuri_core::{
    DirectMessageAttachmentKind, DirectMessageAttachmentManifestV1, DirectMessageEncryptedBlobRefV1,
};

// ---------------------------------------------------------------------------
// dm_conversations(row_to_direct_message_conversation)
// ---------------------------------------------------------------------------

/// max fixture: last_* 3 列すべて Some。
fn conversation_max() -> DirectMessageConversationRow {
    DirectMessageConversationRow {
        dm_id: "dm-max".into(),
        peer_pubkey: "b".repeat(64),
        updated_at: 2_000,
        last_message_at: Some(1_999),
        last_message_id: Some("msg-9".into()),
        last_message_preview: Some("プレビュー ✉️".into()),
    }
}

/// min fixture: last_* 3 列すべて None。
fn conversation_min() -> DirectMessageConversationRow {
    DirectMessageConversationRow {
        dm_id: "dm-min".into(),
        peer_pubkey: "c".repeat(64),
        updated_at: 1_000,
        last_message_at: None,
        last_message_id: None,
        last_message_preview: None,
    }
}

#[tokio::test]
async fn direct_message_conversation_roundtrip_preserves_all_columns() {
    let store = SqliteStore::connect_memory().await.expect("sqlite store");
    let max = conversation_max();
    let min = conversation_min();
    DirectMessageStore::upsert_direct_message_conversation(&store, max.clone())
        .await
        .expect("upsert max conversation");
    DirectMessageStore::upsert_direct_message_conversation(&store, min.clone())
        .await
        .expect("upsert min conversation");

    // None で put した last_* 3 列は None のまま読み出される
    // (WP-B16 で NULL decode quirk を解消)。
    assert_eq!(
        DirectMessageStore::get_direct_message_conversation_by_dm_id(&store, "dm-max")
            .await
            .expect("get by dm_id"),
        Some(max.clone())
    );
    assert_eq!(
        DirectMessageStore::get_direct_message_conversation_by_peer(
            &store,
            min.peer_pubkey.as_str()
        )
        .await
        .expect("get by peer"),
        Some(min.clone())
    );
    // 主キー updated_at DESC の順序ごと固定(副キー dm_id DESC の
    // tie-break はここでは未行使 — T6 の pagination テストと T8 の backend_parity が担保)。
    assert_eq!(
        DirectMessageStore::list_direct_message_conversations(&store)
            .await
            .expect("list conversations"),
        vec![max, min]
    );
}

// ---------------------------------------------------------------------------
// dm_messages(row_to_direct_message_message)
// ---------------------------------------------------------------------------

/// max fixture: text / reply / attachment_manifest(poster あり)/ acked_at
/// すべて Some、outgoing=true。
fn message_max() -> DirectMessageMessageRow {
    DirectMessageMessageRow {
        dm_id: "dm-max".into(),
        message_id: "msg-max".into(),
        sender_pubkey: "a".repeat(64),
        recipient_pubkey: "b".repeat(64),
        created_at: 1_700_000_000_010,
        text: Some("こんにちは 📨".into()),
        reply_to_message_id: Some("msg-parent".into()),
        attachment_manifest: Some(DirectMessageAttachmentManifestV1 {
            attachment_id: "att-1".into(),
            kind: DirectMessageAttachmentKind::Video,
            original: DirectMessageEncryptedBlobRefV1 {
                blob_id: "blob-1".into(),
                hash: BlobHash::new("1".repeat(64)),
                mime: "video/mp4".into(),
                bytes: 9_876_543,
                nonce_hex: "0a0b0c".into(),
            },
            poster: Some(DirectMessageEncryptedBlobRefV1 {
                blob_id: "blob-2".into(),
                hash: BlobHash::new("2".repeat(64)),
                mime: "image/jpeg".into(),
                bytes: 4_321,
                nonce_hex: "0d0e0f".into(),
            }),
        }),
        outgoing: true,
        acked_at: Some(1_700_000_000_020),
    }
}

/// min fixture: 全 Option=None、outgoing=false。
fn message_min() -> DirectMessageMessageRow {
    DirectMessageMessageRow {
        dm_id: "dm-min".into(),
        message_id: "msg-min".into(),
        sender_pubkey: "c".repeat(64),
        recipient_pubkey: "d".repeat(64),
        created_at: 0,
        text: None,
        reply_to_message_id: None,
        attachment_manifest: None,
        outgoing: false,
        acked_at: None,
    }
}

#[tokio::test]
async fn direct_message_message_roundtrip_preserves_all_columns() {
    let store = SqliteStore::connect_memory().await.expect("sqlite store");
    let max = message_max();
    let min = message_min();
    DirectMessageStore::put_direct_message_message(&store, max.clone())
        .await
        .expect("put max message");
    DirectMessageStore::put_direct_message_message(&store, min.clone())
        .await
        .expect("put min message");

    assert_eq!(
        DirectMessageStore::get_direct_message_message(&store, "dm-max", "msg-max")
            .await
            .expect("get max message"),
        Some(max)
    );

    // None で put した text / reply_to_message_id / acked_at / attachment_manifest は
    // None のまま読み出される(WP-B16 で NULL decode quirk を解消)。
    assert_eq!(
        DirectMessageStore::get_direct_message_message(&store, "dm-min", "msg-min")
            .await
            .expect("get min message"),
        Some(min)
    );

    // outgoing の永続形式は i64(true→1 / false→0)。既存 DB 互換の契約として固定。
    let raw_outgoing_max = sqlx::query_scalar::<_, i64>(
        "SELECT outgoing FROM dm_messages WHERE dm_id = 'dm-max' AND message_id = 'msg-max'",
    )
    .fetch_one(store.pool())
    .await
    .expect("raw outgoing max");
    assert_eq!(raw_outgoing_max, 1);
    let raw_outgoing_min = sqlx::query_scalar::<_, i64>(
        "SELECT outgoing FROM dm_messages WHERE dm_id = 'dm-min' AND message_id = 'msg-min'",
    )
    .fetch_one(store.pool())
    .await
    .expect("raw outgoing min");
    assert_eq!(raw_outgoing_min, 0);
}

#[tokio::test]
async fn direct_message_acked_at_set_overwrites_unconditionally() {
    // set_direct_message_acked_at は COALESCE ではなく無条件 UPDATE
    // (notifications の read_at と対照的)。Some の上書きと None→Some を固定。
    let store = SqliteStore::connect_memory().await.expect("sqlite store");
    DirectMessageStore::put_direct_message_message(&store, message_max())
        .await
        .expect("put max message");
    DirectMessageStore::put_direct_message_message(&store, message_min())
        .await
        .expect("put min message");

    DirectMessageStore::set_direct_message_acked_at(&store, "dm-max", "msg-max", 1_700_000_000_099)
        .await
        .expect("ack max");
    DirectMessageStore::set_direct_message_acked_at(&store, "dm-min", "msg-min", 42)
        .await
        .expect("ack min");

    assert_eq!(
        DirectMessageStore::get_direct_message_message(&store, "dm-max", "msg-max")
            .await
            .expect("get max message")
            .expect("max message exists")
            .acked_at,
        Some(1_700_000_000_099)
    );
    assert_eq!(
        DirectMessageStore::get_direct_message_message(&store, "dm-min", "msg-min")
            .await
            .expect("get min message")
            .expect("min message exists")
            .acked_at,
        Some(42)
    );
}

// ---------------------------------------------------------------------------
// dm_outbox(row_to_direct_message_outbox)
// ---------------------------------------------------------------------------

/// max fixture: last_attempt_at=Some。
fn outbox_max() -> DirectMessageOutboxRow {
    DirectMessageOutboxRow {
        dm_id: "dm-max".into(),
        message_id: "msg-out-max".into(),
        peer_pubkey: "b".repeat(64),
        frame_blob_hash: BlobHash::new("3".repeat(64)),
        created_at: 200,
        last_attempt_at: Some(250),
    }
}

/// min fixture: last_attempt_at=None。
fn outbox_min() -> DirectMessageOutboxRow {
    DirectMessageOutboxRow {
        dm_id: "dm-min".into(),
        message_id: "msg-out-min".into(),
        peer_pubkey: "c".repeat(64),
        frame_blob_hash: BlobHash::new("4".repeat(64)),
        created_at: 100,
        last_attempt_at: None,
    }
}

#[tokio::test]
async fn direct_message_outbox_roundtrip_preserves_all_columns() {
    let store = SqliteStore::connect_memory().await.expect("sqlite store");
    let max = outbox_max();
    let min = outbox_min();
    DirectMessageStore::put_direct_message_outbox(&store, max.clone())
        .await
        .expect("put max outbox");
    DirectMessageStore::put_direct_message_outbox(&store, min.clone())
        .await
        .expect("put min outbox");

    // None で put した last_attempt_at は None のまま読み出される
    // (WP-B16 で NULL decode quirk を解消)。
    assert_eq!(
        DirectMessageStore::get_direct_message_outbox(&store, "dm-max", "msg-out-max")
            .await
            .expect("get max outbox"),
        Some(max.clone())
    );
    // 主キー created_at ASC(送信キューは古い順)の順序ごと固定(副キー message_id ASC の
    // tie-break はここでは未行使 — T6 の pagination テストと T8 の backend_parity が担保)。
    assert_eq!(
        DirectMessageStore::list_direct_message_outbox(&store)
            .await
            .expect("list outbox"),
        vec![min, max]
    );
}

// ---------------------------------------------------------------------------
// dm_message_tombstones(row_to_direct_message_tombstone)
// 3 列すべて必須のため max/min の区別なし(単一 fixture で全列固定)。
// ---------------------------------------------------------------------------

#[tokio::test]
async fn direct_message_tombstone_roundtrip_preserves_all_columns() {
    let store = SqliteStore::connect_memory().await.expect("sqlite store");
    let tombstone = DirectMessageTombstoneRow {
        dm_id: "dm-max".into(),
        message_id: "msg-del".into(),
        deleted_at: 555,
    };
    DirectMessageStore::put_direct_message_tombstone(&store, tombstone.clone())
        .await
        .expect("put tombstone");

    assert_eq!(
        DirectMessageStore::list_direct_message_tombstones(&store, "dm-max")
            .await
            .expect("list tombstones"),
        vec![tombstone]
    );
    assert!(
        DirectMessageStore::has_direct_message_tombstone(&store, "dm-max", "msg-del")
            .await
            .expect("has tombstone")
    );
    assert!(
        !DirectMessageStore::has_direct_message_tombstone(&store, "dm-max", "msg-other")
            .await
            .expect("has other tombstone")
    );
}

// ---------------------------------------------------------------------------
// notifications 全 15 列(row_to_notification)
// ---------------------------------------------------------------------------

/// max fixture: 全 Option=Some(source_* / topic / channel / object / dm /
/// message / preview / read_at)。
fn notification_max() -> NotificationRow {
    NotificationRow {
        notification_id: "notif-max".into(),
        recipient_pubkey: "a".repeat(64),
        kind: NotificationKind::QuoteRepost,
        actor_pubkey: "b".repeat(64),
        source_envelope_id: Some(EnvelopeId::from("env-notif")),
        source_replica_id: Some(ReplicaId::new("topic::kukuri:topic:notif-rt")),
        topic_id: Some("kukuri:topic:notif-rt".into()),
        channel_id: Some("ch:notif".into()),
        object_id: Some(EnvelopeId::from("obj-notif")),
        dm_id: Some("dm-notif".into()),
        message_id: Some("msg-notif".into()),
        preview_text: Some("通知プレビュー 🔔".into()),
        created_at: 7_000,
        received_at: 7_100,
        read_at: Some(7_200),
    }
}

/// min fixture: 全 Option=None。
fn notification_min() -> NotificationRow {
    NotificationRow {
        notification_id: "notif-min".into(),
        recipient_pubkey: "c".repeat(64),
        kind: NotificationKind::Mention,
        actor_pubkey: "d".repeat(64),
        source_envelope_id: None,
        source_replica_id: None,
        topic_id: None,
        channel_id: None,
        object_id: None,
        dm_id: None,
        message_id: None,
        preview_text: None,
        created_at: 0,
        received_at: 0,
        read_at: None,
    }
}

#[tokio::test]
async fn notification_roundtrip_preserves_all_15_columns() {
    let store = SqliteStore::connect_memory().await.expect("sqlite store");
    let max = notification_max();
    let min = notification_min();
    assert!(
        NotificationStore::put_notification_if_absent(&store, max.clone())
            .await
            .expect("put max notification")
    );
    assert!(
        NotificationStore::put_notification_if_absent(&store, min.clone())
            .await
            .expect("put min notification")
    );

    // 同一 notification_id の再 put は false を返し既存行を変更しない。
    let mut duplicate = max.clone();
    duplicate.preview_text = Some("上書きされないはず".into());
    assert!(
        !NotificationStore::put_notification_if_absent(&store, duplicate)
            .await
            .expect("put duplicate notification")
    );

    // None で put した read_at は None のまま読み出される(WP-B16 で
    // NULL decode quirk を解消。count_unread の SQL IS NULL 判定とも整合)。
    // received_at DESC, notification_id DESC の順序ごと全列固定。
    assert_eq!(
        NotificationStore::list_notifications(&store)
            .await
            .expect("list notifications"),
        vec![max, min]
    );
}

#[tokio::test]
async fn notification_kind_roundtrip_covers_all_six_kinds() {
    let store = SqliteStore::connect_memory().await.expect("sqlite store");
    // DB の永続文字列(生リテラル)と enum の対応を DB 往復で固定する。
    let kinds: [(NotificationKind, &str); 6] = [
        (NotificationKind::Mention, "mention"),
        (NotificationKind::Reply, "reply"),
        (NotificationKind::Repost, "repost"),
        (NotificationKind::QuoteRepost, "quote_repost"),
        (NotificationKind::DirectMessage, "direct_message"),
        (NotificationKind::Followed, "followed"),
    ];
    for (index, (kind, _)) in kinds.iter().enumerate() {
        let mut row = notification_min();
        row.notification_id = format!("notif-kind-{index}");
        row.received_at = 100 + index as i64;
        row.kind = kind.clone();
        assert!(
            NotificationStore::put_notification_if_absent(&store, row)
                .await
                .expect("put kind notification")
        );
    }

    for (index, (_, raw)) in kinds.iter().enumerate() {
        let stored = sqlx::query_scalar::<_, String>(
            "SELECT kind FROM notifications WHERE notification_id = ?1",
        )
        .bind(format!("notif-kind-{index}"))
        .fetch_one(store.pool())
        .await
        .expect("raw kind");
        assert_eq!(stored, *raw);
    }

    // 読み出しで enum に戻る(received_at DESC → 挿入の逆順)。
    let listed = NotificationStore::list_notifications(&store)
        .await
        .expect("list notifications");
    let kinds_by_id = listed
        .into_iter()
        .map(|row| (row.notification_id, row.kind))
        .collect::<Vec<_>>();
    assert_eq!(
        kinds_by_id,
        vec![
            ("notif-kind-5".to_string(), NotificationKind::Followed),
            ("notif-kind-4".to_string(), NotificationKind::DirectMessage),
            ("notif-kind-3".to_string(), NotificationKind::QuoteRepost),
            ("notif-kind-2".to_string(), NotificationKind::Repost),
            ("notif-kind-1".to_string(), NotificationKind::Reply),
            ("notif-kind-0".to_string(), NotificationKind::Mention),
        ]
    );
}

#[tokio::test]
async fn notification_read_at_coalesce_does_not_overwrite() {
    // mark_notification_read / mark_all_notifications_read は
    // COALESCE(read_at, ?) — 既読の read_at を上書きしない。
    let store = SqliteStore::connect_memory().await.expect("sqlite store");
    let mut first = notification_min();
    first.notification_id = "notif-a".into();
    first.received_at = 100;
    let mut second = notification_min();
    second.notification_id = "notif-b".into();
    second.received_at = 200;
    NotificationStore::put_notification_if_absent(&store, first)
        .await
        .expect("put notif-a");
    NotificationStore::put_notification_if_absent(&store, second)
        .await
        .expect("put notif-b");
    assert_eq!(
        NotificationStore::count_unread_notifications(&store)
            .await
            .expect("count unread"),
        2
    );

    NotificationStore::mark_notification_read(&store, "notif-a", 1_000)
        .await
        .expect("mark notif-a");
    // 2 回目の mark は無視される(既読を上書きしない)。
    NotificationStore::mark_notification_read(&store, "notif-a", 9_999)
        .await
        .expect("re-mark notif-a");
    // mark_all は read_at IS NULL の行(notif-b)だけを埋める。
    NotificationStore::mark_all_notifications_read(&store, 2_000)
        .await
        .expect("mark all");

    let listed = NotificationStore::list_notifications(&store)
        .await
        .expect("list notifications");
    assert_eq!(
        listed
            .iter()
            .map(|row| (row.notification_id.as_str(), row.read_at))
            .collect::<Vec<_>>(),
        vec![("notif-b", Some(2_000)), ("notif-a", Some(1_000))]
    );
    assert_eq!(
        NotificationStore::count_unread_notifications(&store)
            .await
            .expect("count unread after"),
        0
    );
}
