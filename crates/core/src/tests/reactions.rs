use crate::*;

#[test]
fn reaction_envelope_roundtrip_for_emoji() {
    let keys = generate_keys();
    let topic = TopicId::new("kukuri:topic:demo");
    let target_object_id = EnvelopeId::from("post-1");
    let reaction_id = deterministic_reaction_id(
        &ReplicaId::new("replica-1"),
        &target_object_id,
        &keys.public_key(),
        "emoji:👍",
    );
    let envelope = build_reaction_envelope(
        &keys,
        &topic,
        None,
        &target_object_id,
        ReactionKeyV1::Emoji {
            emoji: " 👍 ".into(),
        },
        &reaction_id,
        ObjectStatus::Active,
    )
    .expect("reaction envelope");

    envelope.verify().expect("signature verification");
    let reaction = parse_reaction(&envelope)
        .expect("parse reaction")
        .expect("reaction");
    assert_eq!(reaction.reaction_id, reaction_id);
    assert_eq!(reaction.target_topic_id, topic);
    assert_eq!(reaction.target_object_id, target_object_id);
    assert_eq!(reaction.reaction_key_kind, ReactionKeyKind::Emoji);
    assert_eq!(reaction.emoji.as_deref(), Some("👍"));
    assert_eq!(reaction.normalized_reaction_key, "emoji:👍");
    assert_eq!(reaction.status, ObjectStatus::Active);
}

#[test]
fn custom_reaction_asset_roundtrip_and_reaction_id_stability() {
    let keys = generate_keys();
    let envelope = build_custom_reaction_asset_envelope(
        &keys,
        BlobHash::new("blob-asset-1"),
        "party".into(),
        "image/png".into(),
        128,
        128,
        128,
    )
    .expect("asset envelope");

    envelope.verify().expect("signature verification");
    let asset = parse_custom_reaction_asset(&envelope)
        .expect("parse asset")
        .expect("asset");
    assert_eq!(asset.asset_id, envelope.id.0);
    assert_eq!(asset.mime, "image/png");

    let reaction_key = ReactionKeyV1::CustomAsset {
        asset_id: asset.asset_id.clone(),
        snapshot: CustomReactionAssetSnapshotV1 {
            asset_id: asset.asset_id.clone(),
            owner_pubkey: asset.author_pubkey.clone(),
            blob_hash: asset.blob_hash.clone(),
            search_key: asset.search_key.clone(),
            mime: asset.mime.clone(),
            bytes: asset.bytes,
            width: asset.width,
            height: asset.height,
        },
    };
    let normalized = reaction_key.normalized_key().expect("normalized key");
    let first = deterministic_reaction_id(
        &ReplicaId::new("replica-a"),
        &EnvelopeId::from("post-1"),
        &keys.public_key(),
        normalized.as_str(),
    );
    let second = deterministic_reaction_id(
        &ReplicaId::new("replica-a"),
        &EnvelopeId::from("post-1"),
        &keys.public_key(),
        normalized.as_str(),
    );
    let different = deterministic_reaction_id(
        &ReplicaId::new("replica-a"),
        &EnvelopeId::from("post-1"),
        &keys.public_key(),
        "emoji:🔥",
    );
    assert_eq!(first, second);
    assert_ne!(first, different);
}
