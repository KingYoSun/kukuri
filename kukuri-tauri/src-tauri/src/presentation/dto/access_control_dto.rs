use crate::presentation::dto::Validate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlIssueInviteRequest {
    pub topic_id: String,
    pub expires_in: Option<i64>,
    pub max_uses: Option<i64>,
    pub nonce: Option<String>,
}

impl Validate for AccessControlIssueInviteRequest {
    fn validate(&self) -> Result<(), String> {
        if self.topic_id.trim().is_empty() {
            return Err("topic_id is required".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlIssueInviteResponse {
    pub invite_event_json: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlJoinRequest {
    pub topic_id: Option<String>,
    pub scope: Option<String>,
    pub invite_event_json: Option<serde_json::Value>,
    pub target_pubkey: Option<String>,
    pub broadcast_to_topic: Option<bool>,
}

impl Validate for AccessControlJoinRequest {
    fn validate(&self) -> Result<(), String> {
        if self.invite_event_json.is_none() {
            let topic = self.topic_id.as_ref().map(|v| v.trim()).unwrap_or("");
            if topic.is_empty() {
                return Err("topic_id is required when invite_event_json is absent".to_string());
            }
            if let Some(scope) = self.scope.as_ref() {
                if scope.trim().is_empty() {
                    return Err("scope must not be empty when provided".to_string());
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlJoinResponse {
    pub event_id: String,
    pub sent_topics: Vec<String>,
}
