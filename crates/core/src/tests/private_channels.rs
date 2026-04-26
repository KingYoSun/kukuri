use crate::*;

#[test]
fn friend_only_grant_roundtrip_and_expiry_reject() {
    let keys = generate_keys();
    let token = build_friend_only_grant_token(
        &keys,
        &TopicId::new("kukuri:topic:friends"),
        &ChannelId::new("channel-1"),
        "friends",
        "epoch-1",
        &generate_keys().export_secret_hex(),
        None,
    )
    .expect("friend-only grant");

    let preview = parse_friend_only_grant_token(token.as_str()).expect("parse friend-only grant");
    assert_eq!(preview.owner_pubkey, keys.public_key());
    assert_eq!(preview.epoch_id, "epoch-1");

    let expired = build_friend_only_grant_token(
        &keys,
        &TopicId::new("kukuri:topic:friends"),
        &ChannelId::new("channel-1"),
        "friends",
        "epoch-1",
        &generate_keys().export_secret_hex(),
        Some(1),
    )
    .expect("expired grant");
    let error = parse_friend_only_grant_token(expired.as_str()).expect_err("expired grant");
    assert!(error.to_string().contains("expired"));
}

#[test]
fn friend_only_grant_parser_rejects_signer_mismatch() {
    let signer = generate_keys();
    let other = generate_keys();
    let token = FriendOnlyGrantTokenV1 {
        envelope: sign_envelope_json(
            &signer,
            "channel-friend-grant",
            vec![vec!["object".into(), "channel-friend-grant".into()]],
            &KukuriFriendOnlyGrantEnvelopeContentV1 {
                channel_id: ChannelId::new("channel-1"),
                topic_id: TopicId::new("kukuri:topic:friends"),
                channel_label: "friends".into(),
                owner_pubkey: other.public_key(),
                epoch_id: "epoch-1".into(),
                namespace_secret_hex: generate_keys().export_secret_hex(),
                expires_at: None,
            },
        )
        .expect("grant envelope"),
    };
    let encoded = serde_json::to_string(&token).expect("encode token");
    let error =
        parse_friend_only_grant_token(encoded.as_str()).expect_err("owner mismatch must fail");
    assert!(error.to_string().contains("owner pubkey must match"));
}

#[test]
fn channel_policy_and_participant_roundtrip() {
    let owner = generate_keys();
    let participant = generate_keys();
    let policy = PrivateChannelPolicyDocV1 {
        channel_id: ChannelId::new("channel-1"),
        topic_id: TopicId::new("kukuri:topic:friends"),
        audience_kind: ChannelAudienceKind::FriendOnly,
        owner_pubkey: owner.public_key(),
        epoch_id: "epoch-1".into(),
        sharing_state: ChannelSharingState::Open,
        rotated_at: None,
        previous_epoch_id: None,
    };
    let policy_envelope =
        build_private_channel_policy_envelope(&owner, &policy).expect("policy envelope");
    let parsed_policy = parse_private_channel_policy(&policy_envelope)
        .expect("parse policy")
        .expect("policy");
    assert_eq!(parsed_policy.audience_kind, ChannelAudienceKind::FriendOnly);

    let participant_doc = PrivateChannelParticipantDocV1 {
        channel_id: ChannelId::new("channel-1"),
        topic_id: TopicId::new("kukuri:topic:friends"),
        epoch_id: "epoch-1".into(),
        participant_pubkey: participant.public_key(),
        joined_at: 10,
        is_owner: false,
        join_mode: Some(PrivateChannelJoinMode::FriendOnlyGrant),
        sponsor_pubkey: Some(owner.public_key()),
        share_token_id: None,
        left_at: None,
    };
    let participant_envelope =
        build_private_channel_participant_envelope(&participant, &participant_doc)
            .expect("participant envelope");
    let parsed_participant = parse_private_channel_participant(&participant_envelope)
        .expect("parse participant")
        .expect("participant");
    assert_eq!(
        parsed_participant.participant_pubkey,
        participant.public_key()
    );
}

