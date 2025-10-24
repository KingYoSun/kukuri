use super::ConnectionPool;
use super::Repository;
use crate::shared::error::AppError;
use async_trait::async_trait;

mod bookmarks;
mod events;
mod mapper;
mod posts;
mod queries;
mod topics;
mod users;

pub struct SqliteRepository {
    pool: ConnectionPool,
}

impl SqliteRepository {
    pub fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl Repository for SqliteRepository {
    async fn initialize(&self) -> Result<(), AppError> {
        self.pool.migrate().await?;
        Ok(())
    }

    async fn health_check(&self) -> Result<bool, AppError> {
        let result = sqlx::query("SELECT 1")
            .fetch_one(self.pool.get_pool())
            .await;
        Ok(result.is_ok())
    }
}
