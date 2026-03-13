use crate::domain::value_objects::{EntityId, EntityType, OfflinePayload, SyncStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncStatusRecord {
    pub record_id: i64,
    pub entity_type: EntityType,
    pub entity_id: EntityId,
    pub local_version: i32,
    pub remote_version: Option<i32>,
    pub last_local_update: DateTime<Utc>,
    pub last_remote_sync: Option<DateTime<Utc>>,
    pub sync_status: SyncStatus,
    pub conflict_data: Option<OfflinePayload>,
}
