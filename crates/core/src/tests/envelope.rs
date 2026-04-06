use crate::*;

#[test]
fn signed_envelope_roundtrip_json() {
    let keys = generate_keys();
    let topic = TopicId::new("kukuri:topic:contract");
    let envelope = build_post_envelope(&keys, &topic, "hello", None).expect("envelope");
    let json = serde_json::to_string(&envelope).expect("serialize");
    let restored: KukuriEnvelope = serde_json::from_str(&json).expect("deserialize");

    restored.verify().expect("signature verification");
    assert_eq!(restored.id, envelope.id);
    assert_eq!(restored.topic_id(), Some(topic));
}

#[test]
fn comment_envelope_tracks_root_and_reply() {
    let keys = generate_keys();
    let root = build_post_envelope(&keys, &TopicId::new("kukuri:topic:thread"), "root", None)
        .expect("root");
    let reply = build_post_envelope(
        &keys,
        &TopicId::new("kukuri:topic:thread"),
        "reply",
        Some(&root),
    )
    .expect("reply");

    reply.verify().expect("signature verification");

    let thread = reply.thread_ref().expect("thread ref");
    assert_eq!(thread.root, root.id);
    assert_eq!(thread.reply_to, Some(root.id));
    assert_eq!(reply.kind, "comment");
}

#[test]
fn mutation_breaks_signature_verification() {
    let keys = generate_keys();
    let mut envelope =
        build_post_envelope(&keys, &TopicId::new("kukuri:topic:wire"), "display", None)
            .expect("envelope");
    envelope.content = "mutated".to_string();

    let error = envelope.verify().expect_err("verification should fail");
    assert!(error.to_string().contains("mismatch") || error.to_string().contains("failed"));
}
