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
            if let Some(scope) = self.scope.as_ref()
                && scope.trim().is_empty()
            {
                return Err("scope must not be empty when provided".to_string());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlJoinResponse {
    pub event_id: String,
    pub sent_topics: Vec<String>,
    pub event_json: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlPendingJoinRequest {
    pub event_id: String,
    pub topic_id: String,
    pub scope: String,
    pub requester_pubkey: String,
    pub target_pubkey: Option<String>,
    pub requested_at: Option<i64>,
    pub received_at: i64,
    pub invite_event_json: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlListJoinRequestsResponse {
    pub items: Vec<AccessControlPendingJoinRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlApproveJoinRequest {
    pub event_id: String,
}

impl Validate for AccessControlApproveJoinRequest {
    fn validate(&self) -> Result<(), String> {
        if self.event_id.trim().is_empty() {
            return Err("event_id is required".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlApproveJoinResponse {
    pub event_id: String,
    pub key_envelope_event_id: String,
    pub key_envelope_event_json: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlRejectJoinRequest {
    pub event_id: String,
}

impl Validate for AccessControlRejectJoinRequest {
    fn validate(&self) -> Result<(), String> {
        if self.event_id.trim().is_empty() {
            return Err("event_id is required".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlIngestEventRequest {
    pub event_json: serde_json::Value,
}

impl Validate for AccessControlIngestEventRequest {
    fn validate(&self) -> Result<(), String> {
        if !self.event_json.is_object() {
            return Err("event_json must be an object".to_string());
        }
        Ok(())
    }
}
