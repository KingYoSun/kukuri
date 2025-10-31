use crate::domain::entities::event::Event;
use crate::shared::validation::ValidationFailureKind;
use chrono::Duration;
use nostr_sdk::prelude::*;
use serde_json::json;
use sha2::{Digest, Sha256};

#[tokio::test]
async fn test_validate_nip01_ok() {
    let keys = Keys::generate();
    let nostr_ev = EventBuilder::text_note("hello nip01")
        .sign_with_keys(&keys)
        .unwrap();

    let created_at =
        chrono::DateTime::<chrono::Utc>::from_timestamp(nostr_ev.created_at.as_u64() as i64, 0)
            .unwrap();

    let dom = Event {
        id: nostr_ev.id.to_string(),
        pubkey: nostr_ev.pubkey.to_string(),
        created_at,
        kind: nostr_ev.kind.as_u16() as u32,
        tags: nostr_ev.tags.iter().map(|t| t.clone().to_vec()).collect(),
        content: nostr_ev.content.clone(),
        sig: nostr_ev.sig.to_string(),
    };

    assert!(dom.validate_nip01().is_ok());
}

#[tokio::test]
async fn test_validate_nip01_bad_id() {
    let keys = Keys::generate();
    let nostr_ev = EventBuilder::text_note("oops")
        .sign_with_keys(&keys)
        .unwrap();

    let created_at =
        chrono::DateTime::<chrono::Utc>::from_timestamp(nostr_ev.created_at.as_u64() as i64, 0)
            .unwrap();

    let mut dom = Event {
        id: nostr_ev.id.to_string(),
        pubkey: nostr_ev.pubkey.to_string(),
        created_at,
        kind: nostr_ev.kind.as_u16() as u32,
        tags: nostr_ev.tags.iter().map(|t| t.clone().to_vec()).collect(),
        content: nostr_ev.content.clone(),
        sig: nostr_ev.sig.to_string(),
    };
    dom.content = "tampered".into();
    let err = dom.validate_nip01().unwrap_err();
    assert_eq!(err.kind, ValidationFailureKind::Nip01Integrity);
}

fn build_event_with_data(
    pubkey: &str,
    kind: u32,
    tags: Vec<Vec<String>>,
    content: &str,
    created_at: chrono::DateTime<chrono::Utc>,
) -> Event {
    let id_payload = json!([0, pubkey, created_at.timestamp(), kind, tags, content]);
    let serialized = serde_json::to_vec(&id_payload).expect("serialize event");
    let id = format!("{:x}", Sha256::digest(&serialized));
    Event {
        id,
        pubkey: pubkey.to_string(),
        created_at,
        kind,
        tags,
        content: content.to_string(),
        sig: "f".repeat(128),
    }
}

#[test]
fn test_validate_nip01_rejects_timestamp_drift() {
    let created_at = chrono::Utc::now() - Duration::hours(2);
    let event = build_event_with_data(&"f".repeat(64), 1, Vec::new(), "time drift", created_at);
    let err = event.validate_nip01().unwrap_err();
    assert_eq!(err.kind, ValidationFailureKind::TimestampOutOfRange);
}
