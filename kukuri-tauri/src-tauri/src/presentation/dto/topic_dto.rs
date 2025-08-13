use serde::{Deserialize, Serialize};
use super::Validate;

// レスポンスDTO
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TopicResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub image_url: Option<String>,
    pub member_count: u32,
    pub post_count: u32,
    pub is_joined: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

// リクエストDTO
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTopicRequest {
    pub name: String,
    pub description: String,
    pub image_url: Option<String>,
}

impl Validate for CreateTopicRequest {
    fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("トピック名が必要です".to_string());
        }
        if self.name.len() > 100 {
            return Err("トピック名が長すぎます（最大100文字）".to_string());
        }
        if self.description.len() > 500 {
            return Err("説明が長すぎます（最大500文字）".to_string());
        }
        
        // URLのバリデーション（もし提供されている場合）
        if let Some(url) = &self.image_url {
            if !url.is_empty() && !url.starts_with("http") {
                return Err("無効な画像URLです".to_string());
            }
        }
        
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateTopicRequest {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
}

impl Validate for UpdateTopicRequest {
    fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("トピックIDが必要です".to_string());
        }
        
        if let Some(name) = &self.name {
            if name.len() > 100 {
                return Err("トピック名が長すぎます（最大100文字）".to_string());
            }
        }
        
        if let Some(desc) = &self.description {
            if desc.len() > 500 {
                return Err("説明が長すぎます（最大500文字）".to_string());
            }
        }
        
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JoinTopicRequest {
    pub topic_id: String,
}

impl Validate for JoinTopicRequest {
    fn validate(&self) -> Result<(), String> {
        if self.topic_id.trim().is_empty() {
            return Err("トピックIDが必要です".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetTopicStatsRequest {
    pub topic_id: String,
}

impl Validate for GetTopicStatsRequest {
    fn validate(&self) -> Result<(), String> {
        if self.topic_id.trim().is_empty() {
            return Err("トピックIDが必要です".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TopicStatsResponse {
    pub topic_id: String,
    pub member_count: u32,
    pub post_count: u32,
    pub active_users_24h: u32,
    pub trending_score: f64,
}