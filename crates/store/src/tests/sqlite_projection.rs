use super::*;

#[tokio::test]
async fn recent_reaction_cache_query_returns_latest_rows_for_author() {
    let store = SqliteStore::connect_memory().await.expect("sqlite store");
    let author_pubkey = "a".repeat(64);
    let target_object_id = EnvelopeId::from("target-object");
    let source_replica_id = ReplicaId::new("topic::recent-reactions");
    for row in [
        ReactionProjectionRow {
            source_replica_id: source_replica_id.clone(),
            target_object_id: target_object_id.clone(),
            reaction_id: EnvelopeId::from("reaction-1"),
            author_pubkey: author_pubkey.clone(),
            created_at: 10,
            updated_at: 10,
            reaction_key_kind: ReactionKeyKind::Emoji,
            normalized_reaction_key: "emoji:🔥".into(),
            emoji: Some("🔥".into()),
            custom_asset_id: None,
            custom_asset_snapshot: None,
            status: ObjectStatus::Active,
            source_key: "reactions/1".into(),
            source_envelope_id: EnvelopeId::from("reaction-1"),
            derived_at: 10,
            projection_version: 1,
        },
        ReactionProjectionRow {
            source_replica_id: source_replica_id.clone(),
            target_object_id: target_object_id.clone(),
            reaction_id: EnvelopeId::from("reaction-2"),
            author_pubkey: author_pubkey.clone(),
            created_at: 12,
            updated_at: 25,
            reaction_key_kind: ReactionKeyKind::Emoji,
            normalized_reaction_key: "emoji:😂".into(),
            emoji: Some("😂".into()),
            custom_asset_id: None,
            custom_asset_snapshot: None,
            status: ObjectStatus::Deleted,
            source_key: "reactions/2".into(),
            source_envelope_id: EnvelopeId::from("reaction-2"),
            derived_at: 12,
            projection_version: 1,
        },
        ReactionProjectionRow {
            source_replica_id,
            target_object_id: target_object_id.clone(),
            reaction_id: EnvelopeId::from("reaction-3"),
            author_pubkey: "b".repeat(64),
            created_at: 15,
            updated_at: 30,
            reaction_key_kind: ReactionKeyKind::Emoji,
            normalized_reaction_key: "emoji:🎉".into(),
            emoji: Some("🎉".into()),
            custom_asset_id: None,
            custom_asset_snapshot: None,
            status: ObjectStatus::Active,
            source_key: "reactions/3".into(),
            source_envelope_id: EnvelopeId::from("reaction-3"),
            derived_at: 15,
            projection_version: 1,
        },
    ] {
        ProjectionStore::upsert_reaction_cache(&store, row)
            .await
            .expect("upsert reaction cache");
    }

    let recent =
        ProjectionStore::list_recent_reaction_cache_by_author(&store, author_pubkey.as_str())
            .await
            .expect("list recent reaction cache");

    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].normalized_reaction_key, "emoji:😂");
    assert_eq!(recent[1].normalized_reaction_key, "emoji:🔥");
}
#[tokio::test]
async fn author_relationship_projection_rebuild_roundtrip() {
    let store = SqliteStore::connect_memory().await.expect("sqlite store");
    let local_author = "a".repeat(64);
    let target_author = "b".repeat(64);

    ProjectionStore::rebuild_author_relationships(
        &store,
        local_author.as_str(),
        vec![AuthorRelationshipProjectionRow {
            local_author_pubkey: local_author.clone(),
            author_pubkey: target_author.clone(),
            following: false,
            followed_by: true,
            mutual: false,
            friend_of_friend: true,
            friend_of_friend_via_pubkeys: vec!["c".repeat(64)],
            derived_at: 12,
        }],
    )
    .await
    .expect("rebuild relationships");

    let relationship = ProjectionStore::get_author_relationship(
        &store,
        local_author.as_str(),
        target_author.as_str(),
    )
    .await
    .expect("get relationship")
    .expect("relationship");
    assert!(relationship.friend_of_friend);
    assert_eq!(
        relationship.friend_of_friend_via_pubkeys,
        vec!["c".repeat(64)]
    );
    assert!(relationship.followed_by);
}
#[tokio::test]
async fn muted_authors_restore_after_restart() {
    let tempdir = tempdir().expect("tempdir");
    let db_path = tempdir.path().join("store.db");
    let author_pubkey = "b".repeat(64);

    {
        let store = SqliteStore::connect_file(&db_path)
            .await
            .expect("open sqlite store");
        ProjectionStore::put_muted_author(
            &store,
            MutedAuthorRow {
                author_pubkey: author_pubkey.clone(),
                muted_at: 42,
            },
        )
        .await
        .expect("store muted author");
        store.close().await;
    }

    let reopened = SqliteStore::connect_file(&db_path)
        .await
        .expect("reopen sqlite store");
    let muted = ProjectionStore::get_muted_author(&reopened, author_pubkey.as_str())
        .await
        .expect("get muted author")
        .expect("muted author exists");
    assert_eq!(muted.author_pubkey, author_pubkey);
    assert_eq!(muted.muted_at, 42);
    assert_eq!(
        ProjectionStore::list_muted_authors(&reopened)
            .await
            .expect("list muted authors"),
        vec![muted.clone()]
    );

    ProjectionStore::remove_muted_author(&reopened, author_pubkey.as_str())
        .await
        .expect("remove muted author");
    assert!(
        ProjectionStore::get_muted_author(&reopened, author_pubkey.as_str())
            .await
            .expect("get muted author after delete")
            .is_none()
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn author_relationship_rebuild_stays_visible_to_concurrent_readers() {
    let tempdir = tempdir().expect("tempdir");
    let db_path = tempdir.path().join("relationship-cache.db");
    let writer = SqliteStore::connect_file(&db_path)
        .await
        .expect("writer store");
    let reader = SqliteStore::connect_file(&db_path)
        .await
        .expect("reader store");
    let local_author = "a".repeat(64);
    let target_author = "b".repeat(64);

    let mut rows = (0..512)
        .map(|index| AuthorRelationshipProjectionRow {
            local_author_pubkey: local_author.clone(),
            author_pubkey: format!("{index:064x}"),
            following: true,
            followed_by: true,
            mutual: true,
            friend_of_friend: false,
            friend_of_friend_via_pubkeys: Vec::new(),
            derived_at: index,
        })
        .collect::<Vec<_>>();
    rows.push(AuthorRelationshipProjectionRow {
        local_author_pubkey: local_author.clone(),
        author_pubkey: target_author.clone(),
        following: true,
        followed_by: true,
        mutual: true,
        friend_of_friend: false,
        friend_of_friend_via_pubkeys: Vec::new(),
        derived_at: 999,
    });

    ProjectionStore::rebuild_author_relationships(&writer, local_author.as_str(), rows.clone())
        .await
        .expect("seed relationships");

    let keep_running = Arc::new(AtomicBool::new(true));
    let saw_gap = Arc::new(AtomicBool::new(false));
    let keep_running_for_task = Arc::clone(&keep_running);
    let saw_gap_for_task = Arc::clone(&saw_gap);
    let local_author_for_task = local_author.clone();
    let target_author_for_task = target_author.clone();

    let reader_task = tokio::spawn(async move {
        while keep_running_for_task.load(Ordering::SeqCst) {
            let relationship = ProjectionStore::get_author_relationship(
                &reader,
                local_author_for_task.as_str(),
                target_author_for_task.as_str(),
            )
            .await
            .expect("read relationship");
            if relationship.is_none() {
                saw_gap_for_task.store(true, Ordering::SeqCst);
                break;
            }
            tokio::task::yield_now().await;
        }
    });

    for _ in 0..32 {
        ProjectionStore::rebuild_author_relationships(&writer, local_author.as_str(), rows.clone())
            .await
            .expect("rebuild relationships");
        if saw_gap.load(Ordering::SeqCst) {
            break;
        }
    }

    keep_running.store(false, Ordering::SeqCst);
    reader_task.await.expect("reader task");
    assert!(
        !saw_gap.load(Ordering::SeqCst),
        "concurrent readers should never observe a missing relationship during rebuild",
    );
}
#[tokio::test]
async fn projection_rebuild_from_docs_blobs_only() {
    let store = SqliteStore::connect_memory().await.expect("sqlite store");
    let topic = "kukuri:topic:projection";
    let root_id = EnvelopeId::from("object-root");
    let reply_id = EnvelopeId::from("object-reply");
    let rows = vec![
        ObjectProjectionRow {
            object_id: root_id.clone(),
            topic_id: topic.to_string(),
            channel_id: "public".into(),
            author_pubkey: "a".repeat(64),
            created_at: 10,
            object_kind: "post".into(),
            root_object_id: None,
            reply_to_object_id: None,
            payload_ref: PayloadRef::BlobText {
                hash: BlobHash::new("1".repeat(64)),
                mime: "text/plain".into(),
                bytes: 4,
            },
            content: Some("root".into()),
            repost_of: None,
            source_replica_id: ReplicaId::new(format!("topic::{topic}")),
            source_key: "objects/object-root/header".into(),
            source_envelope_id: root_id.clone(),
            source_blob_hash: Some(BlobHash::new("1".repeat(64))),
            derived_at: 10,
            projection_version: 1,
        },
        ObjectProjectionRow {
            object_id: reply_id.clone(),
            topic_id: topic.to_string(),
            channel_id: "public".into(),
            author_pubkey: "b".repeat(64),
            created_at: 11,
            object_kind: "comment".into(),
            root_object_id: Some(root_id.clone()),
            reply_to_object_id: Some(root_id.clone()),
            payload_ref: PayloadRef::BlobText {
                hash: BlobHash::new("2".repeat(64)),
                mime: "text/plain".into(),
                bytes: 5,
            },
            content: Some("reply".into()),
            repost_of: None,
            source_replica_id: ReplicaId::new(format!("topic::{topic}")),
            source_key: "objects/object-reply/header".into(),
            source_envelope_id: reply_id.clone(),
            source_blob_hash: Some(BlobHash::new("2".repeat(64))),
            derived_at: 11,
            projection_version: 1,
        },
    ];

    ProjectionStore::rebuild_object_projections(&store, rows)
        .await
        .expect("rebuild projection");

    let timeline = ProjectionStore::list_topic_timeline(&store, topic, None, 10)
        .await
        .expect("timeline");
    let thread = ProjectionStore::list_thread(&store, topic, &root_id, None, 10)
        .await
        .expect("thread");

    assert_eq!(timeline.items.len(), 2);
    assert_eq!(timeline.items[0].object_id, reply_id);
    assert_eq!(thread.items.len(), 2);
    assert_eq!(thread.items[0].object_id, root_id);
}
