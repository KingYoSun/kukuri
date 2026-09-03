use super::*;
use kukuri_store::NotificationStore;

#[tokio::test]
async fn adult_labeled_object_notification_preview_is_hidden_until_display_enabled() {
    let (app, store, docs_sync, blob_service) = local_app_with_memory_services();
    let topic = TopicId::new("notifications-adult-mention");
    let remote_keys = generate_keys();
    let adult_preview = format!("adult preview @{}", app.current_author_pubkey());
    let remote_envelope = persist_test_post_with_labels(
        docs_sync.as_ref(),
        None,
        &remote_keys,
        &topic,
        PayloadRef::InlineText {
            text: adult_preview.clone(),
        },
        Vec::new(),
        None,
        vec![kukuri_core::ADULT_CONTENT_LABEL.to_string()],
    )
    .await;
    let remote_object = remote_envelope
        .to_post_object()
        .expect("parse remote adult mention")
        .expect("remote adult mention object");

    assert!(
        create_remote_object_notification(
            &app,
            store.as_ref(),
            docs_sync.as_ref(),
            blob_service.as_ref(),
            remote_doc_event(
                docs_sync.as_ref(),
                &topic_replica_id(topic.as_str()),
                stable_key(
                    "objects",
                    &format!("{}/state", remote_object.object_id.as_str()),
                ),
            )
            .await,
        )
        .await
    );

    let rows = NotificationStore::list_notifications(store.as_ref())
        .await
        .expect("list notification rows");
    assert_eq!(
        rows[0].content_labels.as_deref(),
        Some(&[kukuri_core::ADULT_CONTENT_LABEL.to_string()][..])
    );

    let hidden = app
        .list_notifications()
        .await
        .expect("list hidden notification");
    assert_eq!(hidden.len(), 1);
    assert_eq!(hidden[0].kind, NotificationKind::Mention);
    assert_eq!(hidden[0].preview_text, None);

    app.set_adult_content_display_enabled(true);
    let visible = app
        .list_notifications()
        .await
        .expect("list visible notification");
    assert_eq!(
        visible[0].preview_text.as_deref(),
        Some(adult_preview.as_str())
    );
}

#[tokio::test]
async fn adult_labeled_repost_source_notification_preview_is_hidden_until_display_enabled() {
    let (app, store, docs_sync, blob_service) = local_app_with_memory_services();
    let topic = TopicId::new("notifications-adult-repost-source");
    let source_object_id = app
        .create_post_with_attachments_in_channel(
            topic.as_str(),
            kukuri_core::ChannelRef::Public,
            "adult repost source",
            None,
            Vec::new(),
            vec![kukuri_core::ADULT_CONTENT_LABEL.to_string()],
        )
        .await
        .expect("create adult repost source");
    let repost_source = app
        .resolve_repost_source(topic.as_str(), source_object_id.as_str())
        .await
        .expect("resolve adult repost source");
    let remote_keys = generate_keys();
    let remote_envelope =
        build_repost_envelope(&remote_keys, &topic, repost_source.repost_of, None)
            .expect("build simple repost");
    let remote_object = remote_envelope
        .to_post_object()
        .expect("parse simple repost")
        .expect("simple repost object");
    persist_post_object(
        docs_sync.as_ref(),
        &topic_replica_id(topic.as_str()),
        remote_object.clone(),
        remote_envelope,
    )
    .await
    .expect("persist simple repost");

    assert!(
        create_remote_object_notification(
            &app,
            store.as_ref(),
            docs_sync.as_ref(),
            blob_service.as_ref(),
            remote_doc_event(
                docs_sync.as_ref(),
                &topic_replica_id(topic.as_str()),
                stable_key(
                    "objects",
                    &format!("{}/state", remote_object.object_id.as_str()),
                ),
            )
            .await,
        )
        .await
    );

    let rows = NotificationStore::list_notifications(store.as_ref())
        .await
        .expect("list notification rows");
    assert_eq!(
        rows[0].content_labels.as_deref(),
        Some(&[kukuri_core::ADULT_CONTENT_LABEL.to_string()][..])
    );

    let hidden = app
        .list_notifications()
        .await
        .expect("list hidden notification");
    assert_eq!(hidden[0].kind, NotificationKind::Repost);
    assert_eq!(hidden[0].preview_text, None);

    app.set_adult_content_display_enabled(true);
    let visible = app
        .list_notifications()
        .await
        .expect("list visible notification");
    assert_eq!(
        visible[0].preview_text.as_deref(),
        Some("adult repost source")
    );
}

