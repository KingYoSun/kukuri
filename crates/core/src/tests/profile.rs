use crate::*;

#[test]
fn profile_envelope_roundtrip() {
    let keys = generate_keys();
    let envelope = build_profile_envelope(
        &keys,
        &KukuriProfileEnvelopeContentV1 {
            author_pubkey: keys.public_key(),
            name: Some("alice".into()),
            display_name: Some("Alice".into()),
            about: Some("hello".into()),
            picture: Some("https://example.com/alice.png".into()),
            picture_asset: Some(AssetRef {
                hash: BlobHash::new("avatar-hash"),
                mime: "image/png".into(),
                bytes: 42,
                role: AssetRole::ProfileAvatar,
            }),
        },
    )
    .expect("profile envelope");

    envelope.verify().expect("signature verification");
    let profile = parse_profile(&envelope)
        .expect("parse profile")
        .expect("profile");
    assert_eq!(profile.pubkey, keys.public_key());
    assert_eq!(profile.display_name.as_deref(), Some("Alice"));
    assert_eq!(profile.about.as_deref(), Some("hello"));
    assert_eq!(
        profile
            .picture_asset
            .as_ref()
            .map(|asset| asset.role.clone()),
        Some(AssetRole::ProfileAvatar)
    );
}

#[test]
fn profile_post_envelope_roundtrip() {
    let keys = generate_keys();
    let author_pubkey = keys.public_key();
    let envelope = build_profile_post_envelope(
        &keys,
        &KukuriProfilePostEnvelopeContentV1 {
            author_pubkey: author_pubkey.clone(),
            profile_topic_id: author_profile_topic_id(author_pubkey.as_str()),
            published_topic_id: TopicId::new("kukuri:topic:demo"),
            object_id: EnvelopeId::from("post-1"),
            created_at: 42,
            object_kind: "comment".into(),
            content: "hello profile topic".into(),
            attachments: vec![AssetRef {
                hash: BlobHash::new("hash-1"),
                mime: "image/png".into(),
                bytes: 12,
                role: AssetRole::ImageOriginal,
            }],
            reply_to_object_id: Some(EnvelopeId::from("root-1")),
            root_id: Some(EnvelopeId::from("root-1")),
        },
    )
    .expect("profile post envelope");

    envelope.verify().expect("signature verification");
    let profile_post = parse_profile_post(&envelope)
        .expect("parse profile post")
        .expect("profile post");
    assert_eq!(profile_post.author_pubkey, author_pubkey);
    assert_eq!(
        profile_post.profile_topic_id,
        author_profile_topic_id(author_pubkey.as_str())
    );
    assert_eq!(
        profile_post.published_topic_id.as_str(),
        "kukuri:topic:demo"
    );
    assert_eq!(profile_post.object_id.as_str(), "post-1");
    assert_eq!(profile_post.created_at, 42);
    assert_eq!(profile_post.object_kind, "comment");
    assert_eq!(profile_post.content, "hello profile topic");
    assert_eq!(profile_post.attachments.len(), 1);
    assert_eq!(
        profile_post
            .reply_to_object_id
            .as_ref()
            .map(EnvelopeId::as_str),
        Some("root-1")
    );
    assert_eq!(
        profile_post.root_id.as_ref().map(EnvelopeId::as_str),
        Some("root-1")
    );
    assert_eq!(profile_post.envelope_id, envelope.id);
}

#[test]
fn profile_repost_envelope_roundtrip() {
    let keys = generate_keys();
    let author_pubkey = keys.public_key();
    let envelope = build_profile_repost_envelope(
        &keys,
        &KukuriProfileRepostEnvelopeContentV1 {
            author_pubkey: author_pubkey.clone(),
            profile_topic_id: author_profile_topic_id(author_pubkey.as_str()),
            published_topic_id: TopicId::new("kukuri:topic:target"),
            object_id: EnvelopeId::from("repost-1"),
            created_at: 55,
            commentary: Some("quote commentary".into()),
            repost_of: RepostSourceSnapshotV1 {
                source_object_id: EnvelopeId::from("source-1"),
                source_topic_id: TopicId::new("kukuri:topic:source"),
                source_author_pubkey: generate_keys().public_key(),
                source_object_kind: "post".into(),
                content: "source content".into(),
                attachments: Vec::new(),
                reply_to_object_id: None,
                root_id: Some(EnvelopeId::from("source-1")),
            },
        },
    )
    .expect("profile repost envelope");

    envelope.verify().expect("signature verification");
    let profile_repost = parse_profile_repost(&envelope)
        .expect("parse profile repost")
        .expect("profile repost");
    assert_eq!(profile_repost.author_pubkey, author_pubkey);
    assert_eq!(
        profile_repost.published_topic_id.as_str(),
        "kukuri:topic:target"
    );
    assert_eq!(profile_repost.object_id.as_str(), "repost-1");
    assert_eq!(
        profile_repost.commentary.as_deref(),
        Some("quote commentary")
    );
    assert_eq!(
        profile_repost.repost_of.source_topic_id.as_str(),
        "kukuri:topic:source"
    );
}

#[test]
fn follow_edge_roundtrip_and_self_follow_rejected() {
    let keys = generate_keys();
    let target = generate_keys().public_key();
    let envelope =
        build_follow_edge_envelope(&keys, &target, FollowEdgeStatus::Active).expect("envelope");

    envelope.verify().expect("signature verification");
    let edge = parse_follow_edge(&envelope)
        .expect("parse follow edge")
        .expect("follow edge");
    assert_eq!(edge.subject_pubkey, keys.public_key());
    assert_eq!(edge.target_pubkey, target);
    assert_eq!(edge.status, FollowEdgeStatus::Active);

    let self_follow_error =
        build_follow_edge_envelope(&keys, &keys.public_key(), FollowEdgeStatus::Active)
            .expect_err("self follow should be rejected");
    assert!(self_follow_error.to_string().contains("self follow"));
}

#[test]
fn follow_edge_parser_rejects_subject_mismatch() {
    let signer = generate_keys();
    let subject = generate_keys().public_key();
    let target = generate_keys().public_key();
    let envelope = sign_envelope_json(
        &signer,
        "follow-edge",
        vec![vec!["object".into(), "follow-edge".into()]],
        &KukuriFollowEdgeEnvelopeContentV1 {
            subject_pubkey: subject,
            target_pubkey: target,
            status: FollowEdgeStatus::Active,
        },
    )
    .expect("envelope");

    let error = parse_follow_edge(&envelope).expect_err("subject mismatch must fail");
    assert!(error.to_string().contains("subject pubkey must match"));
}
