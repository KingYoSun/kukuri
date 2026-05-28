use super::super::*;

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
