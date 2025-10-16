use crate::domain::entities::{Event, Post, Topic, User, UserMetadata};
use crate::shared::error::AppError;
use async_trait::async_trait;

#[async_trait]
pub trait PostRepository: Send + Sync {
    async fn create_post(&self, post: &Post) -> Result<(), AppError>;
    async fn get_post(&self, id: &str) -> Result<Option<Post>, AppError>;
    async fn get_posts_by_topic(
        &self,
        topic_id: &str,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<Post>, AppError>;
    async fn get_posts_by_author(
        &self,
        author_pubkey: &str,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<Post>, AppError>;
    async fn update_post(&self, post: &Post) -> Result<(), AppError>;
    async fn delete_post(&self, id: &str) -> Result<(), AppError>;
    async fn get_unsync_posts(&self) -> Result<Vec<Post>, AppError>;
    async fn mark_post_synced(&self, id: &str) -> Result<(), AppError>;
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
    async fn update_user(&self, user: &User) -> Result<(), AppError>;
    async fn delete_user(&self, npub: &str) -> Result<(), AppError>;
    async fn get_followers(&self, npub: &str) -> Result<Vec<User>, AppError>;
    async fn get_following(&self, npub: &str) -> Result<Vec<User>, AppError>;
}

#[async_trait]
pub trait EventRepository: Send + Sync {
    async fn create_event(&self, event: &Event) -> Result<(), AppError>;
    async fn get_event(&self, id: &str) -> Result<Option<Event>, AppError>;
    async fn get_events_by_kind(
        &self,
        kind: u32,
        limit: Option<u32>,
    ) -> Result<Vec<Event>, AppError>;
    async fn get_events_by_author(
        &self,
        author_pubkey: &str,
        limit: Option<u32>,
    ) -> Result<Vec<Event>, AppError>;
    async fn delete_event(&self, id: &str) -> Result<(), AppError>;
    async fn get_unsync_events(&self) -> Result<Vec<Event>, AppError>;
    async fn mark_event_synced(&self, id: &str) -> Result<(), AppError>;
}
