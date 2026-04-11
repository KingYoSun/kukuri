use super::*;

#[tokio::test]
async fn mute_author_restores_after_restart() {
    let dir = tempdir().expect("tempdir");
    let database_path = dir.path().join("mute-restart.sqlite");
    let store = Arc::new(
        SqliteStore::connect_file(&database_path)
            .await
            .expect("connect sqlite store"),
    );
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let keys = generate_keys();
    let author_pubkey = generate_keys().public_key_hex();

    let app = AppService::new_with_services(
        store.clone(),
        store.clone(),
        transport.clone(),
        Arc::new(NoopHintTransport),
        docs_sync.clone(),
        blob_service.clone(),
        keys.clone(),
    );
    let muted = app
        .mute_author(author_pubkey.as_str())
        .await
        .expect("mute author");
    assert!(muted.muted);

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
        keys,
    );

    let restored = reopened_app
        .get_author_social_view(author_pubkey.as_str())
        .await
        .expect("get author after restart");
    let muted_authors = reopened_app
        .list_social_connections(SocialConnectionKind::Muted)
        .await
        .expect("list muted authors");

    assert!(restored.muted);
    assert_eq!(muted_authors.len(), 1);
    assert_eq!(muted_authors[0].author_pubkey, author_pubkey);
    assert!(muted_authors[0].muted);
}

#[tokio::test]
async fn muted_author_is_filtered_from_timeline_thread_profile_and_bookmarks() {
    let (local_app, _local_keys, remote_app, remote_keys, _store, _docs_sync, _blob_service) =
        shared_apps_with_memory_services();
    let topic = "kukuri:topic:mute-filter";
    let remote_pubkey = remote_keys.public_key_hex();
    let root_id = remote_app
        .create_post(topic, "muted root", None)
        .await
        .expect("create root post");
    let reply_id = remote_app
        .create_post(topic, "muted reply", Some(root_id.as_str()))
        .await
        .expect("create reply post");

    let timeline_before = local_app
        .list_timeline(topic, None, 20)
        .await
        .expect("timeline before mute");
    let thread_before = local_app
        .list_thread(topic, root_id.as_str(), None, 20)
        .await
        .expect("thread before mute");
    let profile_before = local_app
        .list_profile_timeline(remote_pubkey.as_str(), None, 20)
        .await
        .expect("profile before mute");

    local_app
        .bookmark_post(topic, root_id.as_str())
        .await
        .expect("bookmark muted author post");
    let bookmarks_before = local_app
        .list_bookmarked_posts()
        .await
        .expect("bookmarks before mute");

    assert!(
        timeline_before
            .items
            .iter()
            .any(|post| post.object_id == root_id)
    );
    assert!(
        timeline_before
            .items
            .iter()
            .any(|post| post.object_id == reply_id)
    );
    assert!(
        thread_before
            .items
            .iter()
            .any(|post| post.object_id == root_id)
    );
    assert!(
        thread_before
            .items
            .iter()
            .any(|post| post.object_id == reply_id)
    );
    assert!(
        profile_before
            .items
            .iter()
            .any(|post| post.object_id == root_id)
    );
    assert_eq!(bookmarks_before.len(), 1);
    assert_eq!(bookmarks_before[0].post.object_id, root_id);

    local_app
        .mute_author(remote_pubkey.as_str())
        .await
        .expect("mute remote author");

    let timeline_after = local_app
        .list_timeline(topic, None, 20)
        .await
        .expect("timeline after mute");
    let thread_after = local_app
        .list_thread(topic, root_id.as_str(), None, 20)
        .await
        .expect("thread after mute");
    let profile_after = local_app
        .list_profile_timeline(remote_pubkey.as_str(), None, 20)
        .await
        .expect("profile after mute");
    let bookmarks_after = local_app
        .list_bookmarked_posts()
        .await
        .expect("bookmarks after mute");

    assert!(timeline_after.items.is_empty());
    assert!(thread_after.items.is_empty());
    assert!(profile_after.items.is_empty());
    assert!(bookmarks_after.is_empty());
}

