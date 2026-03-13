use anyhow::{anyhow, Result};
use nostr_sdk::prelude::{Keys, PublicKey};
use serde_json::{json, Value};

use crate::nostr;

pub const KIND_TRUST_ASSERTION_PUBKEY: u16 = 30382;
pub const KIND_TRUST_ASSERTION_EVENT: u16 = 30383;
pub const KIND_TRUST_ASSERTION_RELAY: u16 = 30384;
pub const KIND_TRUST_ASSERTION_TOPIC: u16 = 30385;
pub const KIND_TRUST_PROVIDER_LIST: u16 = 10040;

pub const CLAIM_REPORT_BASED: &str = "moderation.risk";
pub const CLAIM_COMMUNICATION_DENSITY: &str = "reputation";
pub const METHOD_REPORT_BASED: &str = "report-based";
pub const METHOD_COMMUNICATION_DENSITY: &str = "communication-density";

#[derive(Debug, Clone)]
pub struct TrustedAssertionInput {
    pub subject: String,
    pub claim: String,
    pub score: f64,
    pub value: Value,
    pub evidence: Vec<String>,
    pub context: Value,
    pub exp: i64,
    pub topic_id: Option<String>,
}

impl TrustedAssertionInput {
    pub fn validate(&self) -> Result<()> {
        if self.subject.trim().is_empty() {
            return Err(anyhow!("subject is required"));
        }
        if self.claim.trim().is_empty() {
            return Err(anyhow!("claim is required"));
        }
        if !(0.0..=1.0).contains(&self.score) {
            return Err(anyhow!("score must be between 0 and 1"));
        }
        if self.exp <= 0 {
            return Err(anyhow!("exp must be positive"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct TrustProviderEntry {
    pub assertion_kind: u16,
    pub pubkey: String,
    pub relay_url: Option<String>,
}

impl TrustProviderEntry {
    fn validate(&self) -> Result<()> {
        if !matches!(
            self.assertion_kind,
            KIND_TRUST_ASSERTION_PUBKEY
                | KIND_TRUST_ASSERTION_EVENT
                | KIND_TRUST_ASSERTION_RELAY
                | KIND_TRUST_ASSERTION_TOPIC
        ) {
            return Err(anyhow!("unsupported assertion kind"));
        }
        let pubkey = self.pubkey.trim();
        if pubkey.is_empty() {
            return Err(anyhow!("provider pubkey is required"));
        }
        PublicKey::from_hex(pubkey).map_err(|err| anyhow!("invalid provider pubkey: {err}"))?;
        if let Some(relay_url) = &self.relay_url {
            if relay_url.trim().is_empty() {
                return Err(anyhow!("relay_url must not be empty"));
            }
        }
        Ok(())
    }
}

pub fn build_trusted_assertion_event(
    keys: &Keys,
    input: &TrustedAssertionInput,
) -> Result<nostr::RawEvent> {
    input.validate()?;
    let (kind, d_tag_value) = resolve_subject(&input.subject)?;
    let rank = score_to_rank(input.score);

    let mut tags = vec![
        vec!["d".to_string(), d_tag_value],
        vec!["claim".to_string(), input.claim.clone()],
        vec!["rank".to_string(), rank.to_string()],
        vec!["expiration".to_string(), input.exp.to_string()],
    ];
    if let Some(topic_id) = &input.topic_id {
        tags.push(vec!["t".to_string(), topic_id.clone()]);
    }

    let content = json!({
        "schema": "kukuri-attest-v1",
        "subject": input.subject.clone(),
        "claim": input.claim.clone(),
        "value": input.value.clone(),
        "evidence": input.evidence.clone(),
        "context": input.context.clone(),
        "expires": input.exp
    })
    .to_string();

    nostr::build_signed_event(keys, kind, tags, content)
}

pub fn build_trust_provider_list_event(
    keys: &Keys,
    providers: &[TrustProviderEntry],
) -> Result<nostr::RawEvent> {
    if providers.is_empty() {
        return Err(anyhow!("at least one trust provider is required"));
    }
    let mut tags = Vec::with_capacity(providers.len());
    for provider in providers {
        provider.validate()?;
        let mut tag = vec![
            format!("{}:rank", provider.assertion_kind),
            provider.pubkey.trim().to_string(),
        ];
        if let Some(relay_url) = &provider.relay_url {
            tag.push(relay_url.trim().to_string());
        }
        tags.push(tag);
    }
    nostr::build_signed_event(keys, KIND_TRUST_PROVIDER_LIST, tags, String::new())
}

fn resolve_subject(subject: &str) -> Result<(u16, String)> {
    let mut parts = subject.splitn(2, ':');
    let subject_kind = parts
        .next()
        .ok_or_else(|| anyhow!("invalid subject"))?
        .trim();
    let value = parts
        .next()
        .ok_or_else(|| anyhow!("invalid subject"))?
        .trim();
    if subject_kind.is_empty() || value.is_empty() {
        return Err(anyhow!("invalid subject"));
    }
    match subject_kind {
        "pubkey" => {
            PublicKey::from_hex(value).map_err(|err| anyhow!("invalid pubkey subject: {err}"))?;
            Ok((KIND_TRUST_ASSERTION_PUBKEY, value.to_string()))
        }
        "event" => {
            validate_32byte_hex(value, "event subject")?;
            Ok((KIND_TRUST_ASSERTION_EVENT, value.to_string()))
        }
        "relay" => Ok((KIND_TRUST_ASSERTION_RELAY, value.to_string())),
        "topic" => Ok((KIND_TRUST_ASSERTION_TOPIC, value.to_string())),
        _ => Err(anyhow!("unsupported subject kind: {subject_kind}")),
    }
}

fn validate_32byte_hex(value: &str, field: &str) -> Result<()> {
    let bytes = hex::decode(value).map_err(|err| anyhow!("invalid {field}: {err}"))?;
    if bytes.len() != 32 {
        return Err(anyhow!("invalid {field}: must be 32-byte hex"));
    }
    Ok(())
}

fn score_to_rank(score: f64) -> i64 {
    (score.clamp(0.0, 1.0) * 100.0).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use nostr_sdk::prelude::Keys;

    #[test]
    fn build_trusted_assertion_event_includes_required_tags() {
        let keys = Keys::generate();
        let subject_keys = Keys::generate();
        let input = TrustedAssertionInput {
            subject: format!("pubkey:{}", subject_keys.public_key().to_hex()),
            claim: "moderation.risk".to_string(),
            score: 0.4,
            value: json!({ "score": 0.4 }),
            evidence: vec![],
            context: json!({ "method": "report-based" }),
            exp: 123456,
            topic_id: Some("topic-1".to_string()),
        };

        let event = build_trusted_assertion_event(&keys, &input).expect("assertion builds");
        assert_eq!(event.kind, u32::from(KIND_TRUST_ASSERTION_PUBKEY));
        assert!(event
            .tags
            .iter()
            .any(|tag| tag.get(0) == Some(&"d".to_string())));
        assert!(event
            .tags
            .iter()
            .any(|tag| tag == &vec!["claim".to_string(), "moderation.risk".to_string()]));
        assert!(event
            .tags
            .iter()
            .any(|tag| tag == &vec!["rank".to_string(), "40".to_string()]));
        assert!(event
            .tags
            .iter()
            .any(|tag| tag == &vec!["expiration".to_string(), "123456".to_string()]));
    }

    #[test]
    fn build_trusted_assertion_event_supports_event_subject() {
        let keys = Keys::generate();
        let input = TrustedAssertionInput {
            subject: format!("event:{}", "ab".repeat(32)),
            claim: "reputation".to_string(),
            score: 0.99,
            value: json!({ "score": 0.99 }),
            evidence: vec![],
            context: json!({}),
            exp: 123456,
            topic_id: None,
        };

        let event = build_trusted_assertion_event(&keys, &input).expect("assertion builds");
        assert_eq!(event.kind, u32::from(KIND_TRUST_ASSERTION_EVENT));
        assert_eq!(
            event.first_tag_value("d").as_deref(),
            Some("abababababababababababababababababababababababababababababababab")
        );
    }

    #[test]
    fn build_trust_provider_list_event_uses_10040() {
        let keys = Keys::generate();
        let provider_keys = Keys::generate();
        let providers = vec![TrustProviderEntry {
            assertion_kind: KIND_TRUST_ASSERTION_PUBKEY,
            pubkey: provider_keys.public_key().to_hex(),
            relay_url: Some("wss://relay.example".to_string()),
        }];

        let event = build_trust_provider_list_event(&keys, &providers).expect("provider list");
        assert_eq!(event.kind, u32::from(KIND_TRUST_PROVIDER_LIST));
        assert!(event.tags.iter().any(|tag| {
            tag == &vec![
                "30382:rank".to_string(),
                provider_keys.public_key().to_hex(),
                "wss://relay.example".to_string(),
            ]
        }));
    }
}
