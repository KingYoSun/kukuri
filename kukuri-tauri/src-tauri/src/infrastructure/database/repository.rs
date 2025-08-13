use crate::domain::entities::{Event, Post, Topic, User};
use async_trait::async_trait;

#[async_trait]
pub trait Repository: PostRepository + TopicRepository + UserRepository + EventRepository {
    async fn initialize(&self) -> Result<(), Box<dyn std::error::Error>>;
    async fn health_check(&self) -> Result<bool, Box<dyn std::error::Error>>;
}

#[async_trait]
pub trait PostRepository: Send + Sync {
    async fn create_post(&self, post: &Post) -> Result<(), Box<dyn std::error::Error>>;
    async fn get_post(&self, id: &str) -> Result<Option<Post>, Box<dyn std::error::Error>>;
    async fn get_posts_by_topic(&self, topic_id: &str, limit: usize) -> Result<Vec<Post>, Box<dyn std::error::Error>>;
    async fn update_post(&self, post: &Post) -> Result<(), Box<dyn std::error::Error>>;
    async fn delete_post(&self, id: &str) -> Result<(), Box<dyn std::error::Error>>;
    async fn get_unsync_posts(&self) -> Result<Vec<Post>, Box<dyn std::error::Error>>;
    async fn mark_post_synced(&self, id: &str, event_id: &str) -> Result<(), Box<dyn std::error::Error>>;
}

#[async_trait]
pub trait TopicRepository: Send + Sync {
    async fn create_topic(&self, topic: &Topic) -> Result<(), Box<dyn std::error::Error>>;
    async fn get_topic(&self, id: &str) -> Result<Option<Topic>, Box<dyn std::error::Error>>;
    async fn get_all_topics(&self) -> Result<Vec<Topic>, Box<dyn std::error::Error>>;
    async fn get_joined_topics(&self) -> Result<Vec<Topic>, Box<dyn std::error::Error>>;
    async fn update_topic(&self, topic: &Topic) -> Result<(), Box<dyn std::error::Error>>;
    async fn delete_topic(&self, id: &str) -> Result<(), Box<dyn std::error::Error>>;
    async fn join_topic(&self, id: &str) -> Result<(), Box<dyn std::error::Error>>;
    async fn leave_topic(&self, id: &str) -> Result<(), Box<dyn std::error::Error>>;
    async fn update_topic_stats(&self, id: &str, member_count: u32, post_count: u32) -> Result<(), Box<dyn std::error::Error>>;
}

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create_user(&self, user: &User) -> Result<(), Box<dyn std::error::Error>>;
    async fn get_user(&self, npub: &str) -> Result<Option<User>, Box<dyn std::error::Error>>;
    async fn get_user_by_pubkey(&self, pubkey: &str) -> Result<Option<User>, Box<dyn std::error::Error>>;
    async fn update_user(&self, user: &User) -> Result<(), Box<dyn std::error::Error>>;
    async fn delete_user(&self, npub: &str) -> Result<(), Box<dyn std::error::Error>>;
    async fn get_followers(&self, npub: &str) -> Result<Vec<User>, Box<dyn std::error::Error>>;
    async fn get_following(&self, npub: &str) -> Result<Vec<User>, Box<dyn std::error::Error>>;
}

#[async_trait]
pub trait EventRepository: Send + Sync {
    async fn create_event(&self, event: &Event) -> Result<(), Box<dyn std::error::Error>>;
    async fn get_event(&self, id: &str) -> Result<Option<Event>, Box<dyn std::error::Error>>;
    async fn get_events_by_kind(&self, kind: u32, limit: usize) -> Result<Vec<Event>, Box<dyn std::error::Error>>;
    async fn get_events_by_author(&self, pubkey: &str, limit: usize) -> Result<Vec<Event>, Box<dyn std::error::Error>>;
    async fn delete_event(&self, id: &str) -> Result<(), Box<dyn std::error::Error>>;
    async fn get_unsync_events(&self) -> Result<Vec<Event>, Box<dyn std::error::Error>>;
    async fn mark_event_synced(&self, id: &str) -> Result<(), Box<dyn std::error::Error>>;
}