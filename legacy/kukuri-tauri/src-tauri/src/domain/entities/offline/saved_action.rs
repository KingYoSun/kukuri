use super::OfflineActionRecord;
use crate::domain::value_objects::OfflineActionId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SavedOfflineAction {
    pub local_id: OfflineActionId,
    pub action: OfflineActionRecord,
}

impl SavedOfflineAction {
    pub fn new(local_id: OfflineActionId, action: OfflineActionRecord) -> Self {
        Self { local_id, action }
    }
}
