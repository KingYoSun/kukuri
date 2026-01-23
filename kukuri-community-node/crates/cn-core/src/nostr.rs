use anyhow::{anyhow, Result};
use nostr_sdk::prelude::{Event as NostrEvent, EventBuilder, JsonUtil, Keys, Kind, Tag, TagKind};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawEvent {
    pub id: String,
    pub pubkey: String,
    pub created_at: i64,
    pub kind: u32,
    pub tags: Vec<Vec<String>>,
    pub content: String,
    pub sig: String,
}

impl RawEvent {
    pub fn tag_values(&self, name: &str) -> Vec<String> {
        self.tags
            .iter()
            .filter_map(|tag| {
                if tag.first().map(|v| v.as_str()) == Some(name) {
                    tag.get(1).cloned()
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn first_tag_value(&self, name: &str) -> Option<String> {
        self.tags.iter().find_map(|tag| {
            if tag.first().map(|v| v.as_str()) == Some(name) {
                tag.get(1).cloned()
            } else {
                None
            }
        })
    }

    pub fn topic_ids(&self) -> Vec<String> {
        self.tag_values("t")
    }

    pub fn d_tag(&self) -> Option<String> {
        self.first_tag_value("d")
    }

    pub fn exp_tag(&self) -> Option<i64> {
        self.first_tag_value("exp")
            .and_then(|value| value.parse::<i64>().ok())
    }

    pub fn expiration_tag(&self) -> Option<i64> {
        self.first_tag_value("expiration")
            .and_then(|value| value.parse::<i64>().ok())
    }
}

pub fn parse_event(value: &Value) -> Result<RawEvent> {
    serde_json::from_value(value.clone()).map_err(|err| anyhow!("invalid event json: {err}"))
}

pub fn verify_event(raw: &RawEvent) -> Result<()> {
    let event = to_nostr_event(raw)?;
    event.verify().map_err(|err| anyhow!("event verify failed: {err}"))?;
    Ok(())
}

pub fn to_nostr_event(raw: &RawEvent) -> Result<NostrEvent> {
    let json = serde_json::to_string(raw)?;
    NostrEvent::from_json(json).map_err(|err| anyhow!("failed to parse nostr event: {err}"))
}

pub fn build_signed_event(
    keys: &Keys,
    kind: u16,
    tags: Vec<Vec<String>>,
    content: String,
) -> Result<RawEvent> {
    let mut builder = EventBuilder::new(Kind::Custom(kind), content);
    for tag in tags {
        if tag.is_empty() {
            continue;
        }
        let kind = TagKind::from(tag[0].as_str());
        let values = if tag.len() > 1 { tag[1..].to_vec() } else { Vec::new() };
        builder = builder.tag(Tag::custom(kind, values));
    }
    let signed = builder.sign_with_keys(keys)?;
    let value = serde_json::to_value(&signed)?;
    parse_event(&value)
}
