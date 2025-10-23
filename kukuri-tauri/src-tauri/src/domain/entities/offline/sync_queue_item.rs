use crate::domain::value_objects::{
    OfflineActionType, OfflinePayload, SyncQueueId, SyncQueueStatus,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncQueueItem {
    pub id: SyncQueueId,
    pub action_type: OfflineActionType,
    pub payload: OfflinePayload,
    pub status: SyncQueueStatus,
    pub retry_count: u32,
    pub max_retries: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub synced_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

impl SyncQueueItem {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: SyncQueueId,
        action_type: OfflineActionType,
        payload: OfflinePayload,
        status: SyncQueueStatus,
        retry_count: u32,
        max_retries: u32,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        synced_at: Option<DateTime<Utc>>,
        error_message: Option<String>,
    ) -> Self {
        Self {
            id,
            action_type,
            payload,
            status,
            retry_count,
            max_retries,
            created_at,
            updated_at,
            synced_at,
            error_message,
        }
    }
}
