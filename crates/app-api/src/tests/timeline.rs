use super::*;

#[tokio::test]
async fn create_post_and_list_timeline() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
    let app = AppService::new(store, transport);

    let object_id = app
        .create_post("kukuri:topic:api", "hello app", None)
        .await
        .expect("create post");
    let timeline = app
        .list_timeline("kukuri:topic:api", None, 10)
        .await
        .expect("timeline");

    assert_eq!(timeline.items.len(), 1);
    assert_eq!(timeline.items[0].object_id, object_id);
    assert_eq!(timeline.items[0].content, "hello app");
}

#[tokio::test]
async fn local_bookmarked_posts_restore_after_restart() {
    let dir = tempdir().expect("tempdir");
    let database_path = dir.path().join("bookmark-post-store.sqlite");
    let store = Arc::new(
        SqliteStore::connect_file(&database_path)
            .await
            .expect("sqlite store"),
    );
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let app = AppService::new_with_services(
        store.clone(),
        store.clone(),
        transport.clone(),
        Arc::new(NoopHintTransport),
        docs_sync.clone(),
        blob_service.clone(),
        generate_keys(),
    );
    let topic = "kukuri:topic:bookmark-restore";
    let object_id = app
        .create_post(topic, "saved for restart", None)
        .await
        .expect("create post");

    let bookmarked = app
        .bookmark_post(topic, object_id.as_str())
        .await
        .expect("bookmark post");
    assert_eq!(bookmarked.post.object_id, object_id);

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
        .list_bookmarked_posts()
        .await
        .expect("list bookmarked posts after restart");

    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks[0].post.object_id, object_id);
    assert_eq!(bookmarks[0].post.content, "saved for restart");
    assert!(bookmarks[0].bookmarked_at > 0);
}

#[tokio::test]
async fn bookmark_private_post_remains_local_only_and_readable_after_access_loss() {
    let (app, store, _, _) = local_app_with_memory_services();
    let topic = "kukuri:topic:bookmark-private";
    let channel = app
        .create_private_channel(CreatePrivateChannelInput {
            topic_id: TopicId::new(topic),
            label: "quiet room".into(),
            audience_kind: ChannelAudienceKind::InviteOnly,
        })
        .await
        .expect("create private channel");
    let channel_ref = ChannelRef::PrivateChannel {
        channel_id: ChannelId::new(channel.channel_id.clone()),
    };
    let object_id = app
        .create_post_in_channel(topic, channel_ref, "private bookmark body", None)
        .await
        .expect("create private post");

    app.bookmark_post(topic, object_id.as_str())
        .await
        .expect("bookmark private post");
    app.joined_private_channels.lock().await.clear();
    ProjectionStore::rebuild_object_projections(store.as_ref(), Vec::new())
        .await
        .expect("clear object projections");

    let bookmarks = app
        .list_bookmarked_posts()
        .await
        .expect("list bookmarked private posts");

    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks[0].post.object_id, object_id);
    assert_eq!(bookmarks[0].post.content, "private bookmark body");
    assert_eq!(
        bookmarks[0].post.channel_id.as_deref(),
        Some(channel.channel_id.as_str())
    );
    assert_eq!(bookmarks[0].post.audience_label, "Private channel");
}

#[tokio::test]
async fn unbookmark_removes_only_local_bookmark_record() {
    let (app, store, _, _) = local_app_with_memory_services();
    let topic = "kukuri:topic:bookmark-remove";
    let object_id = app
        .create_post(topic, "still on timeline", None)
        .await
        .expect("create post");

    app.bookmark_post(topic, object_id.as_str())
        .await
        .expect("bookmark post");
    app.remove_bookmarked_post(object_id.as_str())
        .await
        .expect("remove bookmark");

    let bookmarks = app.list_bookmarked_posts().await.expect("list bookmarks");
    let timeline = app
        .list_timeline(topic, None, 20)
        .await
        .expect("list timeline");
    let projection = ProjectionStore::get_object_projection(
        store.as_ref(),
        &EnvelopeId::from(object_id.clone()),
    )
    .await
    .expect("get projection");

    assert!(bookmarks.is_empty());
    assert!(
        timeline
            .items
            .iter()
            .any(|post| post.object_id == object_id)
    );
    assert!(projection.is_some());
}

