use crate::*;

#[test]
fn repost_envelope_roundtrip() {
    let keys = generate_keys();
    let envelope = build_repost_envelope(
        &keys,
        &TopicId::new("kukuri:topic:target"),
        RepostSourceSnapshotV1 {
            source_object_id: EnvelopeId::from("source-1"),
            source_topic_id: TopicId::new("kukuri:topic:source"),
            source_author_pubkey: generate_keys().public_key(),
            source_object_kind: "comment".into(),
            content: "quoted source".into(),
            attachments: vec![AssetRef {
                hash: BlobHash::new("hash-1"),
                mime: "image/png".into(),
                bytes: 24,
                role: AssetRole::ImageOriginal,
            }],
            reply_to_object_id: Some(EnvelopeId::from("root-1")),
            root_id: Some(EnvelopeId::from("root-1")),
        },
        Some("quote commentary"),
    )
    .expect("repost envelope");

    envelope.verify().expect("signature verification");
    let repost = envelope
        .to_post_object()
        .expect("parse repost")
        .expect("repost object");
    assert_eq!(repost.object_kind, "repost");
    assert_eq!(repost.topic_id.as_str(), "kukuri:topic:target");
    assert_eq!(
        repost
            .repost_of
            .as_ref()
            .map(|value| value.source_topic_id.as_str()),
        Some("kukuri:topic:source")
    );
    assert_eq!(
        match repost.payload_ref {
            PayloadRef::InlineText { text } => text,
            PayloadRef::BlobText { .. } => String::new(),
        },
        "quote commentary"
    );
}
