use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use std::sync::Arc;

#[derive(Clone)]
pub struct ConnectionPool {
    pool: Arc<SqlitePool>,
}

impl ConnectionPool {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    pub async fn from_memory() -> Result<Self, sqlx::Error> {
        Self::new(":memory:").await
    }

    pub fn get_pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn migrate(&self) -> Result<(), sqlx::migrate::MigrateError> {
        sqlx::migrate!("./migrations").run(self.pool.as_ref()).await
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }
}
