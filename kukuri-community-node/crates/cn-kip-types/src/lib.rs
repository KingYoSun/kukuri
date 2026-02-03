use anyhow::{anyhow, Result};
use cn_core::nostr::RawEvent;
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

pub const KIP_NAMESPACE: &str = "kukuri";
pub const KIP_VERSION: &str = "1";

pub const KIND_NODE_DESCRIPTOR: u32 = 39000;
pub const KIND_NODE_TOPIC_SERVICE: u32 = 39001;
pub const KIND_REPORT: u32 = 39005;
pub const KIND_LABEL: u32 = 39006;
pub const KIND_ATTESTATION: u32 = 39010;
pub const KIND_TRUST_ANCHOR: u32 = 39011;
pub const KIND_KEY_ENVELOPE: u32 = 39020;
pub const KIND_INVITE_CAPABILITY: u32 = 39021;
pub const KIND_JOIN_REQUEST: u32 = 39022;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KipKind {
    NodeDescriptor,
    NodeTopicService,
    Report,
    Label,
    Attestation,
    TrustAnchor,
    KeyEnvelope,
    InviteCapability,
    JoinRequest,
}

impl KipKind {
    pub fn from_kind(kind: u32) -> Option<Self> {
        match kind {
            KIND_NODE_DESCRIPTOR => Some(Self::NodeDescriptor),
            KIND_NODE_TOPIC_SERVICE => Some(Self::NodeTopicService),
            KIND_REPORT => Some(Self::Report),
            KIND_LABEL => Some(Self::Label),
            KIND_ATTESTATION => Some(Self::Attestation),
            KIND_TRUST_ANCHOR => Some(Self::TrustAnchor),
            KIND_KEY_ENVELOPE => Some(Self::KeyEnvelope),
            KIND_INVITE_CAPABILITY => Some(Self::InviteCapability),
            KIND_JOIN_REQUEST => Some(Self::JoinRequest),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ValidationOptions {
    pub now: i64,
    pub verify_signature: bool,
    pub require_k_tag: bool,
    pub require_ver_tag: bool,
}

impl Default for ValidationOptions {
    fn default() -> Self {
        Self {
            now: current_unix_seconds(),
            verify_signature: true,
            require_k_tag: true,
            require_ver_tag: true,
        }
    }
}

pub fn is_kip_kind(kind: u32) -> bool {
    KipKind::from_kind(kind).is_some()
}

pub fn validate_kip_event(raw: &RawEvent, options: ValidationOptions) -> Result<KipKind> {
    let kind = KipKind::from_kind(raw.kind)
        .ok_or_else(|| anyhow!("unsupported kind: {}", raw.kind))?;

    if options.verify_signature {
        cn_core::nostr::verify_event(raw)?;
    }

    if options.require_k_tag {
        let value = require_tag_value(raw, "k")?;
        if value != KIP_NAMESPACE {
            return Err(anyhow!("invalid k tag: {}", value));
        }
    }

    if options.require_ver_tag {
        let value = require_tag_value(raw, "ver")?;
        if value != KIP_VERSION {
            return Err(anyhow!("invalid ver tag: {}", value));
        }
    }

    match kind {
        KipKind::NodeDescriptor => {
            require_tag_value(raw, "d")?;
            require_exp_tag(raw, options.now)?;
            validate_schema(raw, "kukuri-node-desc-v1")?;
        }
        KipKind::NodeTopicService => {
            require_tag_value(raw, "d")?;
            require_tag_value(raw, "t")?;
            require_tag_value(raw, "role")?;
            let scope = require_tag_value(raw, "scope")?;
            validate_scope(&scope, true)?;
            require_exp_tag(raw, options.now)?;
            validate_schema(raw, "kukuri-topic-service-v1")?;
        }
        KipKind::Report => {
            require_tag_value(raw, "target")?;
            require_tag_value(raw, "reason")?;
        }
        KipKind::Label => {
            require_tag_value(raw, "target")?;
            require_tag_value(raw, "label")?;
            require_exp_tag(raw, options.now)?;
            if has_tag(raw, "policy_url") {
                require_tag_value(raw, "policy_url")?;
            } else if has_tag(raw, "policy") {
                require_tag_value(raw, "policy")?;
            } else {
                return Err(anyhow!("missing policy tag"));
            }
            if has_tag(raw, "policy_ref") {
                require_tag_value(raw, "policy_ref")?;
            }
        }
        KipKind::Attestation => {
            let sub_tag = require_tag(raw, "sub")?;
            if sub_tag.len() < 3 {
                return Err(anyhow!("invalid sub tag"));
            }
            require_tag_value(raw, "claim")?;
            require_exp_tag(raw, options.now)?;
            validate_schema(raw, "kukuri-attest-v1")?;
        }
        KipKind::TrustAnchor => {
            require_tag_value(raw, "attester")?;
            require_tag_value(raw, "weight")?;
        }
        KipKind::KeyEnvelope => {
            require_tag_value(raw, "p")?;
            require_tag_value(raw, "t")?;
            let scope = require_tag_value(raw, "scope")?;
            validate_scope(&scope, false)?;
            let epoch = require_tag_value(raw, "epoch")?
                .parse::<i64>()
                .map_err(|_| anyhow!("invalid epoch tag"))?;
            if epoch <= 0 {
                return Err(anyhow!("invalid epoch tag"));
            }
            require_tag_value(raw, "d")?;
        }
        KipKind::InviteCapability => {
            require_tag_value(raw, "t")?;
            let scope = require_tag_value(raw, "scope")?;
            if scope != "invite" {
                return Err(anyhow!("invalid scope: {scope}"));
            }
            require_tag_value(raw, "d")?;
        }
        KipKind::JoinRequest => {
            require_tag_value(raw, "t")?;
            let scope = require_tag_value(raw, "scope")?;
            match scope.as_str() {
                "invite" | "friend" => {}
                _ => return Err(anyhow!("invalid scope: {scope}")),
            }
            require_tag_value(raw, "d")?;
            validate_schema(raw, "kukuri-join-request-v1")?;
        }
    }

    Ok(kind)
}

fn require_tag_value(raw: &RawEvent, name: &str) -> Result<String> {
    raw.first_tag_value(name)
        .ok_or_else(|| anyhow!("missing {} tag", name))
}

fn require_tag(raw: &RawEvent, name: &str) -> Result<Vec<String>> {
    raw.tags
        .iter()
        .find(|tag| tag.first().map(|value| value.as_str()) == Some(name))
        .cloned()
        .ok_or_else(|| anyhow!("missing {} tag", name))
}

fn has_tag(raw: &RawEvent, name: &str) -> bool {
    raw.tags
        .iter()
        .any(|tag| tag.first().map(|value| value.as_str()) == Some(name))
}

fn require_exp_tag(raw: &RawEvent, now: i64) -> Result<i64> {
    let exp = raw
        .exp_tag()
        .ok_or_else(|| anyhow!("missing exp tag"))?;
    if exp <= now {
        return Err(anyhow!("expired exp tag"));
    }
    Ok(exp)
}

fn validate_scope(scope: &str, allow_public: bool) -> Result<()> {
    match scope {
        "friend_plus" | "friend" | "invite" => Ok(()),
        "public" if allow_public => Ok(()),
        _ => Err(anyhow!("invalid scope: {scope}")),
    }
}

fn validate_schema(raw: &RawEvent, expected: &str) -> Result<()> {
    let content: Value =
        serde_json::from_str(&raw.content).map_err(|_| anyhow!("invalid content json"))?;
    let schema = content
        .get("schema")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("missing schema field"))?;
    if schema != expected {
        return Err(anyhow!("invalid schema: {}", schema));
    }
    Ok(())
}

fn current_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cn_core::nostr;
    use nostr_sdk::prelude::Keys;
    use serde_json::json;

    #[test]
    fn validate_kip_event_rejects_unknown_kind() {
        let keys = Keys::generate();
        let event = nostr::build_signed_event(&keys, 1, vec![], "hello".to_string())
            .expect("event");
        let err = validate_kip_event(&event, ValidationOptions::default())
            .expect_err("unsupported");
        assert!(err.to_string().contains("unsupported kind"));
    }

    #[test]
    fn validate_node_descriptor_requires_tags() {
        let keys = Keys::generate();
        let tags = vec![
            vec!["d".to_string(), "descriptor".to_string()],
            vec!["k".to_string(), KIP_NAMESPACE.to_string()],
            vec!["ver".to_string(), KIP_VERSION.to_string()],
            vec!["exp".to_string(), (current_unix_seconds() + 60).to_string()],
        ];
        let content = json!({ "schema": "kukuri-node-desc-v1" }).to_string();
        let event = nostr::build_signed_event(&keys, KIND_NODE_DESCRIPTOR as u16, tags, content)
            .expect("event");
        let kind = validate_kip_event(&event, ValidationOptions::default()).expect("valid");
        assert_eq!(kind, KipKind::NodeDescriptor);
    }

    #[test]
    fn validate_label_requires_policy_tag() {
        let keys = Keys::generate();
        let tags = vec![
            vec!["k".to_string(), KIP_NAMESPACE.to_string()],
            vec!["ver".to_string(), KIP_VERSION.to_string()],
            vec!["target".to_string(), "event:abc".to_string()],
            vec!["label".to_string(), "spam".to_string()],
            vec!["exp".to_string(), (current_unix_seconds() + 60).to_string()],
        ];
        let event = nostr::build_signed_event(&keys, KIND_LABEL as u16, tags, String::new())
            .expect("event");
        let err = validate_kip_event(&event, ValidationOptions::default()).expect_err("invalid");
        assert!(err.to_string().contains("missing policy"));
    }

    #[test]
    fn validate_kip_event_rejects_invalid_signature_when_enabled() {
        let keys = Keys::generate();
        let tags = vec![
            vec!["d".to_string(), "descriptor".to_string()],
            vec!["k".to_string(), KIP_NAMESPACE.to_string()],
            vec!["ver".to_string(), KIP_VERSION.to_string()],
            vec!["exp".to_string(), (current_unix_seconds() + 60).to_string()],
        ];
        let content = json!({ "schema": "kukuri-node-desc-v1" }).to_string();
        let mut event = nostr::build_signed_event(&keys, KIND_NODE_DESCRIPTOR as u16, tags, content)
            .expect("event");
        event.sig = "00".repeat(64);

        let err = validate_kip_event(
            &event,
            ValidationOptions {
                verify_signature: true,
                ..ValidationOptions::default()
            },
        )
        .expect_err("invalid signature");
        assert!(err.to_string().contains("verify"));
    }

    #[test]
    fn validate_kip_event_allows_missing_k_tag_when_disabled() {
        let keys = Keys::generate();
        let now = current_unix_seconds();
        let tags = vec![
            vec!["d".to_string(), "descriptor".to_string()],
            vec!["ver".to_string(), KIP_VERSION.to_string()],
            vec!["exp".to_string(), (now + 60).to_string()],
        ];
        let content = json!({ "schema": "kukuri-node-desc-v1" }).to_string();
        let event = nostr::build_signed_event(&keys, KIND_NODE_DESCRIPTOR as u16, tags, content)
            .expect("event");

        let kind = validate_kip_event(
            &event,
            ValidationOptions {
                now,
                verify_signature: false,
                require_k_tag: false,
                require_ver_tag: true,
            },
        )
        .expect("valid");
        assert_eq!(kind, KipKind::NodeDescriptor);
    }

    #[test]
    fn validate_invite_capability_requires_invite_scope() {
        let keys = Keys::generate();
        let tags = vec![
            vec!["t".to_string(), "kukuri:topic1".to_string()],
            vec!["scope".to_string(), "friend".to_string()],
            vec!["d".to_string(), "invite:nonce".to_string()],
            vec!["k".to_string(), KIP_NAMESPACE.to_string()],
            vec!["ver".to_string(), KIP_VERSION.to_string()],
        ];
        let content = json!({ "schema": "kukuri-invite-v1" }).to_string();
        let event =
            nostr::build_signed_event(&keys, KIND_INVITE_CAPABILITY as u16, tags, content)
                .expect("event");

        let err = validate_kip_event(
            &event,
            ValidationOptions {
                verify_signature: false,
                ..ValidationOptions::default()
            },
        )
        .expect_err("invalid scope");
        assert!(err.to_string().contains("invalid scope"));
    }

    #[test]
    fn validate_join_request_accepts_invite_scope() {
        let keys = Keys::generate();
        let tags = vec![
            vec!["t".to_string(), "kukuri:topic1".to_string()],
            vec!["scope".to_string(), "invite".to_string()],
            vec!["d".to_string(), "join:kukuri:topic1:nonce:requester".to_string()],
            vec!["k".to_string(), KIP_NAMESPACE.to_string()],
            vec!["ver".to_string(), KIP_VERSION.to_string()],
        ];
        let content = json!({ "schema": "kukuri-join-request-v1" }).to_string();
        let event = nostr::build_signed_event(&keys, KIND_JOIN_REQUEST as u16, tags, content)
            .expect("event");

        let kind = validate_kip_event(
            &event,
            ValidationOptions {
                verify_signature: false,
                ..ValidationOptions::default()
            },
        )
        .expect("valid");
        assert_eq!(kind, KipKind::JoinRequest);
    }

    #[test]
    fn validate_join_request_rejects_invalid_scope() {
        let keys = Keys::generate();
        let tags = vec![
            vec!["t".to_string(), "kukuri:topic1".to_string()],
            vec!["scope".to_string(), "friend_plus".to_string()],
            vec!["d".to_string(), "join:kukuri:topic1:nonce:requester".to_string()],
            vec!["k".to_string(), KIP_NAMESPACE.to_string()],
            vec!["ver".to_string(), KIP_VERSION.to_string()],
        ];
        let content = json!({ "schema": "kukuri-join-request-v1" }).to_string();
        let event = nostr::build_signed_event(&keys, KIND_JOIN_REQUEST as u16, tags, content)
            .expect("event");

        let err = validate_kip_event(
            &event,
            ValidationOptions {
                verify_signature: false,
                ..ValidationOptions::default()
            },
        )
        .expect_err("invalid scope");
        assert!(err.to_string().contains("invalid scope"));
    }

    #[test]
    fn validate_node_topic_service_accepts_valid_event() {
        let keys = Keys::generate();
        let tags = vec![
            vec!["d".to_string(), "service:topic".to_string()],
            vec!["t".to_string(), "kukuri:topic1".to_string()],
            vec!["role".to_string(), "relay".to_string()],
            vec!["scope".to_string(), "public".to_string()],
            vec!["exp".to_string(), (current_unix_seconds() + 60).to_string()],
            vec!["k".to_string(), KIP_NAMESPACE.to_string()],
            vec!["ver".to_string(), KIP_VERSION.to_string()],
        ];
        let content = json!({ "schema": "kukuri-topic-service-v1" }).to_string();
        let event =
            nostr::build_signed_event(&keys, KIND_NODE_TOPIC_SERVICE as u16, tags, content)
                .expect("event");
        let kind = validate_kip_event(&event, ValidationOptions::default()).expect("valid");
        assert_eq!(kind, KipKind::NodeTopicService);
    }

    #[test]
    fn validate_report_accepts_valid_event() {
        let keys = Keys::generate();
        let tags = vec![
            vec!["k".to_string(), KIP_NAMESPACE.to_string()],
            vec!["ver".to_string(), KIP_VERSION.to_string()],
            vec!["target".to_string(), "event:abc".to_string()],
            vec!["reason".to_string(), "spam".to_string()],
        ];
        let event = nostr::build_signed_event(&keys, KIND_REPORT as u16, tags, String::new())
            .expect("event");
        let kind = validate_kip_event(&event, ValidationOptions::default()).expect("valid");
        assert_eq!(kind, KipKind::Report);
    }

    #[test]
    fn validate_attestation_accepts_valid_event() {
        let keys = Keys::generate();
        let tags = vec![
            vec!["sub".to_string(), "topic".to_string(), "trust".to_string()],
            vec!["claim".to_string(), "score:0.7".to_string()],
            vec!["exp".to_string(), (current_unix_seconds() + 60).to_string()],
            vec!["k".to_string(), KIP_NAMESPACE.to_string()],
            vec!["ver".to_string(), KIP_VERSION.to_string()],
        ];
        let content = json!({ "schema": "kukuri-attest-v1" }).to_string();
        let event = nostr::build_signed_event(&keys, KIND_ATTESTATION as u16, tags, content)
            .expect("event");
        let kind = validate_kip_event(&event, ValidationOptions::default()).expect("valid");
        assert_eq!(kind, KipKind::Attestation);
    }

    #[test]
    fn validate_trust_anchor_accepts_valid_event() {
        let keys = Keys::generate();
        let tags = vec![
            vec!["attester".to_string(), keys.public_key().to_hex()],
            vec!["weight".to_string(), "0.5".to_string()],
            vec!["k".to_string(), KIP_NAMESPACE.to_string()],
            vec!["ver".to_string(), KIP_VERSION.to_string()],
        ];
        let event = nostr::build_signed_event(&keys, KIND_TRUST_ANCHOR as u16, tags, String::new())
            .expect("event");
        let kind = validate_kip_event(&event, ValidationOptions::default()).expect("valid");
        assert_eq!(kind, KipKind::TrustAnchor);
    }

    #[test]
    fn validate_key_envelope_accepts_valid_event() {
        let signer_keys = Keys::generate();
        let recipient_keys = Keys::generate();
        let tags = vec![
            vec!["p".to_string(), recipient_keys.public_key().to_hex()],
            vec!["t".to_string(), "kukuri:topic1".to_string()],
            vec!["scope".to_string(), "friend".to_string()],
            vec!["epoch".to_string(), "1".to_string()],
            vec!["d".to_string(), "envelope:1".to_string()],
            vec!["k".to_string(), KIP_NAMESPACE.to_string()],
            vec!["ver".to_string(), KIP_VERSION.to_string()],
        ];
        let event =
            nostr::build_signed_event(&signer_keys, KIND_KEY_ENVELOPE as u16, tags, String::new())
                .expect("event");
        let kind = validate_kip_event(&event, ValidationOptions::default()).expect("valid");
        assert_eq!(kind, KipKind::KeyEnvelope);
    }

    #[test]
    fn validate_kip_event_rejects_expired_event() {
        let keys = Keys::generate();
        let now = current_unix_seconds();
        let tags = vec![
            vec!["d".to_string(), "descriptor".to_string()],
            vec!["k".to_string(), KIP_NAMESPACE.to_string()],
            vec!["ver".to_string(), KIP_VERSION.to_string()],
            vec!["exp".to_string(), (now - 10).to_string()],
        ];
        let content = json!({ "schema": "kukuri-node-desc-v1" }).to_string();
        let event = nostr::build_signed_event(&keys, KIND_NODE_DESCRIPTOR as u16, tags, content)
            .expect("event");

        let err = validate_kip_event(
            &event,
            ValidationOptions {
                now,
                verify_signature: false,
                ..ValidationOptions::default()
            },
        )
        .expect_err("expired");
        assert!(err.to_string().contains("expired"));
    }
}