#[tokio::test]
async fn bookmarked_posts_are_sorted_by_bookmarked_at_desc() {
    let (app, _, _, _) = local_app_with_memory_services();
    let topic = "kukuri:topic:bookmark-order";
    let first_id = app
        .create_post(topic, "first saved", None)
        .await
        .expect("create first post");
    app.bookmark_post(topic, first_id.as_str())
        .await
        .expect("bookmark first post");
    sleep(Duration::from_millis(5)).await;
    let second_id = app
        .create_post(topic, "second saved", None)
        .await
        .expect("create second post");
    app.bookmark_post(topic, second_id.as_str())
        .await
        .expect("bookmark second post");

    let bookmarks = app.list_bookmarked_posts().await.expect("list bookmarks");

    assert_eq!(bookmarks.len(), 2);
    assert_eq!(bookmarks[0].post.object_id, second_id);
    assert_eq!(bookmarks[1].post.object_id, first_id);
    assert!(bookmarks[0].bookmarked_at >= bookmarks[1].bookmarked_at);
}

#[tokio::test]
async fn bookmarked_repost_renders_from_saved_snapshot_without_source_timeline_hydration() {
    let (app, store, _, _) = local_app_with_memory_services();
    let source_topic = "kukuri:topic:bookmark-source";
    let target_topic = "kukuri:topic:bookmark-target";
    let source_object_id = app
        .create_post(source_topic, "source body", None)
        .await
        .expect("create source post");
    let repost_id = app
        .create_repost(
            target_topic,
            source_topic,
            source_object_id.as_str(),
            Some("keep this"),
        )
        .await
        .expect("create repost");

    app.bookmark_post(target_topic, repost_id.as_str())
        .await
        .expect("bookmark repost");
    ProjectionStore::rebuild_object_projections(store.as_ref(), Vec::new())
        .await
        .expect("clear projections");

    let bookmarks = app
        .list_bookmarked_posts()
        .await
        .expect("list bookmarked reposts");

    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks[0].post.object_id, repost_id);
    assert_eq!(bookmarks[0].post.content, "keep this");
    let repost_of = bookmarks[0]
        .post
        .repost_of
        .as_ref()
        .expect("repost snapshot");
    assert_eq!(repost_of.source_object_id, source_object_id);
    assert_eq!(repost_of.source_topic_id, source_topic);
    assert_eq!(repost_of.content, "source body");
}

#[tokio::test]
async fn create_public_post_persists_profile_post_doc_and_lists_profile_timeline() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let keys = generate_keys();
    let author_pubkey = keys.public_key_hex();
    let app = AppService::new_with_services(
        store.clone(),
        store,
        transport.clone(),
        Arc::new(NoopHintTransport),
        docs_sync.clone(),
        blob_service,
        keys,
    );
    let topic = "kukuri:topic:profile-doc";

    let object_id = app
        .create_post(topic, "hello profile", None)
        .await
        .expect("create post");
    let profile_docs = author_profile_post_docs(docs_sync.as_ref(), author_pubkey.as_str()).await;

    assert_eq!(profile_docs.len(), 1);
    assert_eq!(profile_docs[0].author_pubkey.as_str(), author_pubkey);
    assert_eq!(
        profile_docs[0].profile_topic_id,
        author_profile_topic_id(author_pubkey.as_str())
    );
    assert_eq!(profile_docs[0].published_topic_id.as_str(), topic);
    assert_eq!(profile_docs[0].object_id.as_str(), object_id);
    assert_eq!(profile_docs[0].object_kind, "post");

    let timeline = app
        .list_profile_timeline(author_pubkey.as_str(), None, 20)
        .await
        .expect("profile timeline");
    let post = timeline
        .items
        .iter()
        .find(|post| post.object_id == object_id)
        .expect("profile post");

    assert_eq!(post.content, "hello profile");
    assert_eq!(post.published_topic_id.as_deref(), Some(topic));
    assert_eq!(post.channel_id, None);
    assert_eq!(post.audience_label, "Public");
    assert!(post.reaction_summary.is_empty());
    assert!(post.my_reactions.is_empty());
}

