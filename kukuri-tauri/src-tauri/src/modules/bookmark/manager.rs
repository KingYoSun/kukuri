use anyhow::Result;
use sqlx::SqlitePool;
use tracing::info;
use uuid::Uuid;

use super::types::Bookmark;

pub struct BookmarkManager {
    pool: SqlitePool,
}

impl BookmarkManager {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// ブックマークを追加
    pub async fn add_bookmark(&self, user_pubkey: &str, post_id: &str) -> Result<Bookmark> {
        let id = Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now().timestamp_millis();

        sqlx::query(
            r#"
            INSERT INTO bookmarks (id, user_pubkey, post_id, created_at)
            VALUES (?1, ?2, ?3, ?4)
            "#,
        )
        .bind(&id)
        .bind(user_pubkey)
        .bind(post_id)
        .bind(created_at)
        .execute(&self.pool)
        .await?;

        let bookmark = Bookmark {
            id,
            user_pubkey: user_pubkey.to_string(),
            post_id: post_id.to_string(),
            created_at,
        };

        info!("Added bookmark: {} for post: {}", user_pubkey, post_id);
        Ok(bookmark)
    }

    /// ブックマークを削除
    pub async fn remove_bookmark(&self, user_pubkey: &str, post_id: &str) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM bookmarks
            WHERE user_pubkey = ?1 AND post_id = ?2
            "#,
        )
        .bind(user_pubkey)
        .bind(post_id)
        .execute(&self.pool)
        .await?;

        info!("Removed bookmark: {} for post: {}", user_pubkey, post_id);
        Ok(())
    }

    /// ユーザーがブックマークした投稿IDのリストを取得
    pub async fn get_bookmarked_post_ids(&self, user_pubkey: &str) -> Result<Vec<String>> {
        let post_ids: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT post_id
            FROM bookmarks
            WHERE user_pubkey = ?1
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_pubkey)
        .fetch_all(&self.pool)
        .await?;

        Ok(post_ids)
    }
}
