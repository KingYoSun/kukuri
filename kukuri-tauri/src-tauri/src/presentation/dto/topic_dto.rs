use super::{Validate, offline::OfflineAction};
use serde::{Deserialize, Serialize};

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
    pub visibility: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PendingTopicResponse {
    pub pending_id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub offline_action_id: String,
    pub synced_topic_id: Option<String>,
    pub error_message: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnqueueTopicCreationRequest {
    pub name: String,
    pub description: Option<String>,
    pub visibility: Option<String>,
}

impl Validate for EnqueueTopicCreationRequest {
    fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("トピック名が必要です".to_string());
        }
        if self.name.len() > 100 {
            return Err("トピック名は100文字以内で入力してください".to_string());
        }
        if let Some(description) = &self.description {
            if description.len() > 500 {
                return Err("説明は500文字以内で入力してください".to_string());
            }
        }
        if let Some(visibility) = &self.visibility {
            if visibility != "public" && visibility != "private" {
                return Err("visibility must be 'public' or 'private'".to_string());
            }
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnqueueTopicCreationResponse {
    pub pending_topic: PendingTopicResponse,
    pub offline_action: OfflineAction,
}

// リクエストDTO
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTopicRequest {
    pub name: String,
    pub description: String,
    pub image_url: Option<String>,
    pub visibility: Option<String>,
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
        if let Some(visibility) = &self.visibility {
            if visibility != "public" && visibility != "private" {
                return Err("visibility must be 'public' or 'private'".to_string());
            }
        }

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
pub struct DeleteTopicRequest {
    pub id: String,
}

impl Validate for DeleteTopicRequest {
    fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("トピックIDが必要です".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarkPendingTopicSyncedRequest {
    pub pending_id: String,
    pub topic_id: String,
}

impl Validate for MarkPendingTopicSyncedRequest {
    fn validate(&self) -> Result<(), String> {
        if self.pending_id.trim().is_empty() {
            return Err("pending_id is required".to_string());
        }
        if self.topic_id.trim().is_empty() {
            return Err("topic_id is required".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarkPendingTopicFailedRequest {
    pub pending_id: String,
    pub error_message: Option<String>,
}

impl Validate for MarkPendingTopicFailedRequest {
    fn validate(&self) -> Result<(), String> {
        if self.pending_id.trim().is_empty() {
            return Err("pending_id is required".to_string());
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

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ListTrendingTopicsRequest {
    pub limit: Option<u32>,
}

impl Validate for ListTrendingTopicsRequest {
    fn validate(&self) -> Result<(), String> {
        if let Some(limit) = self.limit {
            if limit == 0 {
                return Err("取得件数は1以上で指定してください".to_string());
            }
            if limit > 100 {
                return Err("取得件数は最大100件までです".to_string());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrendingTopicDto {
    pub topic_id: String,
    pub name: String,
    pub description: Option<String>,
    pub member_count: u32,
    pub post_count: u32,
    pub trending_score: f64,
    pub rank: u32,
    pub score_change: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListTrendingTopicsResponse {
    pub generated_at: i64,
    pub topics: Vec<TrendingTopicDto>,
}