#[tokio::test]
async fn public_reply_is_indexed_in_profile_timeline() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let keys = generate_keys();
    let author_pubkey = keys.public_key_hex();
    let app = AppService::new_with_services(
        store.clone(),
        store,
        transport.clone(),
        Arc::new(NoopHintTransport),
        docs_sync.clone(),
        blob_service,
        keys,
    );
    let topic = "kukuri:topic:profile-replies";

    let root_id = app
        .create_post(topic, "root", None)
        .await
        .expect("root post");
    let reply_id = app
        .create_post(topic, "reply", Some(root_id.as_str()))
        .await
        .expect("reply post");
    let profile_docs = author_profile_post_docs(docs_sync.as_ref(), author_pubkey.as_str()).await;
    let reply_doc = profile_docs
        .iter()
        .find(|doc| doc.object_id.as_str() == reply_id)
        .expect("reply profile doc");

    assert_eq!(reply_doc.object_kind, "comment");
    assert_eq!(
        reply_doc.reply_to_object_id.as_ref().map(|id| id.as_str()),
        Some(root_id.as_str())
    );
    assert_eq!(
        reply_doc.root_id.as_ref().map(|id| id.as_str()),
        Some(root_id.as_str())
    );

    let timeline = app
        .list_profile_timeline(author_pubkey.as_str(), None, 20)
        .await
        .expect("profile timeline");
    let reply = timeline
        .items
        .iter()
        .find(|post| post.object_id == reply_id)
        .expect("profile reply");

    assert_eq!(reply.object_kind, "comment");
    assert_eq!(reply.reply_to.as_deref(), Some(root_id.as_str()));
    assert_eq!(reply.root_id.as_deref(), Some(root_id.as_str()));
    assert_eq!(reply.published_topic_id.as_deref(), Some(topic));
}

#[tokio::test]
async fn private_channel_post_is_not_indexed_in_profile_timeline() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let keys = generate_keys();
    let author_pubkey = keys.public_key_hex();
    let app = AppService::new_with_services(
        store.clone(),
        store,
        transport.clone(),
        Arc::new(NoopHintTransport),
        docs_sync.clone(),
        blob_service,
        keys,
    );
    let topic = "kukuri:topic:profile-private";
    let channel = app
        .create_private_channel(CreatePrivateChannelInput {
            topic_id: TopicId::new(topic),
            label: "core".into(),
            audience_kind: ChannelAudienceKind::InviteOnly,
        })
        .await
        .expect("create private channel");

    let private_object_id = app
        .create_post_in_channel(
            topic,
            ChannelRef::PrivateChannel {
                channel_id: ChannelId::new(channel.channel_id.clone()),
            },
            "private hello",
            None,
        )
        .await
        .expect("create private post");

    let profile_docs = author_profile_post_docs(docs_sync.as_ref(), author_pubkey.as_str()).await;
    assert!(profile_docs.is_empty());

    let timeline = app
        .list_profile_timeline(author_pubkey.as_str(), None, 20)
        .await
        .expect("profile timeline");
    assert!(
        timeline
            .items
            .iter()
            .all(|post| post.object_id != private_object_id)
    );
}

#[tokio::test]
async fn set_my_profile_with_avatar_upload_persists_blob_backed_profile_and_author_view() {
    let (app, store, docs_sync, blob_service) = local_app_with_memory_services();
    let avatar_bytes = tiny_png_bytes();

    let updated = app
        .set_my_profile(ProfileInput {
            name: Some("avatar-owner".into()),
            display_name: Some("Avatar Owner".into()),
            about: Some("blob avatar".into()),
            picture: None,
            picture_upload: Some(PendingAttachment {
                mime: "image/png".into(),
                bytes: avatar_bytes.clone(),
                role: AssetRole::ProfileAvatar,
            }),
            clear_picture: false,
        })
        .await
        .expect("set profile");

    let asset = updated.picture_asset.clone().expect("profile avatar asset");
    let stored_profile = store
        .get_profile(updated.pubkey.as_str())
        .await
        .expect("stored profile")
        .expect("stored profile value");
    let profile_doc = author_profile_doc(docs_sync.as_ref(), updated.pubkey.as_str())
        .await
        .expect("profile doc");
    let stored_blob = blob_service
        .fetch_blob(&asset.hash)
        .await
        .expect("fetch avatar blob")
        .expect("avatar blob");
    let local_profile = app.get_my_profile().await.expect("get my profile");
    let author_social = app
        .get_author_social_view(updated.pubkey.as_str())
        .await
        .expect("author social view");

    assert_eq!(updated.picture, None);
    assert_eq!(asset.mime, "image/png");
    assert_eq!(asset.role, AssetRole::ProfileAvatar);
    assert_eq!(stored_blob, avatar_bytes);
    assert_eq!(stored_profile.picture_asset, updated.picture_asset);
    assert_eq!(profile_doc.picture_asset, updated.picture_asset);
    assert_eq!(local_profile.picture_asset, updated.picture_asset);
    assert_eq!(author_social.picture, None);
    assert_eq!(
        author_social
            .picture_asset
            .as_ref()
            .map(|value| value.hash.as_str()),
        Some(asset.hash.as_str())
    );
    assert_eq!(
        author_social
            .picture_asset
            .as_ref()
            .map(|value| value.mime.as_str()),
        Some("image/png")
    );
    assert_eq!(
        author_social
            .picture_asset
            .as_ref()
            .map(|value| value.role.as_str()),
        Some("profile_avatar")
    );
}

