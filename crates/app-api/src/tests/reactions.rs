use super::*;

#[tokio::test]
async fn public_post_reaction_persists_and_aggregates_emoji_and_custom_keys() {
    let (app, store, docs_sync, blob_service) = local_app_with_memory_services();
    let topic = "kukuri:topic:reaction-public";
    let object_id = app
        .create_post(topic, "reactable post", None)
        .await
        .expect("create post");
    let custom_asset = app
        .create_custom_reaction_asset(CreateCustomReactionAssetInput {
            search_key: "party".into(),
            mime: "image/png".into(),
            bytes: tiny_png_bytes(),
            width: 128,
            height: 128,
        })
        .await
        .expect("create custom reaction asset");

    let emoji_state = app
        .toggle_reaction(
            topic,
            object_id.as_str(),
            ReactionKeyV1::Emoji {
                emoji: "👍".into()
            },
            None,
        )
        .await
        .expect("toggle emoji reaction");
    let custom_state = app
        .toggle_reaction(
            topic,
            object_id.as_str(),
            ReactionKeyV1::CustomAsset {
                asset_id: custom_asset.asset_id.clone(),
                snapshot: reaction_snapshot_from_view(&custom_asset),
            },
            None,
        )
        .await
        .expect("toggle custom reaction");
    let target = store
        .get_object_projection(&EnvelopeId::from(object_id.clone()))
        .await
        .expect("object projection")
        .expect("target projection");
    let reaction_rows = store
        .list_reaction_cache_for_target(
            &target.source_replica_id,
            &EnvelopeId::from(object_id.clone()),
        )
        .await
        .expect("reaction rows");
    let timeline = app.list_timeline(topic, None, 20).await.expect("timeline");
    let post = timeline
        .items
        .iter()
        .find(|item| item.object_id == object_id)
        .expect("reaction post");
    let author_replica = author_replica_id(custom_asset.owner_pubkey.as_str());
    let asset_docs = docs_sync
        .query_replica(
            &author_replica,
            DocQuery::Prefix("reactions/assets/".into()),
        )
        .await
        .expect("asset docs");
    let stored_blob = blob_service
        .fetch_blob(&kukuri_core::BlobHash::new(custom_asset.blob_hash.clone()))
        .await
        .expect("fetch stored blob")
        .expect("stored blob bytes");

    assert_eq!(emoji_state.target_object_id, object_id);
    assert_eq!(custom_state.target_object_id, object_id);
    assert_eq!(reaction_rows.len(), 2);
    assert!(
        reaction_rows
            .iter()
            .all(|row| row.status == ObjectStatus::Active)
    );
    assert_eq!(post.reaction_summary.len(), 2);
    assert_eq!(post.my_reactions.len(), 2);
    assert!(post.reaction_summary.iter().any(|entry| {
        entry.reaction_key_kind == "emoji"
            && entry.emoji.as_deref() == Some("👍")
            && entry.count == 1
    }));
    assert!(post.reaction_summary.iter().any(|entry| {
        entry.reaction_key_kind == "custom_asset"
            && entry
                .custom_asset
                .as_ref()
                .map(|asset| asset.asset_id.as_str())
                == Some(custom_asset.asset_id.as_str())
            && entry.count == 1
    }));
    assert_eq!(asset_docs.len(), 2);
    assert_eq!(stored_blob, tiny_png_bytes());
}

#[tokio::test]
async fn same_author_same_reaction_key_toggles_off() {
    let (app, store, _, _) = local_app_with_memory_services();
    let topic = "kukuri:topic:reaction-toggle";
    let object_id = app
        .create_post(topic, "toggle me", None)
        .await
        .expect("create post");

    let first = app
        .toggle_reaction(
            topic,
            object_id.as_str(),
            ReactionKeyV1::Emoji {
                emoji: "🎉".into()
            },
            None,
        )
        .await
        .expect("first toggle");
    let second = app
        .toggle_reaction(
            topic,
            object_id.as_str(),
            ReactionKeyV1::Emoji {
                emoji: "🎉".into()
            },
            None,
        )
        .await
        .expect("second toggle");
    let target = store
        .get_object_projection(&EnvelopeId::from(object_id.clone()))
        .await
        .expect("object projection")
        .expect("target projection");
    let reaction_rows = store
        .list_reaction_cache_for_target(
            &target.source_replica_id,
            &EnvelopeId::from(object_id.clone()),
        )
        .await
        .expect("reaction rows");

    assert_eq!(first.reaction_summary.len(), 1);
    assert!(second.reaction_summary.is_empty());
    assert!(second.my_reactions.is_empty());
    assert_eq!(reaction_rows.len(), 1);
    assert_eq!(reaction_rows[0].status, ObjectStatus::Deleted);
}

