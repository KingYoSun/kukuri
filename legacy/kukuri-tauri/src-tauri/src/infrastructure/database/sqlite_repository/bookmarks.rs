use super::SqliteRepository;
use super::queries::{
    DELETE_BOOKMARK, INSERT_BOOKMARK, SELECT_BOOKMARK_BY_USER_AND_POST, SELECT_BOOKMARKS_BY_USER,
};
use crate::application::ports::repositories::BookmarkRepository;
use crate::domain::entities::Bookmark;
use crate::domain::value_objects::{BookmarkId, EventId, PublicKey};
use crate::shared::{AppError, ValidationFailureKind};
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use sqlx::FromRow;

#[derive(Debug, FromRow)]
struct BookmarkRow {
    id: String,
    user_pubkey: String,
    post_id: String,
    created_at: i64,
}

impl BookmarkRow {
    fn into_domain(self) -> Result<Bookmark, AppError> {
        let id = BookmarkId::new(self.id).map_err(|err| {
            AppError::validation(
                ValidationFailureKind::Generic,
                format!("Invalid BookmarkId: {err}"),
            )
        })?;
        let user_pubkey = PublicKey::from_hex_str(&self.user_pubkey).map_err(|err| {
            AppError::validation(
                ValidationFailureKind::Generic,
                format!("Invalid public key: {err}"),
            )
        })?;
        let post_id = EventId::from_hex(&self.post_id).map_err(|err| {
            AppError::validation(
                ValidationFailureKind::Generic,
                format!("Invalid post id: {err}"),
            )
        })?;
        let created_at = Utc
            .timestamp_millis_opt(self.created_at)
            .single()
            .ok_or_else(|| AppError::DeserializationError("Invalid timestamp".to_string()))?;

        Ok(Bookmark::from_parts(id, user_pubkey, post_id, created_at))
    }
}

impl SqliteRepository {
    async fn fetch_bookmark(
        &self,
        user_pubkey: &PublicKey,
        post_id: &EventId,
    ) -> Result<Bookmark, AppError> {
        let row = sqlx::query_as::<_, BookmarkRow>(SELECT_BOOKMARK_BY_USER_AND_POST)
            .bind(user_pubkey.as_hex())
            .bind(post_id.as_str())
            .fetch_optional(self.pool.get_pool())
            .await?;

        match row {
            Some(row) => row.into_domain(),
            None => Err(AppError::NotFound("Bookmark not found".to_string())),
        }
    }
}

#[async_trait]
impl BookmarkRepository for SqliteRepository {
    async fn create_bookmark(
        &self,
        user_pubkey: &PublicKey,
        post_id: &EventId,
    ) -> Result<Bookmark, AppError> {
        let bookmark = Bookmark::new(user_pubkey.clone(), post_id.clone());

        let result = sqlx::query(INSERT_BOOKMARK)
            .bind(bookmark.id().as_str())
            .bind(bookmark.user_pubkey().as_hex())
            .bind(bookmark.post_id().as_str())
            .bind(bookmark.created_at().timestamp_millis())
            .execute(self.pool.get_pool())
            .await?;

        if result.rows_affected() == 0 {
            return self.fetch_bookmark(user_pubkey, post_id).await;
        }

        Ok(bookmark)
    }

    async fn delete_bookmark(
        &self,
        user_pubkey: &PublicKey,
        post_id: &EventId,
    ) -> Result<(), AppError> {
        sqlx::query(DELETE_BOOKMARK)
            .bind(user_pubkey.as_hex())
            .bind(post_id.as_str())
            .execute(self.pool.get_pool())
            .await?;
        Ok(())
    }

    async fn list_bookmarks(&self, user_pubkey: &PublicKey) -> Result<Vec<Bookmark>, AppError> {
        let rows = sqlx::query_as::<_, BookmarkRow>(SELECT_BOOKMARKS_BY_USER)
            .bind(user_pubkey.as_hex())
            .fetch_all(self.pool.get_pool())
            .await?;

        rows.into_iter().map(BookmarkRow::into_domain).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::database::connection_pool::ConnectionPool;

    async fn setup_repository() -> SqliteRepository {
        let pool = ConnectionPool::new("sqlite::memory:?cache=shared")
            .await
            .expect("failed to create pool");

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS bookmarks (
                id TEXT PRIMARY KEY,
                user_pubkey TEXT NOT NULL,
                post_id TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                UNIQUE(user_pubkey, post_id)
            );
            "#,
        )
        .execute(pool.get_pool())
        .await
        .expect("failed to create table");

        SqliteRepository::new(pool)
    }

    fn sample_pubkey() -> PublicKey {
        let hex = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        PublicKey::from_hex_str(hex).expect("valid pubkey")
    }

    fn sample_event_id() -> EventId {
        EventId::generate()
    }

    #[tokio::test]
    async fn create_and_list_bookmarks() {
        let repo = setup_repository().await;
        let pubkey = sample_pubkey();
        let event_id = sample_event_id();

        repo.create_bookmark(&pubkey, &event_id)
            .await
            .expect("bookmark created");

        let bookmarks = repo.list_bookmarks(&pubkey).await.expect("list");
        assert_eq!(bookmarks.len(), 1);
        assert_eq!(bookmarks[0].post_id().as_str(), event_id.as_str());
        assert_eq!(bookmarks[0].user_pubkey().as_hex(), pubkey.as_hex());
    }

    #[tokio::test]
    async fn create_is_idempotent() {
        let repo = setup_repository().await;
        let pubkey = sample_pubkey();
        let event_id = sample_event_id();

        let first = repo
            .create_bookmark(&pubkey, &event_id)
            .await
            .expect("bookmark created");
        let second = repo
            .create_bookmark(&pubkey, &event_id)
            .await
            .expect("bookmark idempotent");

        assert_eq!(first.id().as_str(), second.id().as_str());
    }

    #[tokio::test]
    async fn delete_bookmark_succeeds() {
        let repo = setup_repository().await;
        let pubkey = sample_pubkey();
        let event_id = sample_event_id();

        repo.create_bookmark(&pubkey, &event_id)
            .await
            .expect("bookmark created");

        repo.delete_bookmark(&pubkey, &event_id)
            .await
            .expect("deleted");

        let bookmarks = repo.list_bookmarks(&pubkey).await.expect("list");
        assert!(bookmarks.is_empty());
    }
}
