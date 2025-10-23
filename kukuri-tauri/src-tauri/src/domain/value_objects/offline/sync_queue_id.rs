use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SyncQueueId(i64);

impl SyncQueueId {
    pub fn new(value: i64) -> Result<Self, String> {
        if value <= 0 {
            return Err("Sync queue id must be positive".to_string());
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> i64 {
        self.0
    }
}

impl fmt::Display for SyncQueueId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<SyncQueueId> for i64 {
    fn from(id: SyncQueueId) -> Self {
        id.0
    }
}