#[tokio::test]
async fn different_reaction_keys_can_coexist_on_same_target() {
    let (app, _, _, _) = local_app_with_memory_services();
    let topic = "kukuri:topic:reaction-coexist";
    let object_id = app
        .create_post(topic, "multiple reactions", None)
        .await
        .expect("create post");

    app.toggle_reaction(
        topic,
        object_id.as_str(),
        ReactionKeyV1::Emoji {
            emoji: "🔥".into()
        },
        None,
    )
    .await
    .expect("fire reaction");
    let state = app
        .toggle_reaction(
            topic,
            object_id.as_str(),
            ReactionKeyV1::Emoji {
                emoji: "😂".into()
            },
            None,
        )
        .await
        .expect("laugh reaction");

    assert_eq!(state.reaction_summary.len(), 2);
    assert_eq!(state.my_reactions.len(), 2);
    assert!(
        state
            .reaction_summary
            .iter()
            .any(|entry| entry.normalized_reaction_key == "emoji:🔥" && entry.count == 1)
    );
    assert!(
        state
            .reaction_summary
            .iter()
            .any(|entry| entry.normalized_reaction_key == "emoji:😂" && entry.count == 1)
    );
}

#[tokio::test]
async fn custom_reaction_asset_is_author_owned_public_blob_backed_object() {
    let (app, _, docs_sync, blob_service) = local_app_with_memory_services();
    let asset = app
        .create_custom_reaction_asset(CreateCustomReactionAssetInput {
            search_key: "party".into(),
            mime: "image/png".into(),
            bytes: tiny_png_bytes(),
            width: 128,
            height: 128,
        })
        .await
        .expect("create custom reaction asset");
    let listed = app
        .list_my_custom_reaction_assets()
        .await
        .expect("list owned assets");
    let author_replica = author_replica_id(asset.owner_pubkey.as_str());
    let asset_docs = docs_sync
        .query_replica(
            &author_replica,
            DocQuery::Prefix("reactions/assets/".into()),
        )
        .await
        .expect("asset docs");
    let stored_blob = blob_service
        .fetch_blob(&kukuri_core::BlobHash::new(asset.blob_hash.clone()))
        .await
        .expect("fetch blob")
        .expect("stored blob");

    assert_eq!(listed, vec![asset.clone()]);
    assert_eq!(asset.search_key, "party");
    assert_eq!(asset_docs.len(), 2);
    assert_eq!(stored_blob, tiny_png_bytes());
}

#[tokio::test]
async fn recent_reactions_return_latest_unique_keys_even_after_toggle_off() {
    let (app, _, _, _) = local_app_with_memory_services();
    let topic = "kukuri:topic:reaction-recent";
    let object_id = app
        .create_post(topic, "recent reactions", None)
        .await
        .expect("create post");

    app.toggle_reaction(
        topic,
        object_id.as_str(),
        ReactionKeyV1::Emoji {
            emoji: "🔥".into()
        },
        None,
    )
    .await
    .expect("fire reaction");
    sleep(Duration::from_millis(5)).await;
    app.toggle_reaction(
        topic,
        object_id.as_str(),
        ReactionKeyV1::Emoji {
            emoji: "😂".into()
        },
        None,
    )
    .await
    .expect("laugh reaction");
    sleep(Duration::from_millis(5)).await;
    app.toggle_reaction(
        topic,
        object_id.as_str(),
        ReactionKeyV1::Emoji {
            emoji: "🔥".into()
        },
        None,
    )
    .await
    .expect("toggle fire reaction off");

    let recent = app
        .list_recent_reactions(8)
        .await
        .expect("list recent reactions");

    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].normalized_reaction_key, "emoji:🔥");
    assert_eq!(recent[1].normalized_reaction_key, "emoji:😂");
}

#[tokio::test]
async fn local_bookmarks_restore_saved_custom_reactions_after_restart() {
    let dir = tempdir().expect("tempdir");
    let database_path = dir.path().join("bookmark-store.sqlite");
    let store = Arc::new(
        SqliteStore::connect_file(&database_path)
            .await
            .expect("sqlite store"),
    );
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let local_keys = generate_keys();
    let foreign_keys = generate_keys();
    let foreign_pubkey = foreign_keys.public_key_hex();
    let app = AppService::new_with_services(
        store.clone(),
        store.clone(),
        transport.clone(),
        Arc::new(NoopHintTransport),
        docs_sync.clone(),
        blob_service.clone(),
        local_keys,
    );

    app.bookmark_custom_reaction(CustomReactionAssetSnapshotV1 {
        asset_id: "asset-bookmarked".into(),
        owner_pubkey: Pubkey::from(foreign_pubkey.as_str()),
        blob_hash: kukuri_core::BlobHash::new("blob-bookmarked"),
        search_key: "bookmark".into(),
        mime: "image/png".into(),
        bytes: 128,
        width: 128,
        height: 128,
    })
    .await
    .expect("bookmark custom reaction");
    drop(app);
    store.close().await;

    let reopened = Arc::new(
        SqliteStore::connect_file(&database_path)
            .await
            .expect("reopen sqlite store"),
    );
    let reopened_app = AppService::new_with_services(
        reopened.clone(),
        reopened.clone(),
        transport,
        Arc::new(NoopHintTransport),
        docs_sync,
        blob_service,
        generate_keys(),
    );
    let bookmarks = reopened_app
        .list_bookmarked_custom_reactions()
        .await
        .expect("list bookmarks after restart");

    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks[0].asset_id, "asset-bookmarked");
    assert_eq!(bookmarks[0].owner_pubkey, foreign_pubkey);
    assert_eq!(bookmarks[0].search_key, "bookmark");
}

