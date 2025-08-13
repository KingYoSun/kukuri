use crate::presentation::dto::Validate;
use serde::{Deserialize, Serialize};

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
pub struct DeleteEventsRequest {
    pub event_ids: Vec<String>,
    pub reason: Option<String>,
}

impl Validate for DeleteEventsRequest {
    fn validate(&self) -> Result<(), String> {
        if self.event_ids.is_empty() {
            return Err("At least one event ID is required".to_string());
        }
        if self.event_ids.len() > 100 {
            return Err("Too many events to delete (max 100)".to_string());
        }
        if let Some(reason) = &self.reason {
            if reason.len() > 500 {
                return Err("Reason is too long (max 500 characters)".to_string());
            }
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