#[tokio::test]
async fn set_my_profile_keeps_legacy_picture_url_backward_compatible() {
    let (app, store, docs_sync, _) = local_app_with_memory_services();
    let legacy_picture = "https://example.com/avatar.png".to_string();

    let updated = app
        .set_my_profile(ProfileInput {
            name: Some("legacy-owner".into()),
            display_name: Some("Legacy Owner".into()),
            about: Some("legacy avatar".into()),
            picture: Some(legacy_picture.clone()),
            picture_upload: None,
            clear_picture: false,
        })
        .await
        .expect("set profile");

    let stored_profile = store
        .get_profile(updated.pubkey.as_str())
        .await
        .expect("stored profile")
        .expect("stored profile value");
    let profile_doc = author_profile_doc(docs_sync.as_ref(), updated.pubkey.as_str())
        .await
        .expect("profile doc");
    let local_profile = app.get_my_profile().await.expect("get my profile");
    let author_social = app
        .get_author_social_view(updated.pubkey.as_str())
        .await
        .expect("author social view");

    assert_eq!(updated.picture.as_deref(), Some(legacy_picture.as_str()));
    assert_eq!(updated.picture_asset, None);
    assert_eq!(
        stored_profile.picture.as_deref(),
        Some(legacy_picture.as_str())
    );
    assert_eq!(stored_profile.picture_asset, None);
    assert_eq!(
        profile_doc.picture.as_deref(),
        Some(legacy_picture.as_str())
    );
    assert_eq!(profile_doc.picture_asset, None);
    assert_eq!(
        local_profile.picture.as_deref(),
        Some(legacy_picture.as_str())
    );
    assert_eq!(local_profile.picture_asset, None);
    assert_eq!(
        author_social.picture.as_deref(),
        Some(legacy_picture.as_str())
    );
    assert_eq!(author_social.picture_asset, None);
}

#[tokio::test]
async fn create_same_topic_repost_persists_repost_object_and_profile_repost_doc() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let keys = generate_keys();
    let author_pubkey = keys.public_key_hex();
    let app = AppService::new_with_services(
        store.clone(),
        store,
        transport.clone(),
        Arc::new(NoopHintTransport),
        docs_sync.clone(),
        blob_service,
        keys,
    );
    let topic = "kukuri:topic:repost-same";

    let source_object_id = app
        .create_post(topic, "hello repost", None)
        .await
        .expect("create source post");
    let repost_object_id = app
        .create_repost(topic, topic, source_object_id.as_str(), None)
        .await
        .expect("create repost");

    let timeline = app.list_timeline(topic, None, 20).await.expect("timeline");
    let repost = timeline
        .items
        .iter()
        .find(|post| post.object_id == repost_object_id)
        .expect("repost item");

    assert_eq!(repost.object_kind, "repost");
    assert_eq!(repost.published_topic_id.as_deref(), Some(topic));
    assert!(repost.repost_of.is_some());
    assert_eq!(repost.repost_commentary, None);
    assert!(!repost.is_threadable);

    let profile_docs = author_profile_repost_docs(docs_sync.as_ref(), author_pubkey.as_str()).await;
    assert_eq!(profile_docs.len(), 1);
    assert_eq!(profile_docs[0].object_id.as_str(), repost_object_id);
    assert_eq!(profile_docs[0].published_topic_id.as_str(), topic);
    assert_eq!(profile_docs[0].repost_of.source_topic_id.as_str(), topic);
}