#[test]
fn legacy_custom_reaction_records_fall_back_to_asset_id_for_search_key() {
    let owner_pubkey = "b".repeat(64);
    let snapshot = CustomReactionAssetSnapshotV1 {
        asset_id: "asset-legacy".into(),
        owner_pubkey: Pubkey::from(owner_pubkey.as_str()),
        blob_hash: kukuri_core::BlobHash::new("blob-legacy"),
        search_key: "   ".into(),
        mime: "image/png".into(),
        bytes: 128,
        width: 128,
        height: 128,
    };
    let row = BookmarkedCustomReactionRow {
        asset_id: "asset-bookmarked-legacy".into(),
        owner_pubkey: owner_pubkey.clone(),
        blob_hash: kukuri_core::BlobHash::new("blob-bookmarked-legacy"),
        search_key: String::new(),
        mime: "image/png".into(),
        bytes: 128,
        width: 128,
        height: 128,
        bookmarked_at: 1,
    };

    assert_eq!(
        custom_reaction_asset_view_from_snapshot(&snapshot).search_key,
        "asset-legacy"
    );
    assert_eq!(
        bookmarked_custom_reaction_view_from_row(row).search_key,
        "asset-bookmarked-legacy"
    );
}

#[tokio::test]
async fn private_channel_reaction_stays_epoch_scoped_after_rotate() {
    let (app, store, _, _) = local_app_with_memory_services();
    let topic = "kukuri:topic:reaction-private";
    let channel = app
        .create_private_channel(CreatePrivateChannelInput {
            topic_id: TopicId::new(topic),
            label: "friends".into(),
            audience_kind: ChannelAudienceKind::FriendOnly,
        })
        .await
        .expect("create private channel");
    let channel_id = ChannelId::new(channel.channel_id.clone());
    let channel_ref = ChannelRef::PrivateChannel {
        channel_id: channel_id.clone(),
    };
    let old_post_id = app
        .create_post_in_channel(topic, channel_ref.clone(), "before rotate", None)
        .await
        .expect("create old epoch post");
    let old_state = app
        .toggle_reaction(
            topic,
            old_post_id.as_str(),
            ReactionKeyV1::Emoji {
                emoji: "👍".into()
            },
            Some(channel_ref.clone()),
        )
        .await
        .expect("toggle old epoch reaction");
    let old_target = store
        .get_object_projection(&EnvelopeId::from(old_post_id.clone()))
        .await
        .expect("old projection")
        .expect("old target");

    let rotated = app
        .rotate_private_channel(topic, channel.channel_id.as_str())
        .await
        .expect("rotate private channel");
    let new_post_id = app
        .create_post_in_channel(topic, channel_ref.clone(), "after rotate", None)
        .await
        .expect("create new epoch post");
    let new_state = app
        .toggle_reaction(
            topic,
            new_post_id.as_str(),
            ReactionKeyV1::Emoji {
                emoji: "👍".into()
            },
            Some(channel_ref),
        )
        .await
        .expect("toggle new epoch reaction");
    let new_target = store
        .get_object_projection(&EnvelopeId::from(new_post_id.clone()))
        .await
        .expect("new projection")
        .expect("new target");
    let old_rows = store
        .list_reaction_cache_for_target(
            &old_target.source_replica_id,
            &EnvelopeId::from(old_post_id),
        )
        .await
        .expect("old reaction rows");
    let new_rows = store
        .list_reaction_cache_for_target(
            &new_target.source_replica_id,
            &EnvelopeId::from(new_post_id),
        )
        .await
        .expect("new reaction rows");

    assert_ne!(rotated.current_epoch_id, channel.current_epoch_id);
    assert_ne!(old_target.source_replica_id, new_target.source_replica_id);
    assert_eq!(
        old_state.source_replica_id,
        old_target.source_replica_id.as_str()
    );
    assert_eq!(
        new_state.source_replica_id,
        new_target.source_replica_id.as_str()
    );
    assert_eq!(old_rows.len(), 1);
    assert_eq!(new_rows.len(), 1);
    assert_eq!(old_rows[0].status, ObjectStatus::Active);
    assert_eq!(new_rows[0].status, ObjectStatus::Active);
    assert_ne!(old_rows[0].source_replica_id, new_rows[0].source_replica_id);
}