#[tokio::test]
async fn repost_of_muted_author_is_hidden() {
    let (local_app, _local_keys, remote_app, remote_keys, _store, _docs_sync, _blob_service) =
        shared_apps_with_memory_services();
    let topic = "kukuri:topic:mute-repost";
    let remote_pubkey = remote_keys.public_key_hex();
    let source_id = remote_app
        .create_post(topic, "muted source", None)
        .await
        .expect("create muted source post");
    let visible_local_id = local_app
        .create_post(topic, "visible local post", None)
        .await
        .expect("create visible local post");
    let repost_id = local_app
        .create_repost(topic, topic, source_id.as_str(), Some("quote muted source"))
        .await
        .expect("create quote repost");

    let timeline_before = local_app
        .list_timeline(topic, None, 20)
        .await
        .expect("timeline before mute");
    assert!(
        timeline_before
            .items
            .iter()
            .any(|post| post.object_id == repost_id)
    );

    local_app
        .mute_author(remote_pubkey.as_str())
        .await
        .expect("mute source author");

    let timeline_after = local_app
        .list_timeline(topic, None, 20)
        .await
        .expect("timeline after mute");

    assert!(
        timeline_after
            .items
            .iter()
            .all(|post| post.object_id != source_id)
    );
    assert!(
        timeline_after
            .items
            .iter()
            .all(|post| post.object_id != repost_id)
    );
    assert!(
        timeline_after
            .items
            .iter()
            .any(|post| post.object_id == visible_local_id)
    );
}

#[tokio::test]
async fn list_social_connections_followed_is_local_known_only() {
    let (local_app, local_keys, remote_app, remote_keys, _store, _docs_sync, _blob_service) =
        shared_apps_with_memory_services();
    let local_pubkey = local_keys.public_key_hex();
    let remote_pubkey = remote_keys.public_key_hex();
    let unseen_pubkey = generate_keys().public_key_hex();

    remote_app
        .follow_author(local_pubkey.as_str())
        .await
        .expect("remote follows local");
    local_app
        .warm_social_graph()
        .await
        .expect("warm local social graph");

    let followed = local_app
        .list_social_connections(SocialConnectionKind::Followed)
        .await
        .expect("list followed authors");

    assert_eq!(followed.len(), 1);
    assert_eq!(followed[0].author_pubkey, remote_pubkey);
    assert!(followed[0].followed_by);
    assert!(
        followed
            .iter()
            .all(|author| author.author_pubkey != unseen_pubkey)
    );
}

#[tokio::test]
async fn unmute_restores_visibility() {
    let (local_app, _local_keys, remote_app, remote_keys, _store, _docs_sync, _blob_service) =
        shared_apps_with_memory_services();
    let topic = "kukuri:topic:unmute-restore";
    let remote_pubkey = remote_keys.public_key_hex();
    let object_id = remote_app
        .create_post(topic, "restored after unmute", None)
        .await
        .expect("create remote post");

    let timeline_before = local_app
        .list_timeline(topic, None, 20)
        .await
        .expect("timeline before mute");
    assert!(
        timeline_before
            .items
            .iter()
            .any(|post| post.object_id == object_id)
    );

    local_app
        .mute_author(remote_pubkey.as_str())
        .await
        .expect("mute remote author");
    let muted_timeline = local_app
        .list_timeline(topic, None, 20)
        .await
        .expect("timeline while muted");
    assert!(
        muted_timeline
            .items
            .iter()
            .all(|post| post.object_id != object_id)
    );

    local_app
        .unmute_author(remote_pubkey.as_str())
        .await
        .expect("unmute remote author");
    let restored_timeline = local_app
        .list_timeline(topic, None, 20)
        .await
        .expect("timeline after unmute");

    assert!(
        restored_timeline
            .items
            .iter()
            .any(|post| post.object_id == object_id)
    );
}

