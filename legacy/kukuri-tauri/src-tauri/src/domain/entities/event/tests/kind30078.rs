use crate::domain::entities::event::{Event, KIND30078_KIND, KIND30078_MAX_ATTACHMENTS};
use crate::shared::validation::ValidationFailureKind;
use chrono::Utc;
use serde_json::json;
use sha2::{Digest, Sha256};

fn build_kind30078_event(
    pubkey: String,
    tags: Vec<Vec<String>>,
    content: serde_json::Value,
) -> Event {
    let created_at = Utc::now();
    let kind = KIND30078_KIND;
    let content_str = content.to_string();
    let tags_for_event = tags.clone();
    let id_payload = json!([0, pubkey, created_at.timestamp(), kind, tags, content_str]);
    let serialized = serde_json::to_vec(&id_payload).expect("serialize kind30078 event");
    let id = format!("{:x}", Sha256::digest(&serialized));
    Event {
        id,
        pubkey,
        created_at,
        kind,
        tags: tags_for_event,
        content: content_str,
        sig: "f".repeat(128),
    }
}

fn base_kind30078_tags(pubkey: &str, slug: &str, revision: &str) -> Vec<Vec<String>> {
    vec![
        vec![
            "d".into(),
            format!("kukuri:topic:{}:post:{}", slug, revision),
        ],
        vec!["k".into(), "topic-post".into()],
        vec!["t".into(), format!("topic:{}", slug)],
        vec![
            "a".into(),
            format!("30078:{}:kukuri:topic:{}:post:{}", pubkey, slug, revision),
        ],
    ]
}

#[test]
fn test_validate_kind30078_ok() {
    let slug = "sample-topic";
    let revision = "A".repeat(26);
    let pubkey = "f".repeat(64);
    let tags = base_kind30078_tags(&pubkey, slug, &revision);
    let content = json!({
        "body": "hello",
        "attachments": ["iroh://attachment"],
        "metadata": {"app_version": "1.0.0", "edited": false}
    });
    let event = build_kind30078_event(pubkey.clone(), tags, content);
    assert!(event.validate_kind30078().is_ok());
}

#[test]
fn test_validate_kind30078_missing_d_tag() {
    let mut tags = base_kind30078_tags(&"f".repeat(64), "slug", &"A".repeat(26));
    tags.retain(|tag| tag.first().map(|s| s != "d").unwrap_or(true));
    let content = json!({
        "body": "hello",
        "attachments": [],
        "metadata": {"app_version": "1.0.0", "edited": false}
    });
    let event = build_kind30078_event("f".repeat(64), tags, content);
    let err = event.validate_kind30078().unwrap_err();
    assert_eq!(err.kind, ValidationFailureKind::Kind30078TagMissing);
}

#[test]
fn test_validate_kind30078_invalid_t_value() {
    let slug = "sample-topic";
    let revision = "A".repeat(26);
    let pubkey = "f".repeat(64);
    let mut tags = base_kind30078_tags(&pubkey, slug, &revision);
    if let Some(t_tag) = tags
        .iter_mut()
        .find(|tag| tag.first().map(|s| s == "t").unwrap_or(false))
    {
        t_tag[1] = "topic:wrong".into();
    }
    let content = json!({
        "body": "hello",
        "attachments": [],
        "metadata": {"app_version": "1.0.0", "edited": false}
    });
    let event = build_kind30078_event(pubkey.clone(), tags, content);
    let err = event.validate_kind30078().unwrap_err();
    assert_eq!(err.kind, ValidationFailureKind::Kind30078TagMismatch);
}

#[test]
fn test_validate_kind30078_invalid_attachment() {
    let slug = "sample-topic";
    let revision = "A".repeat(26);
    let pubkey = "f".repeat(64);
    let tags = base_kind30078_tags(&pubkey, slug, &revision);
    let content = json!({
        "body": "hello",
        "attachments": ["invalid://attachment"],
        "metadata": {"app_version": "1.0.0", "edited": false}
    });
    let event = build_kind30078_event(pubkey.clone(), tags, content);
    let err = event.validate_kind30078().unwrap_err();
    assert_eq!(err.kind, ValidationFailureKind::Kind30078ContentSchema);
}

#[test]
fn test_validate_kind30078_missing_a_tag() {
    let slug = "sample-topic";
    let revision = "A".repeat(26);
    let pubkey = "f".repeat(64);
    let mut tags = base_kind30078_tags(&pubkey, slug, &revision);
    tags.retain(|tag| tag.first().map(|s| s != "a").unwrap_or(true));
    let content = json!({
        "body": "hello",
        "attachments": [],
        "metadata": {"app_version": "1.0.0", "edited": false}
    });
    let event = build_kind30078_event(pubkey.clone(), tags, content);
    let err = event.validate_kind30078().unwrap_err();
    assert_eq!(err.kind, ValidationFailureKind::Kind30078TagMissing);
}

#[test]
fn test_validate_kind30078_attachment_overflow() {
    let slug = "sample-topic";
    let revision = "A".repeat(26);
    let pubkey = "f".repeat(64);
    let tags = base_kind30078_tags(&pubkey, slug, &revision);
    let attachments: Vec<String> = (0..=KIND30078_MAX_ATTACHMENTS)
        .map(|i| format!("iroh://attachment/{i}"))
        .collect();
    let content = json!({
        "body": "hello",
        "attachments": attachments,
        "metadata": {"app_version": "1.0.0", "edited": false}
    });
    let event = build_kind30078_event(pubkey.clone(), tags, content);
    let err = event.validate_kind30078().unwrap_err();
    assert_eq!(err.kind, ValidationFailureKind::Kind30078ContentSize);
}

#[test]
fn test_validate_kind30078_invalid_semver() {
    let slug = "sample-topic";
    let revision = "A".repeat(26);
    let pubkey = "f".repeat(64);
    let tags = base_kind30078_tags(&pubkey, slug, &revision);
    let content = json!({
        "body": "hello",
        "attachments": [],
        "metadata": {"app_version": "not-a-version", "edited": false}
    });
    let event = build_kind30078_event(pubkey.clone(), tags, content);
    let err = event.validate_kind30078().unwrap_err();
    assert_eq!(err.kind, ValidationFailureKind::Kind30078ContentSchema);
}
