use super::super::*;

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
async fn author_withdrawal_scrubs_timeline_bookmark_and_durable_docs_state() {
    let (app, _store, docs, _blobs) = local_app_with_memory_services();
    let topic = "kukuri:topic:withdrawal";
    let object_id = app
        .create_post_with_attachments(
            topic,
            "sensitive body",
            None,
            vec![pending_image_attachment("image/png", &tiny_png_bytes())],
        )
        .await
        .expect("create post");
    app.bookmark_post(topic, &object_id)
        .await
        .expect("bookmark before withdrawal");

    app.withdraw_post(
        topic,
        &object_id,
        ChannelRef::Public,
        None,
        WithdrawalReasonVisibility::Public,
        Some(PostWithdrawalReason::AuthorRequest),
    )
    .await
    .expect("withdraw post");

    let timeline = app.list_timeline(topic, None, 10).await.expect("timeline");
    let withdrawn = &timeline.items[0];
    assert_eq!(withdrawn.object_id, object_id);
    assert_eq!(withdrawn.content, "");
    assert!(withdrawn.attachments.is_empty());
    assert!(withdrawn.withdrawal.is_some());

    let bookmarks = app.list_bookmarked_posts().await.expect("bookmarks");
    assert_eq!(bookmarks[0].post.content, "");
    assert!(bookmarks[0].post.attachments.is_empty());
    assert!(bookmarks[0].post.withdrawal.is_some());

    let records = docs
        .query_replica(
            &topic_replica_id(topic),
            DocQuery::Exact(stable_key("withdrawals", &format!("{object_id}/state"))),
        )
        .await
        .expect("withdrawal docs query");
    assert_eq!(records.len(), 1);
}

#[tokio::test]
async fn non_author_cannot_withdraw_a_post() {
    let (author, _author_keys, other, _other_keys, _store, _docs, _blobs) =
        shared_apps_with_memory_services();
    let topic = "kukuri:topic:withdrawal-auth";
    let object_id = author
        .create_post(topic, "author content", None)
        .await
        .expect("create post");
    let error = other
        .withdraw_post(
            topic,
            &object_id,
            ChannelRef::Public,
            None,
            WithdrawalReasonVisibility::Public,
            Some(PostWithdrawalReason::AuthorRequest),
        )
        .await
        .expect_err("non-author withdrawal must fail");
    assert!(error.to_string().contains("original author"));
}
