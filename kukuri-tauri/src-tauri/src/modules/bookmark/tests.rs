#[cfg(test)]
mod tests {
    use super::super::*;
    use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
    use tempfile::TempDir;
    use uuid::Uuid;

    async fn setup_test_db() -> (SqlitePool, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
        
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await
            .unwrap();

        // テーブル作成
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS bookmarks (
                id TEXT PRIMARY KEY,
                user_pubkey TEXT NOT NULL,
                post_id TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                UNIQUE(user_pubkey, post_id)
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        (pool, temp_dir)
    }

    #[tokio::test]
    async fn test_add_bookmark() {
        let (pool, _temp_dir) = setup_test_db().await;
        let manager = BookmarkManager::new(pool);

        let result = manager.add_bookmark("user1", "post1").await;
        assert!(result.is_ok());
        
        let bookmark = result.unwrap();
        assert_eq!(bookmark.user_pubkey, "user1");
        assert_eq!(bookmark.post_id, "post1");
        assert!(bookmark.created_at > 0);
    }

    #[tokio::test]
    async fn test_add_duplicate_bookmark() {
        let (pool, _temp_dir) = setup_test_db().await;
        let manager = BookmarkManager::new(pool);

        // 最初のブックマーク
        let result1 = manager.add_bookmark("user1", "post1").await;
        assert!(result1.is_ok());

        // 同じユーザーが同じ投稿をブックマーク（エラーになるはず）
        let result2 = manager.add_bookmark("user1", "post1").await;
        assert!(result2.is_err());
    }

    #[tokio::test]
    async fn test_remove_bookmark() {
        let (pool, _temp_dir) = setup_test_db().await;
        let manager = BookmarkManager::new(pool);

        // ブックマークを追加
        manager.add_bookmark("user1", "post1").await.unwrap();

        // ブックマークを削除
        let result = manager.remove_bookmark("user1", "post1").await;
        assert!(result.is_ok());

        // 削除されたことを確認
        let is_bookmarked = manager.is_bookmarked("user1", "post1").await.unwrap();
        assert!(!is_bookmarked);
    }

    #[tokio::test]
    async fn test_remove_nonexistent_bookmark() {
        let (pool, _temp_dir) = setup_test_db().await;
        let manager = BookmarkManager::new(pool);

        // 存在しないブックマークを削除（エラーにはならない）
        let result = manager.remove_bookmark("user1", "post1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_user_bookmarks() {
        let (pool, _temp_dir) = setup_test_db().await;
        let manager = BookmarkManager::new(pool);

        // 複数のブックマークを追加
        manager.add_bookmark("user1", "post1").await.unwrap();
        manager.add_bookmark("user1", "post2").await.unwrap();
        manager.add_bookmark("user2", "post1").await.unwrap();

        // user1のブックマークを取得
        let bookmarks = manager.get_user_bookmarks("user1").await.unwrap();
        assert_eq!(bookmarks.len(), 2);
        
        // 新しい順にソートされているか確認
        assert!(bookmarks[0].created_at >= bookmarks[1].created_at);
    }

    #[tokio::test]
    async fn test_is_bookmarked() {
        let (pool, _temp_dir) = setup_test_db().await;
        let manager = BookmarkManager::new(pool);

        // ブックマークを追加
        manager.add_bookmark("user1", "post1").await.unwrap();

        // ブックマークされているか確認
        let is_bookmarked1 = manager.is_bookmarked("user1", "post1").await.unwrap();
        assert!(is_bookmarked1);

        // ブックマークされていないものを確認
        let is_bookmarked2 = manager.is_bookmarked("user1", "post2").await.unwrap();
        assert!(!is_bookmarked2);

        let is_bookmarked3 = manager.is_bookmarked("user2", "post1").await.unwrap();
        assert!(!is_bookmarked3);
    }

    #[tokio::test]
    async fn test_get_bookmarked_post_ids() {
        let (pool, _temp_dir) = setup_test_db().await;
        
        // 手動でcreated_atを制御するためにSQLを直接実行
        let base_time = chrono::Utc::now().timestamp_millis();
        
        // 最初のブックマーク（一番古い）
        sqlx::query(
            r#"
            INSERT INTO bookmarks (id, user_pubkey, post_id, created_at)
            VALUES (?1, ?2, ?3, ?4)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind("user1")
        .bind("post3")
        .bind(base_time - 200)
        .execute(&pool)
        .await
        .unwrap();
        
        // 2番目のブックマーク
        sqlx::query(
            r#"
            INSERT INTO bookmarks (id, user_pubkey, post_id, created_at)
            VALUES (?1, ?2, ?3, ?4)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind("user1")
        .bind("post1")
        .bind(base_time - 100)
        .execute(&pool)
        .await
        .unwrap();
        
        // 3番目のブックマーク（一番新しい）
        sqlx::query(
            r#"
            INSERT INTO bookmarks (id, user_pubkey, post_id, created_at)
            VALUES (?1, ?2, ?3, ?4)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind("user1")
        .bind("post2")
        .bind(base_time)
        .execute(&pool)
        .await
        .unwrap();
        
        let manager = BookmarkManager::new(pool);
        
        // 投稿IDのリストを取得
        let post_ids = manager.get_bookmarked_post_ids("user1").await.unwrap();
        assert_eq!(post_ids.len(), 3);
        
        // 新しい順にソートされているか確認
        assert_eq!(post_ids[0], "post2");
        assert_eq!(post_ids[1], "post1");
        assert_eq!(post_ids[2], "post3");
    }
}