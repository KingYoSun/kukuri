use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Bookmark {
    pub id: String,
    pub user_pubkey: String,
    pub post_id: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBookmarkRequest {
    pub post_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkResponse {
    pub bookmarks: Vec<Bookmark>,
}