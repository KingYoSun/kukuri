use anyhow::Result;
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use std::path::Path;
use tracing::info;

pub type DbPool = Pool<Sqlite>;

pub struct Database;

impl Database {
    pub async fn initialize(database_url: &str) -> Result<DbPool> {
        // Create database directory
        if let Some(parent) = Path::new(database_url).parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Create database connection pool
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        info!("Database connected: {}", database_url);

        // Run migrations
        Self::run_migrations(&pool).await?;

        Ok(pool)
    }

    async fn run_migrations(pool: &DbPool) -> Result<()> {
        info!("Running database migrations...");

        // Execute migrations
        sqlx::migrate!("./migrations").run(pool).await?;

        info!("Database migrations completed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_db() -> Result<(DbPool, TempDir)> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&db_url)
            .await?;

        Ok((pool, temp_dir))
    }

    #[tokio::test]
    async fn test_database_initialize() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_init.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

        // Initialize database
        let result = Database::initialize(&db_url).await;
        assert!(result.is_ok());

        let pool = result.unwrap();

        // Verify database file was created
        assert!(db_path.exists());

        // Close the pool
        pool.close().await;
    }

    #[tokio::test]
    async fn test_database_tables_created() {
        let (pool, _temp_dir) = create_test_db().await.unwrap();

        // Run migrations manually for testing
        let result = sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS profiles (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                public_key TEXT NOT NULL UNIQUE,
                display_name TEXT,
                about TEXT,
                picture_url TEXT,
                banner_url TEXT,
                nip05 TEXT,
                created_at INTEGER NOT NULL DEFAULT (unixepoch()),
                updated_at INTEGER NOT NULL DEFAULT (unixepoch())
            )
            "#,
        )
        .execute(&pool)
        .await;

        assert!(result.is_ok());

        // Verify table exists
        let table_check =
            sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='profiles'")
                .fetch_optional(&pool)
                .await
                .unwrap();

        assert!(table_check.is_some());

        pool.close().await;
    }

    #[tokio::test]
    async fn test_database_connection_pool() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_pool.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await
            .unwrap();

        // Test concurrent connections
        let mut handles = vec![];
        for i in 0..5 {
            let pool_clone = pool.clone();
            let handle = tokio::spawn(async move {
                let result = sqlx::query("SELECT 1 as value")
                    .fetch_one(&pool_clone)
                    .await;
                assert!(result.is_ok());
                i
            });
            handles.push(handle);
        }

        for handle in handles {
            let result = handle.await;
            assert!(result.is_ok());
        }

        pool.close().await;
    }
}