#[tokio::test]
async fn create_cross_topic_repost_renders_from_target_topic_without_tracking_source_topic() {
    let store_a = Arc::new(MemoryStore::default());
    let store_b = Arc::new(MemoryStore::default());
    let peer_snapshot = PeerSnapshot::default();
    let transport_a = Arc::new(StaticTransport::new(peer_snapshot.clone()));
    let transport_b = Arc::new(StaticTransport::new(peer_snapshot));
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let keys_a = generate_keys();
    let author_pubkey = keys_a.public_key_hex();
    let app_a = AppService::new_with_services(
        store_a.clone(),
        store_a,
        transport_a,
        Arc::new(NoopHintTransport),
        docs_sync.clone(),
        blob_service.clone(),
        keys_a,
    );
    let app_b = AppService::new_with_services(
        store_b.clone(),
        store_b,
        transport_b,
        Arc::new(NoopHintTransport),
        docs_sync.clone(),
        blob_service,
        generate_keys(),
    );
    let source_topic = "kukuri:topic:repost-source";
    let target_topic = "kukuri:topic:repost-target";

    let source_object_id = app_a
        .create_post(source_topic, "source post", None)
        .await
        .expect("create source post");
    let repost_object_id = app_a
        .create_repost(target_topic, source_topic, source_object_id.as_str(), None)
        .await
        .expect("create cross-topic repost");

    let target_timeline = app_b
        .list_timeline(target_topic, None, 20)
        .await
        .expect("target timeline");
    let repost = target_timeline
        .items
        .iter()
        .find(|post| post.object_id == repost_object_id)
        .expect("repost item");

    assert_eq!(repost.object_kind, "repost");
    assert_eq!(repost.published_topic_id.as_deref(), Some(target_topic));
    assert_eq!(
        repost
            .repost_of
            .as_ref()
            .map(|value| value.source_topic_id.as_str()),
        Some(source_topic)
    );
    assert_eq!(
        repost
            .repost_of
            .as_ref()
            .map(|value| value.source_object_id.as_str()),
        Some(source_object_id.as_str())
    );
    assert_eq!(
        repost
            .repost_of
            .as_ref()
            .map(|value| value.content.as_str()),
        Some("source post")
    );

    let profile_timeline = app_a
        .list_profile_timeline(author_pubkey.as_str(), None, 20)
        .await
        .expect("profile timeline");
    assert!(
        profile_timeline
            .items
            .iter()
            .any(|post| post.object_id == repost_object_id)
    );
}

#[tokio::test]
async fn simple_repost_is_unique_per_author_target_and_original() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let app = AppService::new_with_services(
        store.clone(),
        store,
        transport.clone(),
        Arc::new(NoopHintTransport),
        docs_sync,
        blob_service,
        generate_keys(),
    );
    let source_topic = "kukuri:topic:repost-unique-source";
    let target_topic = "kukuri:topic:repost-unique-target";

    let source_object_id = app
        .create_post(source_topic, "source post", None)
        .await
        .expect("create source post");
    let repost_a = app
        .create_repost(target_topic, source_topic, source_object_id.as_str(), None)
        .await
        .expect("create first repost");
    let repost_b = app
        .create_repost(target_topic, source_topic, source_object_id.as_str(), None)
        .await
        .expect("create second repost");

    assert_eq!(repost_a, repost_b);

    let timeline = app
        .list_timeline(target_topic, None, 20)
        .await
        .expect("timeline");
    assert_eq!(
        timeline
            .items
            .iter()
            .filter(|post| post.object_id == repost_a)
            .count(),
        1
    );
}

