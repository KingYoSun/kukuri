use crate::application::services::{SubscriptionRecord, SubscriptionTarget};
use crate::presentation::dto::Validate;
use nostr_sdk::prelude::Url;
use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

const MAX_NIP65_RELAYS: usize = 64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nip65RelayDto {
    pub url: String,
    #[serde(default = "default_true")]
    pub read: bool,
    #[serde(default = "default_true")]
    pub write: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NostrMetadataDto {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub about: Option<String>,
    pub picture: Option<String>,
    pub banner: Option<String>,
    pub nip05: Option<String>,
    pub lud16: Option<String>,
    pub website: Option<String>,
    pub relays: Option<Vec<Nip65RelayDto>>,
    #[serde(rename = "kukuri_privacy")]
    pub privacy: Option<PrivacyPreferencesDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PrivacyPreferencesDto {
    pub public_profile: Option<bool>,
    pub show_online_status: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishTextNoteRequest {
    pub content: String,
}

impl Validate for PublishTextNoteRequest {
    fn validate(&self) -> Result<(), String> {
        if self.content.is_empty() {
            return Err("Content cannot be empty".to_string());
        }
        if self.content.len() > 10000 {
            return Err("Content is too long (max 10000 characters)".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishTopicPostRequest {
    pub topic_id: String,
    pub content: String,
    pub reply_to: Option<String>,
}

impl Validate for PublishTopicPostRequest {
    fn validate(&self) -> Result<(), String> {
        if self.topic_id.is_empty() {
            return Err("Topic ID is required".to_string());
        }
        if self.content.is_empty() {
            return Err("Content cannot be empty".to_string());
        }
        if self.content.len() > 10000 {
            return Err("Content is too long (max 10000 characters)".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendReactionRequest {
    pub event_id: String,
    pub reaction: String,
}

impl Validate for SendReactionRequest {
    fn validate(&self) -> Result<(), String> {
        if self.event_id.is_empty() {
            return Err("Event ID is required".to_string());
        }
        if self.reaction.is_empty() {
            return Err("Reaction cannot be empty".to_string());
        }
        if self.reaction.len() > 20 {
            return Err("Reaction is too long (max 20 characters)".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMetadataRequest {
    pub metadata: NostrMetadataDto,
}

impl Validate for UpdateMetadataRequest {
    fn validate(&self) -> Result<(), String> {
        // 各フィールドの長さチェック
        if let Some(name) = &self.metadata.name {
            if name.len() > 100 {
                return Err("Name is too long (max 100 characters)".to_string());
            }
        }
        if let Some(display_name) = &self.metadata.display_name {
            if display_name.len() > 100 {
                return Err("Display name is too long (max 100 characters)".to_string());
            }
        }
        if let Some(about) = &self.metadata.about {
            if about.len() > 1000 {
                return Err("About is too long (max 1000 characters)".to_string());
            }
        }
        if let Some(relays) = &self.metadata.relays {
            if relays.len() > MAX_NIP65_RELAYS {
                return Err(format!(
                    "Relay list is too long (max {MAX_NIP65_RELAYS} entries)"
                ));
            }
            for relay in relays {
                if relay.url.trim().is_empty() {
                    return Err("Relay URL cannot be empty".to_string());
                }
                let parsed = Url::parse(relay.url.as_str())
                    .map_err(|_| "Relay URL must be a valid websocket URL".to_string())?;
                match parsed.scheme() {
                    "ws" | "wss" => {}
                    _ => {
                        return Err("Relay URL must use ws:// or wss://".to_string());
                    }
                }
            }
        }
        // URLの検証は省略（実際の実装では必要に応じて追加）
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeRequest {
    pub topic_id: String,
}

impl Validate for SubscribeRequest {
    fn validate(&self) -> Result<(), String> {
        if self.topic_id.is_empty() {
            return Err("Topic ID is required".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventResponse {
    pub event_id: String,
    pub success: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetDefaultP2PTopicRequest {
    pub topic_id: String,
}

impl Validate for SetDefaultP2PTopicRequest {
    fn validate(&self) -> Result<(), String> {
        if self.topic_id.is_empty() {
            return Err("Topic ID is required".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NostrSubscriptionStateDto {
    pub target: String,
    pub target_type: String,
    pub status: String,
    pub last_synced_at: Option<i64>,
    pub last_attempt_at: Option<i64>,
    pub failure_count: i64,
    pub error_message: Option<String>,
}

impl From<SubscriptionRecord> for NostrSubscriptionStateDto {
    fn from(record: SubscriptionRecord) -> Self {
        let (target_type, target_value) = match record.target {
            SubscriptionTarget::Topic(id) => ("topic".to_string(), id),
            SubscriptionTarget::User(id) => ("user".to_string(), id),
        };
        Self {
            target: target_value,
            target_type,
            status: record.status.as_str().to_string(),
            last_synced_at: record.last_synced_at,
            last_attempt_at: record.last_attempt_at,
            failure_count: record.failure_count,
            error_message: record.error_message,
        }
    }
}
