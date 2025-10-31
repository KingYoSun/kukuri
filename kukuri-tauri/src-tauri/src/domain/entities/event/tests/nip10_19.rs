use crate::domain::entities::event::Event;
use crate::shared::validation::ValidationFailureKind;
use bech32::{ToBase32 as _, Variant};
use nostr_sdk::prelude::*;

fn dummy_event_with_tags(tags: Vec<Vec<String>>) -> Event {
    Event {
        id: "0".repeat(64),
        pubkey: "f".repeat(64),
        created_at: chrono::Utc::now(),
        kind: 1,
        tags,
        content: String::new(),
        sig: "f".repeat(128),
    }
}

#[test]
fn test_validate_nip10_19_ok_with_bech32_refs() {
    let keys = Keys::generate();
    let npub = keys.public_key().to_bech32().unwrap();

    let nostr_ev = EventBuilder::text_note("x").sign_with_keys(&keys).unwrap();
    let note = nostr_ev.id.to_bech32().unwrap();

    let e_root = vec!["e".into(), note.clone(), String::new(), "root".into()];
    let e_reply = vec!["e".into(), note, String::new(), "reply".into()];
    let p_tag = vec!["p".into(), npub];
    let ev = dummy_event_with_tags(vec![e_root, e_reply, p_tag]);
    assert!(ev.validate_nip10_19().is_ok());
}

#[test]
fn test_validate_nip10_19_rejects_invalid_marker_and_pk() {
    let e_tag = vec!["e".into(), "0".repeat(64), String::new(), "bad".into()];
    let p_tag = vec!["p".into(), "zzz".into()];
    let ev = dummy_event_with_tags(vec![e_tag, p_tag]);
    let err = ev.validate_nip10_19().unwrap_err();
    assert_eq!(err.kind, ValidationFailureKind::Nip10TagStructure);
}

#[test]
fn test_validate_nip10_reply_without_root_ok() {
    let e_tag_reply = vec!["e".into(), "0".repeat(64), String::new(), "reply".into()];
    let ev = dummy_event_with_tags(vec![e_tag_reply]);
    assert!(ev.validate_nip10_19().is_ok());
}

#[test]
fn test_nprofile_tlv_multiple_relays_ok() {
    let keys = Keys::generate();
    let mut bytes = Vec::new();
    bytes.push(0);
    bytes.push(32);
    bytes.extend_from_slice(&keys.public_key().to_bytes());
    for relay in ["wss://relay.one", "wss://relay.two"] {
        let relay_bytes = relay.as_bytes();
        bytes.push(1);
        bytes.push(relay_bytes.len() as u8);
        bytes.extend_from_slice(relay_bytes);
    }
    let encoded = bech32::encode("nprofile", bytes.to_base32(), Variant::Bech32).expect("encode");
    assert!(Event::validate_nprofile_tlv(&encoded).is_ok());
}

#[test]
fn test_nprofile_tlv_rejects_invalid_relay_scheme() {
    let keys = Keys::generate();
    let mut bytes = Vec::new();
    bytes.push(0);
    bytes.push(32);
    bytes.extend_from_slice(&keys.public_key().to_bytes());
    let relay_bytes = b"https://relay.invalid";
    bytes.push(1);
    bytes.push(relay_bytes.len() as u8);
    bytes.extend_from_slice(relay_bytes);
    let encoded = bech32::encode("nprofile", bytes.to_base32(), Variant::Bech32).expect("encode");
    assert!(Event::validate_nprofile_tlv(&encoded).is_err());
}

#[test]
fn test_nevent_tlv_with_optional_author_and_kind() {
    let keys = Keys::generate();
    let nostr_ev = EventBuilder::text_note("tlv")
        .sign_with_keys(&keys)
        .expect("sign");
    let mut bytes = Vec::new();
    bytes.push(0);
    bytes.push(32);
    bytes.extend_from_slice(&nostr_ev.id.to_bytes());
    let relay_bytes = b"wss://relay.example";
    bytes.push(1);
    bytes.push(relay_bytes.len() as u8);
    bytes.extend_from_slice(relay_bytes);
    bytes.push(2);
    bytes.push(32);
    bytes.extend_from_slice(&nostr_ev.pubkey.to_bytes());
    let kind_bytes = (nostr_ev.kind.as_u16() as u32).to_be_bytes();
    bytes.push(3);
    bytes.push(kind_bytes.len() as u8);
    bytes.extend_from_slice(&kind_bytes);
    let encoded = bech32::encode("nevent", bytes.to_base32(), Variant::Bech32).unwrap();
    assert!(Event::validate_nevent_tlv(&encoded).is_ok());
}

#[test]
fn test_nevent_tlv_rejects_invalid_author_length() {
    let mut bytes = Vec::new();
    bytes.push(0);
    bytes.push(32);
    bytes.extend_from_slice(&[0u8; 32]);
    bytes.push(2);
    bytes.push(31);
    bytes.extend_from_slice(&[0u8; 31]);
    let encoded = bech32::encode("nevent", bytes.to_base32(), Variant::Bech32).unwrap();
    assert!(Event::validate_nevent_tlv(&encoded).is_err());
}