// #858 / ADR 0046: 成人向けラベル付き投稿の添付は、表示設定(既定 OFF)を有効化する
// まで `blob_media_payload` がバイト列を返さない(fail-closed)。ローカル blob store に
// バイト列が存在していても読み出さないことを確認する = ネットワーク取得・デコードの
// 前段で遮断される。OFF へ戻すと以後の取得も再び止まる。
#[tokio::test]
async fn adult_labeled_media_payload_is_blocked_until_display_enabled() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
    let app = AppService::new(store.clone(), transport);
    let topic = "kukuri:topic:adult-gate";

    let object_id = app
        .create_post_with_attachments_in_channel(
            topic,
            kukuri_core::ChannelRef::Public,
            "labeled caption",
            None,
            vec![PendingAttachment {
                mime: "image/png".into(),
                bytes: b"adult-labeled-image".to_vec(),
                role: AssetRole::ImageOriginal,
            }],
            vec![kukuri_core::ADULT_CONTENT_LABEL.to_string()],
        )
        .await
        .expect("create labeled post");

    let timeline = app.list_timeline(topic, None, 10).await.expect("timeline");
    let post = timeline
        .items
        .iter()
        .find(|post| post.object_id == object_id)
        .expect("labeled post");
    assert_eq!(post.content_labels, vec!["adult".to_string()]);
    let attachment_hash = post.attachments[0].hash.clone();

    // 逆引き記録: 投稿の projection 書き込みで hash がゲート対象になっている。
    assert!(
        ObjectProjectionStore::is_adult_media_hash(
            store.as_ref(),
            &kukuri_core::BlobHash::new(attachment_hash.clone())
        )
        .await
        .expect("is_adult_media_hash")
    );

    // 既定 OFF: バイト列はローカルにあっても返さない。
    assert!(!app.adult_content_display_enabled());
    assert!(
        app.blob_media_payload(attachment_hash.as_str(), "image/png")
            .await
            .expect("gated payload result")
            .is_none()
    );

    // 明示的に有効化した場合だけ返す。
    app.set_adult_content_display_enabled(true);
    let payload = app
        .blob_media_payload(attachment_hash.as_str(), "image/png")
        .await
        .expect("enabled payload result")
        .expect("payload present after enabling");
    assert_eq!(payload.mime, "image/png");

    // OFF へ戻すと以後の取得は再び止まる。
    app.set_adult_content_display_enabled(false);
    assert!(
        app.blob_media_payload(attachment_hash.as_str(), "image/png")
            .await
            .expect("re-disabled payload result")
            .is_none()
    );
}

// #858: ラベルなし添付は表示設定 OFF でも従来どおり取得できる(fail-open)。
#[tokio::test]
async fn unlabeled_media_payload_is_unaffected_by_adult_display_setting() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
    let app = AppService::new(store, transport);
    let topic = "kukuri:topic:unlabeled-gate";

    let object_id = app
        .create_post_with_attachments(
            topic,
            "plain caption",
            None,
            vec![PendingAttachment {
                mime: "image/png".into(),
                bytes: b"plain-image".to_vec(),
                role: AssetRole::ImageOriginal,
            }],
        )
        .await
        .expect("create unlabeled post");
    let timeline = app.list_timeline(topic, None, 10).await.expect("timeline");
    let post = timeline
        .items
        .iter()
        .find(|post| post.object_id == object_id)
        .expect("unlabeled post");
    assert!(post.content_labels.is_empty());

    assert!(!app.adult_content_display_enabled());
    assert!(
        app.blob_media_payload(post.attachments[0].hash.as_str(), "image/png")
            .await
            .expect("payload result")
            .is_some()
    );
}

// #858: self-label は既知値(`adult`)だけを受け付ける。
#[tokio::test]
async fn create_post_rejects_unknown_content_labels() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
    let app = AppService::new(store, transport);

    let error = app
        .create_post_with_attachments_in_channel(
            "kukuri:topic:bad-label",
            kukuri_core::ChannelRef::Public,
            "body",
            None,
            Vec::new(),
            vec!["nsfw-unknown".to_string()],
        )
        .await
        .expect_err("unknown label must be rejected");
    assert!(error.to_string().contains("unknown content label"));
}
