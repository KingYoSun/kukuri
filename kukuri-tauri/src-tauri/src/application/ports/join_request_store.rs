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
}
