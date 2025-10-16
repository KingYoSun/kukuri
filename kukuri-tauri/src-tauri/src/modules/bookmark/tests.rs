#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use super::super::BookmarkManager;
    use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
    use uuid::Uuid;

    async fn setup_test_db() -> (SqlitePool, Option<()>) {
        // メモリ内SQLiteデータベースを使用（Docker環境での権限問題を回避）
        let db_url = "sqlite::memory:";

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(db_url)
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

        (pool, None)
    }

    #[tokio::test]
    async fn test_add_bookmark() {
        let (pool, _) = setup_test_db().await;
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
        let (pool, _) = setup_test_db().await;
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
        let (pool, _) = setup_test_db().await;
        let manager = BookmarkManager::new(pool);

        // ブックマークを追加
        manager.add_bookmark("user1", "post1").await.unwrap();

        // ブックマークを削除
        let result = manager.remove_bookmark("user1", "post1").await;
        assert!(result.is_ok());

        // 削除は成功するが、削除確認の方法は get_bookmarked_post_ids を使用
    }

    #[tokio::test]
    async fn test_remove_nonexistent_bookmark() {
        let (pool, _) = setup_test_db().await;
        let manager = BookmarkManager::new(pool);

        // 存在しないブックマークを削除（エラーにはならない）
        let result = manager.remove_bookmark("user1", "post1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_bookmarked_post_ids() {
        let (pool, _) = setup_test_db().await;

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
