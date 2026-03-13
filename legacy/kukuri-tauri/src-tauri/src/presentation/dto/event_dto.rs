use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct EventResponse {
    pub id: String,
    pub kind: u32,
    pub pubkey: String,
    pub content: String,
    pub tags: Vec<Vec<String>>,
    pub created_at: i64,
    pub sig: String,
}