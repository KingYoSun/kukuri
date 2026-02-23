use super::{PaginationRequest, Validate};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// レスポンスDTO
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PostResponse {
    pub id: String,
    pub content: String,
    pub author_pubkey: String,
    pub author_npub: String,
    pub topic_id: String,
    pub thread_namespace: Option<String>,
    pub thread_uuid: Option<String>,
    pub thread_root_event_id: Option<String>,
    pub thread_parent_event_id: Option<String>,
    pub scope: Option<String>,
    pub epoch: Option<i64>,
    pub is_encrypted: bool,
    pub created_at: i64,
    pub likes: u32,
    pub boosts: u32,
    pub replies: u32,
    pub is_synced: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TopicTimelineEntryResponse {
    pub thread_uuid: String,
    pub parent_post: PostResponse,
    pub first_reply: Option<PostResponse>,
    pub reply_count: u32,
    pub last_activity_at: i64,
}

// リクエストDTO
#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePostRequest {
    pub content: String,
    pub topic_id: String,
    pub thread_uuid: String,
    pub reply_to: Option<String>,
    pub media_urls: Option<Vec<String>>,
    pub scope: Option<String>,
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
        if self.thread_uuid.trim().is_empty() {
            return Err("thread_uuid が必要です".to_string());
        }
        if Uuid::parse_str(self.thread_uuid.trim()).is_err() {
            return Err("thread_uuid の形式が不正です".to_string());
        }
        if let Some(normalized) = self.scope.as_deref().map(str::trim)
            && !normalized.is_empty()
            && normalized != "public"
            && normalized != "friend_plus"
            && normalized != "friend"
            && normalized != "invite"
        {
            return Err("スコープが不正です".to_string());
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
pub struct GetThreadPostsRequest {
    pub topic_id: String,
    pub thread_uuid: String,
    pub pagination: Option<PaginationRequest>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetTopicTimelineRequest {
    pub topic_id: String,
    pub pagination: Option<PaginationRequest>,
}

impl Validate for GetTopicTimelineRequest {
    fn validate(&self) -> Result<(), String> {
        if self.topic_id.trim().is_empty() {
            return Err("トピックIDが必要です".to_string());
        }
        Ok(())
    }
}

impl Validate for GetThreadPostsRequest {
    fn validate(&self) -> Result<(), String> {
        if self.topic_id.trim().is_empty() {
            return Err("トピックIDが必要です".to_string());
        }
        if self.thread_uuid.trim().is_empty() {
            return Err("thread_uuid が必要です".to_string());
        }
        if Uuid::parse_str(self.thread_uuid.trim()).is_err() {
            return Err("thread_uuid の形式が不正です".to_string());
        }
        Ok(())
    }
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

#[derive(Debug, Serialize, Deserialize)]
pub struct ListTrendingPostsRequest {
    pub topic_ids: Vec<String>,
    pub per_topic: Option<u32>,
}

impl Validate for ListTrendingPostsRequest {
    fn validate(&self) -> Result<(), String> {
        if self.topic_ids.is_empty() {
            return Err("トピックIDを少なくとも1つ指定してください".to_string());
        }
        if let Some(per_topic) = self.per_topic
            && per_topic == 0
        {
            return Err("トピックごとの取得件数は1以上を指定してください".to_string());
        }
        if let Some(per_topic) = self.per_topic
            && per_topic > 20
        {
            return Err("トピックごとの取得件数は最大20件までです".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrendingTopicPostsResponse {
    pub topic_id: String,
    pub topic_name: String,
    pub relative_rank: u32,
    pub posts: Vec<PostResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListTrendingPostsResponse {
    pub generated_at: i64,
    pub topics: Vec<TrendingTopicPostsResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListFollowingFeedRequest {
    pub cursor: Option<String>,
    pub limit: Option<u32>,
    pub include_reactions: Option<bool>,
}

impl Validate for ListFollowingFeedRequest {
    fn validate(&self) -> Result<(), String> {
        if let Some(limit) = self.limit
            && limit == 0
        {
            return Err("取得件数は1以上で指定してください".to_string());
        }
        if let Some(limit) = self.limit
            && limit > 100
        {
            return Err("取得件数は最大100件までです".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FollowingFeedPageResponse {
    pub items: Vec<PostResponse>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
    pub server_time: i64,
}
