use super::*;

#[tokio::test]
async fn direct_message_delete_clears_outbox_but_keeps_tombstone() {
    let store = SqliteStore::connect_memory().await.expect("sqlite store");
    let dm_id = "dm-test";
    ProjectionStore::upsert_direct_message_conversation(
        &store,
        DirectMessageConversationRow {
            dm_id: dm_id.into(),
            peer_pubkey: "b".repeat(64),
            updated_at: 10,
            last_message_at: Some(10),
            last_message_id: Some("message-1".into()),
            last_message_preview: Some("queued".into()),
        },
    )
    .await
    .expect("upsert conversation");
    ProjectionStore::put_direct_message_message(
        &store,
        DirectMessageMessageRow {
            dm_id: dm_id.into(),
            message_id: "message-1".into(),
            sender_pubkey: "a".repeat(64),
            recipient_pubkey: "b".repeat(64),
            created_at: 10,
            text: Some("queued".into()),
            reply_to_message_id: None,
            attachment_manifest: None,
            outgoing: true,
            acked_at: None,
        },
    )
    .await
    .expect("put message");
    ProjectionStore::put_direct_message_outbox(
        &store,
        DirectMessageOutboxRow {
            dm_id: dm_id.into(),
            message_id: "message-1".into(),
            peer_pubkey: "b".repeat(64),
            frame_blob_hash: BlobHash::new("frame-hash"),
            created_at: 10,
            last_attempt_at: None,
        },
    )
    .await
    .expect("put outbox");
    ProjectionStore::put_direct_message_tombstone(
        &store,
        DirectMessageTombstoneRow {
            dm_id: dm_id.into(),
            message_id: "message-1".into(),
            deleted_at: 20,
        },
    )
    .await
    .expect("put tombstone");
    ProjectionStore::delete_direct_message_message_local(&store, dm_id, "message-1")
        .await
        .expect("delete message");
    ProjectionStore::clear_direct_message_local(&store, dm_id)
        .await
        .expect("clear conversation");

    assert!(
        ProjectionStore::get_direct_message_outbox(&store, dm_id, "message-1")
            .await
            .expect("get outbox")
            .is_none()
    );
    assert!(
        ProjectionStore::get_direct_message_conversation_by_dm_id(&store, dm_id)
            .await
            .expect("get conversation")
            .is_none()
    );
    assert!(
        ProjectionStore::has_direct_message_tombstone(&store, dm_id, "message-1")
            .await
            .expect("has tombstone")
    );
}
#[tokio::test]
async fn direct_message_local_delete_prevents_duplicate_reinsert() {
    let store = SqliteStore::connect_memory().await.expect("sqlite store");
    let dm_id = "dm-test";
    let message = DirectMessageMessageRow {
        dm_id: dm_id.into(),
        message_id: "message-1".into(),
        sender_pubkey: "a".repeat(64),
        recipient_pubkey: "b".repeat(64),
        created_at: 10,
        text: Some("hello".into()),
        reply_to_message_id: None,
        attachment_manifest: None,
        outgoing: false,
        acked_at: None,
    };

    ProjectionStore::put_direct_message_message(&store, message.clone())
        .await
        .expect("insert message");
    ProjectionStore::put_direct_message_tombstone(
        &store,
        DirectMessageTombstoneRow {
            dm_id: dm_id.into(),
            message_id: "message-1".into(),
            deleted_at: 20,
        },
    )
    .await
    .expect("insert tombstone");
    ProjectionStore::delete_direct_message_message_local(&store, dm_id, "message-1")
        .await
        .expect("delete local message");
    ProjectionStore::put_direct_message_message(&store, message)
        .await
        .expect("reinsert ignored");

    let page = ProjectionStore::list_direct_message_messages(&store, dm_id, None, 20)
        .await
        .expect("list messages");
    assert!(page.items.is_empty());
    assert!(
        ProjectionStore::has_direct_message_tombstone(&store, dm_id, "message-1")
            .await
            .expect("has tombstone")
    );
}
