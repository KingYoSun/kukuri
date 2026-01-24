use anyhow::{anyhow, Result};
use nostr_sdk::prelude::Keys;
use serde::{Deserialize, Serialize};

use crate::nostr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleCondition {
    pub kinds: Option<Vec<i32>>,
    pub content_regex: Option<String>,
    pub content_keywords: Option<Vec<String>>,
    pub tag_filters: Option<std::collections::HashMap<String, Vec<String>>>,
    pub author_pubkeys: Option<Vec<String>>,
}

impl RuleCondition {
    pub fn validate(&self) -> Result<()> {
        if let Some(regex) = &self.content_regex {
            if regex.trim().is_empty() {
                return Err(anyhow!("content_regex cannot be empty"));
            }
        }
        if let Some(tags) = &self.tag_filters {
            for (key, values) in tags {
                if key.trim().is_empty() {
                    return Err(anyhow!("tag_filters key cannot be empty"));
                }
                if values.iter().any(|value| value.trim().is_empty()) {
                    return Err(anyhow!("tag_filters values cannot be empty"));
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleAction {
    pub label: String,
    pub confidence: Option<f64>,
    pub exp_seconds: i64,
    pub policy_url: String,
    pub policy_ref: String,
}

impl RuleAction {
    pub fn validate(&self) -> Result<()> {
        if self.label.trim().is_empty() {
            return Err(anyhow!("label is required"));
        }
        if let Some(confidence) = self.confidence {
            if !(0.0..=1.0).contains(&confidence) {
                return Err(anyhow!("confidence must be between 0 and 1"));
            }
        }
        if self.exp_seconds <= 0 {
            return Err(anyhow!("exp_seconds must be positive"));
        }
        if self.policy_url.trim().is_empty() {
            return Err(anyhow!("policy_url is required"));
        }
        if self.policy_ref.trim().is_empty() {
            return Err(anyhow!("policy_ref is required"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationRule {
    pub rule_id: String,
    pub name: String,
    pub description: Option<String>,
    pub is_enabled: bool,
    pub priority: i32,
    pub conditions: RuleCondition,
    pub action: RuleAction,
}

#[derive(Debug, Clone)]
pub struct LabelInput {
    pub target: String,
    pub label: String,
    pub confidence: Option<f64>,
    pub exp: i64,
    pub policy_url: String,
    pub policy_ref: String,
    pub topic_id: Option<String>,
}

impl LabelInput {
    pub fn validate(&self) -> Result<()> {
        if self.target.trim().is_empty() {
            return Err(anyhow!("target is required"));
        }
        if self.label.trim().is_empty() {
            return Err(anyhow!("label is required"));
        }
        if self.exp <= 0 {
            return Err(anyhow!("exp must be positive"));
        }
        if self.policy_url.trim().is_empty() {
            return Err(anyhow!("policy_url is required"));
        }
        if self.policy_ref.trim().is_empty() {
            return Err(anyhow!("policy_ref is required"));
        }
        Ok(())
    }
}

pub fn build_label_event(keys: &Keys, input: &LabelInput) -> Result<nostr::RawEvent> {
    input.validate()?;

    let mut tags = vec![
        vec!["k".to_string(), "kukuri".to_string()],
        vec!["ver".to_string(), "1".to_string()],
        vec!["target".to_string(), input.target.clone()],
        vec!["label".to_string(), input.label.clone()],
        vec!["exp".to_string(), input.exp.to_string()],
        vec!["policy_url".to_string(), input.policy_url.clone()],
        vec!["policy_ref".to_string(), input.policy_ref.clone()],
    ];
    if let Some(confidence) = input.confidence {
        tags.push(vec![
            "confidence".to_string(),
            format!("{:.3}", confidence),
        ]);
    }
    if let Some(topic_id) = &input.topic_id {
        tags.push(vec!["t".to_string(), topic_id.clone()]);
    }

    nostr::build_signed_event(keys, 39006, tags, String::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nostr_sdk::prelude::Keys;

    #[test]
    fn build_label_event_includes_required_tags() {
        let keys = Keys::generate();
        let input = LabelInput {
            target: "event:abc123".to_string(),
            label: "spam".to_string(),
            confidence: Some(0.5),
            exp: 123456,
            policy_url: "https://example.com/policy".to_string(),
            policy_ref: "moderation-v1".to_string(),
            topic_id: Some("topic-1".to_string()),
        };

        let event = build_label_event(&keys, &input).expect("label event builds");
        assert_eq!(event.kind, 39006);
        assert!(event
            .tags
            .iter()
            .any(|tag| tag == &vec!["target".to_string(), "event:abc123".to_string()]));
        assert!(event
            .tags
            .iter()
            .any(|tag| tag == &vec!["policy_url".to_string(), "https://example.com/policy".to_string()]));
        assert!(event
            .tags
            .iter()
            .any(|tag| tag == &vec!["policy_ref".to_string(), "moderation-v1".to_string()]));
    }
}
