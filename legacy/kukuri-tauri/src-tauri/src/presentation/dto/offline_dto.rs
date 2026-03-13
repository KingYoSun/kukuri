use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct OfflineActionResponse {
    pub id: String,
    pub action_type: String,
    pub entity_type: String,
    pub entity_id: String,
    pub payload: serde_json::Value,
    pub created_at: i64,
}