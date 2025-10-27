use crate::application::ports::repositories::{
    BookmarkRepository, EventRepository, PostRepository, TopicRepository, UserRepository,
};
use crate::shared::error::AppError;
use async_trait::async_trait;

#[async_trait]
pub trait Repository:
    PostRepository + TopicRepository + UserRepository + EventRepository + BookmarkRepository
{
    async fn initialize(&self) -> Result<(), AppError>;
    async fn health_check(&self) -> Result<bool, AppError>;
}
