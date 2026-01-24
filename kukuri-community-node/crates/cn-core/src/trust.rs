use anyhow::{anyhow, Result};
use nostr_sdk::prelude::Keys;
use serde_json::{json, Value};

use crate::nostr;

pub const CLAIM_REPORT_BASED: &str = "moderation.risk";
pub const CLAIM_COMMUNICATION_DENSITY: &str = "reputation";
pub const METHOD_REPORT_BASED: &str = "report-based";
pub const METHOD_COMMUNICATION_DENSITY: &str = "communication-density";

#[derive(Debug, Clone)]
pub struct AttestationInput {
    pub subject: String,
    pub claim: String,
    pub score: f64,
    pub value: Value,
    pub evidence: Vec<String>,
    pub context: Value,
    pub exp: i64,
    pub topic_id: Option<String>,
}

impl AttestationInput {
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

pub fn build_attestation_event(keys: &Keys, input: &AttestationInput) -> Result<nostr::RawEvent> {
    input.validate()?;

    let sub_tag = build_subject_tag(&input.subject)?;
    let mut tags = vec![
        vec!["k".to_string(), "kukuri".to_string()],
        vec!["ver".to_string(), "1".to_string()],
        sub_tag,
        vec!["claim".to_string(), input.claim.clone()],
        vec!["exp".to_string(), input.exp.to_string()],
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

    nostr::build_signed_event(keys, 39010, tags, content)
}

fn build_subject_tag(subject: &str) -> Result<Vec<String>> {
    let mut parts = subject.splitn(2, ':');
    let kind = parts
        .next()
        .ok_or_else(|| anyhow!("invalid subject"))?
        .trim();
    let value = parts
        .next()
        .ok_or_else(|| anyhow!("invalid subject"))?
        .trim();
    if kind.is_empty() || value.is_empty() {
        return Err(anyhow!("invalid subject"));
    }
    Ok(vec![
        "sub".to_string(),
        kind.to_string(),
        value.to_string(),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use nostr_sdk::prelude::Keys;

    #[test]
    fn build_attestation_event_includes_required_tags() {
        let keys = Keys::generate();
        let input = AttestationInput {
            subject: "pubkey:abcd".to_string(),
            claim: "moderation.risk".to_string(),
            score: 0.4,
            value: json!({ "score": 0.4 }),
            evidence: vec![],
            context: json!({ "method": "report-based" }),
            exp: 123456,
            topic_id: Some("topic-1".to_string()),
        };

        let event = build_attestation_event(&keys, &input).expect("attestation builds");
        assert_eq!(event.kind, 39010);
        assert!(event
            .tags
            .iter()
            .any(|tag| tag == &vec!["sub".to_string(), "pubkey".to_string(), "abcd".to_string()]));
        assert!(event
            .tags
            .iter()
            .any(|tag| tag == &vec!["claim".to_string(), "moderation.risk".to_string()]));
        assert!(event
            .tags
            .iter()
            .any(|tag| tag == &vec!["exp".to_string(), "123456".to_string()]));
    }
}
