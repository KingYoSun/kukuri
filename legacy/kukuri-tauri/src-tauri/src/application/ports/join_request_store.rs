use crate::domain::entities::Event;
use crate::shared::error::AppError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRequestRecord {
    pub event: Event,
    pub topic_id: String,
    pub scope: String,
    pub requester_pubkey: String,
    pub target_pubkey: Option<String>,
    pub requested_at: Option<i64>,
    pub received_at: i64,
    pub invite_event_json: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteUsageRecord {
    pub invite_event_id: String,
    pub max_uses: i64,
    pub used_count: i64,
    pub last_used_at: i64,
}

#[async_trait]
pub trait JoinRequestStore: Send + Sync {
    async fn upsert_request(
        &self,
        owner_pubkey: &str,
        record: JoinRequestRecord,
    ) -> Result<(), AppError>;
    async fn list_requests(&self, owner_pubkey: &str) -> Result<Vec<JoinRequestRecord>, AppError>;
    async fn get_request(
        &self,
        owner_pubkey: &str,
        event_id: &str,
    ) -> Result<Option<JoinRequestRecord>, AppError>;
    async fn delete_request(&self, owner_pubkey: &str, event_id: &str) -> Result<(), AppError>;
    async fn get_invite_usage(
        &self,
        owner_pubkey: &str,
        invite_event_id: &str,
    ) -> Result<Option<InviteUsageRecord>, AppError>;
    async fn upsert_invite_usage(
        &self,
        owner_pubkey: &str,
        record: InviteUsageRecord,
    ) -> Result<(), AppError>;
}