#[tokio::test]
async fn mute_does_not_change_follow_mutual_or_friend_gating() {
    let (local_app, local_keys, remote_app, remote_keys, _store, _docs_sync, _blob_service) =
        shared_apps_with_memory_services();
    let topic = "kukuri:topic:mute-friend-gating";
    let local_pubkey = local_keys.public_key_hex();
    let remote_pubkey = remote_keys.public_key_hex();

    local_app
        .follow_author(remote_pubkey.as_str())
        .await
        .expect("local follows remote");
    remote_app
        .follow_author(local_pubkey.as_str())
        .await
        .expect("remote follows local");

    local_app
        .mute_author(remote_pubkey.as_str())
        .await
        .expect("mute mutual author");

    let social_view = local_app
        .get_author_social_view(remote_pubkey.as_str())
        .await
        .expect("load muted mutual author");
    let dm_status = local_app
        .get_direct_message_status(remote_pubkey.as_str())
        .await
        .expect("get direct message status");
    let channel = local_app
        .create_private_channel(CreatePrivateChannelInput {
            topic_id: TopicId::new(topic),
            label: "friends".into(),
            audience_kind: ChannelAudienceKind::FriendOnly,
        })
        .await
        .expect("create friend-only channel");
    let grant = local_app
        .export_friend_only_grant(topic, channel.channel_id.as_str(), None)
        .await
        .expect("export friend-only grant after mute");
    let preview = remote_app
        .import_friend_only_grant(grant.as_str())
        .await
        .expect("import friend-only grant after mute");

    assert!(social_view.following);
    assert!(social_view.followed_by);
    assert!(social_view.mutual);
    assert!(social_view.muted);
    assert!(dm_status.mutual);
    assert!(dm_status.send_enabled);
    assert_eq!(preview.channel_id.as_str(), channel.channel_id);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn social_graph_derives_friend_of_friend_and_clears_after_unfollow() {
    let _guard = iroh_integration_test_lock().lock_owned().await;
    let dir = tempdir().expect("tempdir");
    let stack_a = TestIrohStack::new(&dir.path().join("author-a")).await;
    let stack_b = TestIrohStack::new(&dir.path().join("author-b")).await;
    let store_a = Arc::new(MemoryStore::default());
    let store_b = Arc::new(MemoryStore::default());
    let keys_a = generate_keys();
    let keys_b = generate_keys();
    let keys_c = generate_keys();
    let app_a = AppService::new_with_services(
        store_a.clone(),
        store_a.clone(),
        stack_a.transport.clone(),
        stack_a.transport.clone(),
        stack_a.docs_sync.clone(),
        stack_a.blob_service.clone(),
        keys_a.clone(),
    );
    let app_b = AppService::new_with_services(
        store_b.clone(),
        store_b.clone(),
        stack_b.transport.clone(),
        stack_b.transport.clone(),
        stack_b.docs_sync.clone(),
        stack_b.blob_service.clone(),
        keys_b.clone(),
    );
    app_a
        .warm_social_graph()
        .await
        .expect("warm social graph a");
    app_b
        .warm_social_graph()
        .await
        .expect("warm social graph b");

    let ticket_a = stack_a
        .transport
        .export_ticket()
        .await
        .expect("ticket a")
        .expect("ticket a value");
    let ticket_b = stack_b
        .transport
        .export_ticket()
        .await
        .expect("ticket b")
        .expect("ticket b value");
    app_a.import_peer_ticket(&ticket_b).await.expect("import b");
    app_b.import_peer_ticket(&ticket_a).await.expect("import a");

    let b_pubkey = keys_b.public_key_hex();
    let c_pubkey = keys_c.public_key_hex();
    app_a
        .follow_author(b_pubkey.as_str())
        .await
        .expect("a follows b");
    app_b
        .follow_author(c_pubkey.as_str())
        .await
        .expect("b follows c");

    timeout(Duration::from_secs(10), async {
        loop {
            let social_view = app_a
                .get_author_social_view(c_pubkey.as_str())
                .await
                .expect("load c social view");
            if social_view.friend_of_friend {
                assert_eq!(
                    social_view.friend_of_friend_via_pubkeys,
                    vec![b_pubkey.clone()]
                );
                break;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("derive friend of friend");

    let b_view = app_a
        .get_author_social_view(b_pubkey.as_str())
        .await
        .expect("load b social view");
    assert!(b_view.following);
    assert!(!b_view.friend_of_friend);

    app_a
        .unfollow_author(b_pubkey.as_str())
        .await
        .expect("a unfollows b");

    let c_view = app_a
        .get_author_social_view(c_pubkey.as_str())
        .await
        .expect("load c social view after unfollow");
    assert!(!c_view.friend_of_friend);
    assert!(c_view.friend_of_friend_via_pubkeys.is_empty());
}
