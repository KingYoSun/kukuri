use crate::*;

#[test]
fn media_manifest_envelope_uses_protocol_object_kind() {
    let keys = generate_keys();
    let envelope = build_media_manifest_envelope(
        &keys,
        &TopicId::new("kukuri:topic:media"),
        &KukuriMediaManifestV1 {
            manifest_id: "manifest-1".into(),
            owner_pubkey: keys.public_key(),
            created_at: 1,
            items: vec![MediaManifestItem {
                blob_hash: BlobHash::new("blob-1"),
                mime: "image/png".into(),
                size: 123,
                width: Some(10),
                height: Some(10),
                duration_ms: None,
                codec: None,
                thumbnail_blob_hash: None,
            }],
        },
    )
    .expect("manifest envelope");

    envelope.verify().expect("signature verification");
    assert_eq!(envelope.kind, "media-manifest");
}
