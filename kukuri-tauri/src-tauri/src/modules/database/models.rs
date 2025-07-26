use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Profile {
    pub id: i64,
    pub public_key: String,
    pub display_name: Option<String>,
    pub about: Option<String>,
    pub picture_url: Option<String>,
    pub banner_url: Option<String>,
    pub nip05: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[allow(clippy::struct_field_names)]
pub struct Event {
    pub id: i64,
    pub event_id: String,
    pub public_key: String,
    pub created_at: i64,
    pub kind: i64,
    pub content: String,
    pub tags: String, // JSON format
    pub sig: String,
    pub saved_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Topic {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Relay {
    pub id: i64,
    pub url: String,
    pub name: Option<String>,
    pub is_active: bool,
    pub created_at: i64,
}
