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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

// バッチ処理用リクエストDTO
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchGetPostsRequest {
    pub post_ids: Vec<String>,
}

impl Validate for BatchGetPostsRequest {
    fn validate(&self) -> Result<(), String> {
        if self.post_ids.is_empty() {
            return Err("投稿IDが必要です".to_string());
        }
        if self.post_ids.len() > 100 {
            return Err("一度に取得できる投稿は100件までです".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchReactRequest {
    pub reactions: Vec<ReactToPostRequest>,
}

impl Validate for BatchReactRequest {
    fn validate(&self) -> Result<(), String> {
        if self.reactions.is_empty() {
            return Err("リアクションが必要です".to_string());
        }
        if self.reactions.len() > 50 {
            return Err("一度に処理できるリアクションは50件までです".to_string());
        }
        // 各リアクションの検証
        for reaction in &self.reactions {
            reaction.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchBookmarkRequest {
    pub post_ids: Vec<String>,
    pub action: BookmarkAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BookmarkAction {
    Add,
    Remove,
}

impl Validate for BatchBookmarkRequest {
    fn validate(&self) -> Result<(), String> {
        if self.post_ids.is_empty() {
            return Err("投稿IDが必要です".to_string());
        }
        if self.post_ids.len() > 100 {
            return Err("一度に処理できるブックマークは100件までです".to_string());
        }
        Ok(())
    }
}