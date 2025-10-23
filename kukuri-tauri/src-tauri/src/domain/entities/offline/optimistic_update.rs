use crate::domain::value_objects::{EntityId, EntityType, OfflinePayload, OptimisticUpdateId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OptimisticUpdateRecord {
    pub record_id: i64,
    pub update_id: OptimisticUpdateId,
    pub entity_type: EntityType,
    pub entity_id: EntityId,
    pub original_data: Option<OfflinePayload>,
    pub updated_data: OfflinePayload,
    pub is_confirmed: bool,
    pub created_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
}
