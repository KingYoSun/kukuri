use super::super::*;

#[tokio::test]
async fn reply_posts_include_parent_preview_in_timeline_views() {
    let (app, _, _, _) = local_app_with_memory_services();
    let topic = "kukuri:topic:reply-preview";

    let root_id = app
        .create_post(topic, "root body", None)
        .await
        .expect("root post");
    let reply_id = app
        .create_post(topic, "reply body", Some(root_id.as_str()))
        .await
        .expect("reply post");

    let timeline = app.list_timeline(topic, None, 20).await.expect("timeline");
    let reply = timeline
        .items
        .iter()
        .find(|post| post.object_id == reply_id)
        .expect("reply post in timeline");
    let preview = reply.reply_preview.as_ref().expect("reply preview");

    assert_eq!(reply.reply_to.as_deref(), Some(root_id.as_str()));
    assert_eq!(preview.object_id, root_id);
    assert_eq!(preview.topic, topic);
    assert_eq!(preview.content, "root body");
    assert_eq!(preview.root_id, None);
    assert_eq!(preview.reply_to, None);
}

#[tokio::test]
async fn reply_preview_is_restored_from_docs_when_parent_projection_is_unavailable() {
    let (app, store, _, _) = local_app_with_memory_services();
    let topic = "kukuri:topic:reply-preview-fallback";

    let root_id = app
        .create_post(topic, "root body", None)
        .await
        .expect("root post");
    let reply_id = app
        .create_post(topic, "reply body", Some(root_id.as_str()))
        .await
        .expect("reply post");

    let reply_projection =
        ProjectionStore::get_object_projection(store.as_ref(), &EnvelopeId::from(reply_id.clone()))
            .await
            .expect("reply projection lookup")
            .expect("reply projection");
    ProjectionStore::rebuild_object_projections(store.as_ref(), vec![reply_projection])
        .await
        .expect("rebuild reply-only projections");

    let timeline = app.list_timeline(topic, None, 20).await.expect("timeline");
    let reply = timeline
        .items
        .iter()
        .find(|post| post.object_id == reply_id)
        .expect("reply post in timeline");
    let preview = reply.reply_preview.as_ref().expect("reply preview");

    assert_eq!(reply.reply_to.as_deref(), Some(root_id.as_str()));
    assert_eq!(preview.object_id, root_id);
    assert_eq!(preview.topic, topic);
    assert_eq!(preview.content, "root body");
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
