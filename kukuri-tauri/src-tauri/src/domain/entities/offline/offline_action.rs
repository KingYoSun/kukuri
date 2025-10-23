use crate::domain::value_objects::{
    EntityId, OfflineActionId, OfflineActionType, OfflinePayload, PublicKey, RemoteEventId,
    SyncStatus,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OfflineActionRecord {
    pub record_id: Option<i64>,
    pub action_id: OfflineActionId,
    pub user_pubkey: PublicKey,
    pub action_type: OfflineActionType,
    pub target_id: Option<EntityId>,
    pub payload: OfflinePayload,
    pub remote_id: Option<RemoteEventId>,
    pub sync_status: SyncStatus,
    pub created_at: DateTime<Utc>,
    pub synced_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

impl OfflineActionRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        record_id: Option<i64>,
        action_id: OfflineActionId,
        user_pubkey: PublicKey,
        action_type: OfflineActionType,
        target_id: Option<EntityId>,
        payload: OfflinePayload,
        sync_status: SyncStatus,
        created_at: DateTime<Utc>,
        synced_at: Option<DateTime<Utc>>,
        remote_id: Option<RemoteEventId>,
    ) -> Self {
        Self {
            record_id,
            action_id,
            user_pubkey,
            action_type,
            target_id,
            payload,
            remote_id,
            sync_status,
            created_at,
            synced_at,
            error_message: None,
        }
    }

    pub fn with_error_message(mut self, message: Option<String>) -> Self {
        self.error_message = message;
        self
    }

    pub fn mark_synced(
        &mut self,
        status: SyncStatus,
        synced_at: Option<DateTime<Utc>>,
        remote_id: Option<RemoteEventId>,
    ) {
        self.sync_status = status;
        self.synced_at = synced_at;
        self.remote_id = remote_id;
    }
}