#[tokio::test]
async fn quote_repost_allows_multiple_distinct_quotes_for_same_original() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let app = AppService::new_with_services(
        store.clone(),
        store,
        transport.clone(),
        Arc::new(NoopHintTransport),
        docs_sync,
        blob_service,
        generate_keys(),
    );
    let source_topic = "kukuri:topic:quote-source";
    let target_topic = "kukuri:topic:quote-target";

    let source_object_id = app
        .create_post(source_topic, "quoted source", None)
        .await
        .expect("create source post");
    let quote_a = app
        .create_repost(
            target_topic,
            source_topic,
            source_object_id.as_str(),
            Some("first quote"),
        )
        .await
        .expect("create first quote repost");
    let quote_b = app
        .create_repost(
            target_topic,
            source_topic,
            source_object_id.as_str(),
            Some("second quote"),
        )
        .await
        .expect("create second quote repost");

    assert_ne!(quote_a, quote_b);

    let timeline = app
        .list_timeline(target_topic, None, 20)
        .await
        .expect("timeline");
    assert!(
        timeline
            .items
            .iter()
            .filter(|post| post.object_kind == "repost")
            .count()
            >= 2
    );
}

#[tokio::test]
async fn quote_repost_opens_own_thread_and_simple_repost_cannot_be_reply_parent() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let app = AppService::new_with_services(
        store.clone(),
        store,
        transport.clone(),
        Arc::new(NoopHintTransport),
        docs_sync,
        blob_service,
        generate_keys(),
    );
    let source_topic = "kukuri:topic:reply-source";
    let target_topic = "kukuri:topic:reply-target";

    let source_object_id = app
        .create_post(source_topic, "source post", None)
        .await
        .expect("create source post");
    let simple_repost_id = app
        .create_repost(target_topic, source_topic, source_object_id.as_str(), None)
        .await
        .expect("create simple repost");
    let quote_repost_id = app
        .create_repost(
            target_topic,
            source_topic,
            source_object_id.as_str(),
            Some("quoted reply target"),
        )
        .await
        .expect("create quote repost");

    let simple_reply_error = app
        .create_post(
            target_topic,
            "reply to simple repost",
            Some(simple_repost_id.as_str()),
        )
        .await
        .expect_err("simple repost should reject replies");
    assert!(
        simple_reply_error
            .to_string()
            .contains("simple repost cannot be a reply parent")
    );

    let reply_id = app
        .create_post(
            target_topic,
            "reply to quote repost",
            Some(quote_repost_id.as_str()),
        )
        .await
        .expect("reply to quote repost");
    let thread = app
        .list_thread(target_topic, quote_repost_id.as_str(), None, 20)
        .await
        .expect("quote repost thread");
    assert!(
        thread
            .items
            .iter()
            .any(|post| post.object_id == quote_repost_id)
    );
    assert!(thread.items.iter().any(|post| post.object_id == reply_id));
}

#[tokio::test]
async fn private_channel_post_cannot_be_reposted_publicly() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let app = AppService::new_with_services(
        store.clone(),
        store,
        transport.clone(),
        Arc::new(NoopHintTransport),
        docs_sync,
        blob_service,
        generate_keys(),
    );
    let topic = "kukuri:topic:repost-private";
    let channel = app
        .create_private_channel(CreatePrivateChannelInput {
            topic_id: TopicId::new(topic),
            label: "core".into(),
            audience_kind: ChannelAudienceKind::InviteOnly,
        })
        .await
        .expect("create private channel");
    let private_object_id = app
        .create_post_in_channel(
            topic,
            ChannelRef::PrivateChannel {
                channel_id: ChannelId::new(channel.channel_id.clone()),
            },
            "private source",
            None,
        )
        .await
        .expect("create private post");

    let error = app
        .create_repost(topic, topic, private_object_id.as_str(), None)
        .await
        .expect_err("private post should not be repostable");
    assert!(
        error
            .to_string()
            .contains("only public posts and comments can be reposted")
    );
}

