use serde::{Deserialize, Serialize};
use super::{PaginationRequest, Validate};

// レスポンスDTO
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PostResponse {
    pub id: String,
    pub content: String,
    pub author_pubkey: String,
    pub author_npub: String,
    pub topic_id: String,
    pub created_at: i64,
    pub likes: u32,
    pub boosts: u32,
    pub replies: u32,
    pub is_synced: bool,
}

// リクエストDTO
#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePostRequest {
    pub content: String,
    pub topic_id: String,
    pub media_urls: Option<Vec<String>>,
}

impl Validate for CreatePostRequest {
    fn validate(&self) -> Result<(), String> {
        if self.content.trim().is_empty() {
            return Err("投稿内容が空です".to_string());
        }
        if self.content.len() > 5000 {
            return Err("投稿内容が長すぎます（最大5000文字）".to_string());
        }
        if self.topic_id.trim().is_empty() {
            return Err("トピックIDが必要です".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetPostsRequest {
    pub topic_id: Option<String>,
    pub author_pubkey: Option<String>,
    pub pagination: Option<PaginationRequest>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeletePostRequest {
    pub post_id: String,
    pub reason: Option<String>,
}

impl Validate for DeletePostRequest {
    fn validate(&self) -> Result<(), String> {
        if self.post_id.trim().is_empty() {
            return Err("投稿IDが必要です".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReactToPostRequest {
    pub post_id: String,
    pub reaction: String,
}

impl Validate for ReactToPostRequest {
    fn validate(&self) -> Result<(), String> {
        if self.post_id.trim().is_empty() {
            return Err("投稿IDが必要です".to_string());
        }
        if self.reaction.trim().is_empty() {
            return Err("リアクションが必要です".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BookmarkPostRequest {
    pub post_id: String,
}

impl Validate for BookmarkPostRequest {
    fn validate(&self) -> Result<(), String> {
        if self.post_id.trim().is_empty() {
            return Err("投稿IDが必要です".to_string());
        }
        Ok(())
    }
}