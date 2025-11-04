use crate::domain::entities::{Bookmark, Event, Post, Topic, User};
use crate::domain::value_objects::{EventId, PublicKey};
use crate::shared::error::AppError;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct UserCursorPage {
    pub users: Vec<User>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[async_trait]
pub trait PostRepository: Send + Sync {
    async fn create_post(&self, post: &Post) -> Result<(), AppError>;
    async fn get_post(&self, id: &str) -> Result<Option<Post>, AppError>;
    async fn get_posts_by_topic(&self, topic_id: &str, limit: usize)
    -> Result<Vec<Post>, AppError>;
    async fn update_post(&self, post: &Post) -> Result<(), AppError>;
    async fn delete_post(&self, id: &str) -> Result<(), AppError>;
    async fn get_unsync_posts(&self) -> Result<Vec<Post>, AppError>;
    async fn mark_post_synced(&self, id: &str, event_id: &str) -> Result<(), AppError>;
    async fn get_posts_by_author(
        &self,
        author_pubkey: &str,
        limit: usize,
    ) -> Result<Vec<Post>, AppError>;
    async fn get_recent_posts(&self, limit: usize) -> Result<Vec<Post>, AppError>;
}

#[async_trait]
pub trait TopicRepository: Send + Sync {
    async fn create_topic(&self, topic: &Topic) -> Result<(), AppError>;
    async fn get_topic(&self, id: &str) -> Result<Option<Topic>, AppError>;
    async fn get_all_topics(&self) -> Result<Vec<Topic>, AppError>;
    async fn get_joined_topics(&self, user_pubkey: &str) -> Result<Vec<Topic>, AppError>;
    async fn update_topic(&self, topic: &Topic) -> Result<(), AppError>;
    async fn delete_topic(&self, id: &str) -> Result<(), AppError>;
    async fn join_topic(&self, topic_id: &str, user_pubkey: &str) -> Result<(), AppError>;
    async fn leave_topic(&self, topic_id: &str, user_pubkey: &str) -> Result<(), AppError>;
    async fn update_topic_stats(
        &self,
        topic_id: &str,
        member_count: u32,
        post_count: u32,
    ) -> Result<(), AppError>;
}

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create_user(&self, user: &User) -> Result<(), AppError>;
    async fn get_user(&self, npub: &str) -> Result<Option<User>, AppError>;
    async fn get_user_by_pubkey(&self, pubkey: &str) -> Result<Option<User>, AppError>;
    async fn search_users(&self, query: &str, limit: usize) -> Result<Vec<User>, AppError>;
    async fn update_user(&self, user: &User) -> Result<(), AppError>;
    async fn delete_user(&self, npub: &str) -> Result<(), AppError>;
    async fn get_followers_paginated(
        &self,
        npub: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<UserCursorPage, AppError>;
    async fn get_following_paginated(
        &self,
        npub: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<UserCursorPage, AppError>;
    async fn add_follow_relation(
        &self,
        follower_pubkey: &str,
        followed_pubkey: &str,
    ) -> Result<bool, AppError>;
    async fn remove_follow_relation(
        &self,
        follower_pubkey: &str,
        followed_pubkey: &str,
    ) -> Result<bool, AppError>;
}

#[async_trait]
pub trait EventRepository: Send + Sync {
    async fn create_event(&self, event: &Event) -> Result<(), AppError>;
    async fn get_event(&self, id: &str) -> Result<Option<Event>, AppError>;
    async fn get_events_by_kind(&self, kind: u32, limit: usize) -> Result<Vec<Event>, AppError>;
    async fn get_events_by_author(
        &self,
        pubkey: &str,
        limit: usize,
    ) -> Result<Vec<Event>, AppError>;
    async fn delete_event(&self, id: &str) -> Result<(), AppError>;
    async fn get_unsync_events(&self) -> Result<Vec<Event>, AppError>;
    async fn mark_event_synced(&self, id: &str) -> Result<(), AppError>;

    /// イベントとトピックのマッピングを登録（冪等）
    async fn add_event_topic(&self, _event_id: &str, _topic_id: &str) -> Result<(), AppError> {
        // 既定実装: 実装なし
        Ok(())
    }

    /// イベントが属するトピックID一覧を取得
    async fn get_event_topics(&self, _event_id: &str) -> Result<Vec<String>, AppError> {
        // 既定実装: 空
        Ok(vec![])
    }
}

#[async_trait]
pub trait BookmarkRepository: Send + Sync {
    async fn create_bookmark(
        &self,
        user_pubkey: &PublicKey,
        post_id: &EventId,
    ) -> Result<Bookmark, AppError>;

    async fn delete_bookmark(
        &self,
        user_pubkey: &PublicKey,
        post_id: &EventId,
    ) -> Result<(), AppError>;

    async fn list_bookmarks(&self, user_pubkey: &PublicKey) -> Result<Vec<Bookmark>, AppError>;
}
