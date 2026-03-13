use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncResult {
    pub synced_count: u32,
    pub failed_count: u32,
    pub pending_count: u32,
}

impl SyncResult {
    pub fn new(synced_count: u32, failed_count: u32, pending_count: u32) -> Self {
        Self {
            synced_count,
            failed_count,
            pending_count,
        }
    }
}