#[tokio::test]
async fn list_profile_timeline_ignores_profile_post_with_signer_mismatch() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let keys = generate_keys();
    let author_pubkey = keys.public_key_hex();
    let app = AppService::new_with_services(
        store.clone(),
        store,
        transport.clone(),
        Arc::new(NoopHintTransport),
        docs_sync.clone(),
        blob_service,
        keys,
    );
    let topic = "kukuri:topic:profile-invalid";
    let valid_object_id = app
        .create_post(topic, "valid profile post", None)
        .await
        .expect("valid post");
    let forged_content = KukuriProfilePostEnvelopeContentV1 {
        author_pubkey: Pubkey::from(author_pubkey.as_str()),
        profile_topic_id: author_profile_topic_id(author_pubkey.as_str()),
        published_topic_id: TopicId::new(topic),
        object_id: EnvelopeId::from("forged-profile-post"),
        created_at: 123,
        object_kind: "post".into(),
        content: "forged profile post".into(),
        attachments: Vec::new(),
        reply_to_object_id: None,
        root_id: None,
    };
    let forged_envelope = kukuri_core::sign_envelope_json(
        &generate_keys(),
        "profile-post",
        vec![
            vec!["author".into(), author_pubkey.clone()],
            vec!["object".into(), "profile-post".into()],
            vec!["published_topic".into(), topic.into()],
            vec!["post".into(), forged_content.object_id.as_str().to_string()],
        ],
        &forged_content,
    )
    .expect("forged envelope");
    let replica = author_replica_id(author_pubkey.as_str());
    docs_sync
        .open_replica(&replica)
        .await
        .expect("open author replica");
    docs_sync
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("profile/posts", forged_content.object_id.as_str()),
                value: serde_json::to_value(AuthorProfilePostDocV1 {
                    author_pubkey: forged_content.author_pubkey.clone(),
                    profile_topic_id: forged_content.profile_topic_id.clone(),
                    published_topic_id: forged_content.published_topic_id.clone(),
                    object_id: forged_content.object_id.clone(),
                    created_at: forged_content.created_at,
                    object_kind: forged_content.object_kind.clone(),
                    content: forged_content.content.clone(),
                    attachments: forged_content.attachments.clone(),
                    reply_to_object_id: None,
                    root_id: None,
                    envelope_id: forged_envelope.id.clone(),
                })
                .expect("forged doc json"),
            },
        )
        .await
        .expect("persist forged profile doc");
    docs_sync
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("envelopes", forged_envelope.id.as_str()),
                value: serde_json::to_value(&forged_envelope).expect("forged envelope json"),
            },
        )
        .await
        .expect("persist forged envelope");

    let profile_docs = author_profile_post_docs(docs_sync.as_ref(), author_pubkey.as_str()).await;
    assert_eq!(profile_docs.len(), 2);

    let timeline = app
        .list_profile_timeline(author_pubkey.as_str(), None, 20)
        .await
        .expect("profile timeline");

    assert!(
        timeline
            .items
            .iter()
            .any(|post| post.object_id == valid_object_id)
    );
    assert!(
        timeline
            .items
            .iter()
            .all(|post| post.object_id != forged_content.object_id.as_str())
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bookmarks_do_not_sync_between_apps() {
    let _guard = iroh_integration_test_lock().lock_owned().await;
    let dir = tempdir().expect("tempdir");
    let stack_a = TestIrohStack::new(&dir.path().join("bookmark-a")).await;
    let stack_b = TestIrohStack::new(&dir.path().join("bookmark-b")).await;
    let store_a = Arc::new(MemoryStore::default());
    let store_b = Arc::new(MemoryStore::default());
    let app_a = app_with_iroh_services(store_a, &stack_a);
    let app_b = app_with_iroh_services(store_b.clone(), &stack_b);

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
    app_a
        .import_peer_ticket(&ticket_b)
        .await
        .expect("import b into a");
    app_b
        .import_peer_ticket(&ticket_a)
        .await
        .expect("import a into b");

    let topic = "kukuri:topic:bookmark-local-only";
    let _ = app_b
        .list_timeline(topic, None, 20)
        .await
        .expect("app b should subscribe to topic");

    let object_id = app_a
        .create_post(topic, "bookmark should stay local", None)
        .await
        .expect("app a should create post");

    timeout(Duration::from_secs(30), async {
        loop {
            let timeline = app_b
                .list_timeline(topic, None, 20)
                .await
                .expect("timeline should load");
            if timeline
                .items
                .iter()
                .any(|post| post.object_id == object_id)
            {
                return;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("timeline sync timeout");

    app_a
        .bookmark_post(topic, object_id.as_str())
        .await
        .expect("bookmark post on app a");

    let bookmarks_a = app_a
        .list_bookmarked_posts()
        .await
        .expect("list bookmarks on app a");
    let bookmarks_b = app_b
        .list_bookmarked_posts()
        .await
        .expect("list bookmarks on app b");

    assert_eq!(bookmarks_a.len(), 1);
    assert_eq!(bookmarks_a[0].post.object_id, object_id);
    assert!(bookmarks_b.is_empty());
}