#[test]
fn friend_plus_share_roundtrip_and_expiry_reject() {
    let owner = generate_keys();
    let sponsor = generate_keys();
    let token = build_friend_plus_share_token(
        &sponsor,
        &TopicId::new("kukuri:topic:friends-plus"),
        &ChannelId::new("channel-1"),
        "friends+",
        &owner.public_key(),
        "epoch-1",
        &generate_keys().export_secret_hex(),
        None,
    )
    .expect("friend-plus share");

    let preview = parse_friend_plus_share_token(token.as_str()).expect("parse friend-plus share");
    assert_eq!(preview.owner_pubkey, owner.public_key());
    assert_eq!(preview.sponsor_pubkey, sponsor.public_key());
    assert_eq!(preview.epoch_id, "epoch-1");
    assert_eq!(preview.share_token_id.len(), 64);

    let expired = build_friend_plus_share_token(
        &sponsor,
        &TopicId::new("kukuri:topic:friends-plus"),
        &ChannelId::new("channel-1"),
        "friends+",
        &owner.public_key(),
        "epoch-1",
        &generate_keys().export_secret_hex(),
        Some(1),
    )
    .expect("expired friend-plus share");
    let error = parse_friend_plus_share_token(expired.as_str()).expect_err("expired share");
    assert!(error.to_string().contains("expired"));
}

#[test]
fn friend_plus_share_parser_rejects_signer_mismatch() {
    let owner = generate_keys();
    let signer = generate_keys();
    let sponsor = generate_keys();
    let token = FriendPlusShareTokenV1 {
        envelope: sign_envelope_json(
            &signer,
            "channel-share",
            vec![vec!["object".into(), "channel-share".into()]],
            &KukuriFriendPlusShareEnvelopeContentV1 {
                channel_id: ChannelId::new("channel-1"),
                topic_id: TopicId::new("kukuri:topic:friends-plus"),
                channel_label: "friends+".into(),
                owner_pubkey: owner.public_key(),
                sponsor_pubkey: sponsor.public_key(),
                epoch_id: "epoch-1".into(),
                namespace_secret_hex: generate_keys().export_secret_hex(),
                expires_at: None,
            },
        )
        .expect("share envelope"),
    };
    let encoded = serde_json::to_string(&token).expect("encode share");
    let error =
        parse_friend_plus_share_token(encoded.as_str()).expect_err("sponsor mismatch must fail");
    assert!(error.to_string().contains("sponsor pubkey must match"));
}

#[test]
fn channel_rotation_grant_encrypt_decrypt_roundtrip_and_wrong_recipient_fails() {
    let owner = generate_keys();
    let recipient = generate_keys();
    let wrong_recipient = generate_keys();
    let payload = PrivateChannelRotationGrantPayloadV1 {
        channel_id: ChannelId::new("channel-1"),
        topic_id: TopicId::new("kukuri:topic:friends-plus"),
        owner_pubkey: owner.public_key(),
        recipient_pubkey: recipient.public_key(),
        old_epoch_id: "epoch-1".into(),
        new_epoch_id: "epoch-2".into(),
        new_namespace_secret_hex: generate_keys().export_secret_hex(),
    };
    let doc =
        encrypt_private_channel_rotation_grant(&owner, &payload).expect("encrypt rotation grant");
    let envelope = build_private_channel_rotation_grant_envelope(&owner, &doc).expect("envelope");
    let parsed_doc = parse_private_channel_rotation_grant(&envelope)
        .expect("parse rotation grant")
        .expect("rotation grant");
    let decrypted = decrypt_private_channel_rotation_grant(&recipient, &parsed_doc)
        .expect("decrypt rotation grant");
    assert_eq!(decrypted.new_epoch_id, "epoch-2");
    assert_eq!(decrypted.recipient_pubkey, recipient.public_key());

    let error = decrypt_private_channel_rotation_grant(&wrong_recipient, &parsed_doc)
        .expect_err("wrong recipient must fail");
    assert!(error.to_string().contains("recipient pubkey"));
}
