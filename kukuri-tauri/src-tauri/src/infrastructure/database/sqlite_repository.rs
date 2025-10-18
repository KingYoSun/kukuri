use super::{
    ConnectionPool, EventRepository, PostRepository, Repository, TopicRepository, UserRepository,
};
use crate::domain::entities::{Event, Post, Topic, User};
use crate::shared::error::AppError;
use async_trait::async_trait;
use sqlx::Row;

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

#[async_trait]
impl PostRepository for SqliteRepository {
    async fn create_post(&self, post: &Post) -> Result<(), AppError> {
        // NostrイベントとしてDBに保存
        let tags_json = serde_json::to_string(&vec![vec!["t".to_string(), post.topic_id.clone()]])
            .unwrap_or_else(|_| "[]".to_string());

        sqlx::query(
            r#"
            INSERT INTO events (event_id, public_key, content, kind, tags, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&post.id)
        .bind(post.author.pubkey())
        .bind(&post.content)
        .bind(1) // Kind 1 for text notes
        .bind(&tags_json)
        .bind(post.created_at.timestamp_millis())
        .execute(self.pool.get_pool())
        .await?;

        Ok(())
    }

    async fn get_post(&self, id: &str) -> Result<Option<Post>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT event_id, public_key, content, created_at, tags
            FROM events
            WHERE event_id = ? AND kind = 1
            "#,
        )
        .bind(id)
        .fetch_optional(self.pool.get_pool())
        .await?;

        match row {
            Some(row) => {
                use sqlx::Row;
                let event_id: String = row.try_get("event_id")?;
                let public_key: String = row.try_get("public_key")?;
                let content: String = row.try_get("content")?;
                let created_at: i64 = row.try_get("created_at")?;
                let tags_json: String = row.try_get("tags").unwrap_or_default();

                // タグからトピックIDを抽出
                let mut topic_id = String::new();
                if let Ok(tags) = serde_json::from_str::<Vec<Vec<String>>>(&tags_json) {
                    for tag in tags {
                        if tag.len() >= 2 && tag[0] == "t" {
                            topic_id = tag[1].clone();
                            break;
                        }
                    }
                }

                let user = User::from_pubkey(&public_key);
                let post = Post::new_with_id(
                    event_id,
                    content,
                    user,
                    topic_id,
                    chrono::DateTime::from_timestamp_millis(created_at)
                        .unwrap_or_else(chrono::Utc::now),
                );

                Ok(Some(post))
            }
            None => Ok(None),
        }
    }

    async fn get_posts_by_topic(
        &self,
        topic_id: &str,
        limit: usize,
    ) -> Result<Vec<Post>, AppError> {
        let topic_tag = format!(r#"["t","{topic_id}"]"#);
        let rows = sqlx::query(
            r#"
            SELECT event_id, public_key, content, created_at, tags
            FROM events
            WHERE kind = 1
            AND tags LIKE '%' || ? || '%'
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(&topic_tag)
        .bind(limit as i64)
        .fetch_all(self.pool.get_pool())
        .await?;

        let mut posts = Vec::new();
        for row in rows {
            use sqlx::Row;
            let event_id: String = row.try_get("event_id")?;
            let public_key: String = row.try_get("public_key")?;
            let content: String = row.try_get("content")?;
            let created_at: i64 = row.try_get("created_at")?;

            let user = User::from_pubkey(&public_key);
            let post = Post::new_with_id(
                event_id,
                content,
                user,
                topic_id.to_string(),
                chrono::DateTime::from_timestamp_millis(created_at)
                    .unwrap_or_else(chrono::Utc::now),
            );
            posts.push(post);
        }

        Ok(posts)
    }

    async fn update_post(&self, post: &Post) -> Result<(), AppError> {
        // 投稿の更新（主にいいね数、ブースト数などのメタデータ更新用）
        sqlx::query(
            r#"
            UPDATE events 
            SET content = ?, updated_at = ?
            WHERE event_id = ?
            "#,
        )
        .bind(&post.content)
        .bind(chrono::Utc::now().timestamp_millis())
        .bind(&post.id)
        .execute(self.pool.get_pool())
        .await?;

        Ok(())
    }

    async fn delete_post(&self, id: &str) -> Result<(), AppError> {
        // Nostrでは削除イベント(Kind 5)を発行するが、元のイベントは残す
        // ここではフラグを立てるか、別テーブルで管理
        sqlx::query(
            r#"
            UPDATE events 
            SET deleted = 1, updated_at = ?
            WHERE event_id = ?
            "#,
        )
        .bind(chrono::Utc::now().timestamp_millis())
        .bind(id)
        .execute(self.pool.get_pool())
        .await?;

        Ok(())
    }

    async fn get_unsync_posts(&self) -> Result<Vec<Post>, AppError> {
        // sync_statusカラムがない場合は、オフライン中に作成されたものを取得
        let rows = sqlx::query(
            r#"
            SELECT event_id, public_key, content, created_at, tags
            FROM events
            WHERE kind = 1
            AND (sync_status IS NULL OR sync_status = 0)
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(self.pool.get_pool())
        .await?;

        let mut posts = Vec::new();
        for row in rows {
            use sqlx::Row;
            let event_id: String = row.try_get("event_id")?;
            let public_key: String = row.try_get("public_key")?;
            let content: String = row.try_get("content")?;
            let created_at: i64 = row.try_get("created_at")?;
            let tags_json: String = row.try_get("tags").unwrap_or_default();

            // タグからトピックIDを抽出
            let mut topic_id = String::new();
            if let Ok(tags) = serde_json::from_str::<Vec<Vec<String>>>(&tags_json) {
                for tag in tags {
                    if tag.len() >= 2 && tag[0] == "t" {
                        topic_id = tag[1].clone();
                        break;
                    }
                }
            }

            let user = User::from_pubkey(&public_key);
            let mut post = Post::new_with_id(
                event_id,
                content,
                user,
                topic_id,
                chrono::DateTime::from_timestamp_millis(created_at)
                    .unwrap_or_else(chrono::Utc::now),
            );
            post.mark_as_unsynced();
            posts.push(post);
        }

        Ok(posts)
    }

    async fn mark_post_synced(&self, id: &str, event_id: &str) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE events 
            SET sync_status = 1, sync_event_id = ?, synced_at = ?
            WHERE event_id = ?
            "#,
        )
        .bind(event_id)
        .bind(chrono::Utc::now().timestamp_millis())
        .bind(id)
        .execute(self.pool.get_pool())
        .await?;

        Ok(())
    }

    async fn get_posts_by_author(
        &self,
        author_pubkey: &str,
        limit: usize,
    ) -> Result<Vec<Post>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT event_id, public_key, content, created_at, tags
            FROM events
            WHERE kind = 1 AND public_key = ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(author_pubkey)
        .bind(limit as i64)
        .fetch_all(self.pool.get_pool())
        .await?;

        let mut posts = Vec::new();
        for row in rows {
            use sqlx::Row;
            let event_id: String = row.try_get("event_id")?;
            let public_key: String = row.try_get("public_key")?;
            let content: String = row.try_get("content")?;
            let created_at: i64 = row.try_get("created_at")?;
            let tags_json: String = row.try_get("tags").unwrap_or_default();

            // タグからトピックIDを抽出
            let mut topic_id = String::new();
            if let Ok(tags) = serde_json::from_str::<Vec<Vec<String>>>(&tags_json) {
                for tag in tags {
                    if tag.len() >= 2 && tag[0] == "t" {
                        topic_id = tag[1].clone();
                        break;
                    }
                }
            }

            let user = User::from_pubkey(&public_key);
            let post = Post::new_with_id(
                event_id,
                content,
                user,
                topic_id,
                chrono::DateTime::from_timestamp_millis(created_at)
                    .unwrap_or_else(chrono::Utc::now),
            );
            posts.push(post);
        }

        Ok(posts)
    }

    async fn get_recent_posts(&self, limit: usize) -> Result<Vec<Post>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT event_id, public_key, content, created_at, tags
            FROM events
            WHERE kind = 1
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit as i64)
        .fetch_all(self.pool.get_pool())
        .await?;

        let mut posts = Vec::new();
        for row in rows {
            use sqlx::Row;
            let event_id: String = row.try_get("event_id")?;
            let public_key: String = row.try_get("public_key")?;
            let content: String = row.try_get("content")?;
            let created_at: i64 = row.try_get("created_at")?;
            let tags_json: String = row.try_get("tags").unwrap_or_default();

            // タグからトピックIDを抽出
            let mut topic_id = String::new();
            if let Ok(tags) = serde_json::from_str::<Vec<Vec<String>>>(&tags_json) {
                for tag in tags {
                    if tag.len() >= 2 && tag[0] == "t" {
                        topic_id = tag[1].clone();
                        break;
                    }
                }
            }

            let user = User::from_pubkey(&public_key);
            let post = Post::new_with_id(
                event_id,
                content,
                user,
                topic_id,
                chrono::DateTime::from_timestamp_millis(created_at)
                    .unwrap_or_else(chrono::Utc::now),
            );
            posts.push(post);
        }

        Ok(posts)
    }
}

#[async_trait]
impl TopicRepository for SqliteRepository {
    async fn create_topic(&self, topic: &Topic) -> Result<(), AppError> {
        // トピックの作成
        sqlx::query(
            r#"
            INSERT INTO topics (topic_id, name, description, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&topic.id)
        .bind(&topic.name)
        .bind(&topic.description)
        .bind(topic.created_at.timestamp_millis())
        .bind(topic.updated_at.timestamp_millis())
        .execute(self.pool.get_pool())
        .await?;

        Ok(())
    }

    async fn get_topic(&self, id: &str) -> Result<Option<Topic>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT topic_id, name, description, created_at, updated_at, member_count, post_count
            FROM topics
            WHERE topic_id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(self.pool.get_pool())
        .await?;

        match row {
            Some(row) => {
                use sqlx::Row;
                let created_at =
                    chrono::DateTime::from_timestamp_millis(row.try_get("created_at")?)
                        .unwrap_or_else(chrono::Utc::now);
                let mut topic = Topic::new_with_id(
                    row.try_get("topic_id")?,
                    row.try_get::<String, _>("name")?,
                    row.try_get::<Option<String>, _>("description")?
                        .unwrap_or_default(),
                    created_at,
                );
                topic.updated_at =
                    chrono::DateTime::from_timestamp_millis(row.try_get("updated_at")?)
                        .unwrap_or(created_at);
                topic.member_count = row.try_get::<i64, _>("member_count")? as u32;
                topic.post_count = row.try_get::<i64, _>("post_count")? as u32;
                Ok(Some(topic))
            }
            None => Ok(None),
        }
    }

    async fn get_all_topics(&self) -> Result<Vec<Topic>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT topic_id, name, description, created_at, updated_at, member_count, post_count
            FROM topics
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(self.pool.get_pool())
        .await?;

        let mut topics = Vec::new();
        for row in rows {
            use sqlx::Row;
            let created_at = chrono::DateTime::from_timestamp_millis(row.try_get("created_at")?)
                .unwrap_or_else(chrono::Utc::now);
            let mut topic = Topic::new_with_id(
                row.try_get("topic_id")?,
                row.try_get::<String, _>("name")?,
                row.try_get::<Option<String>, _>("description")?
                    .unwrap_or_default(),
                created_at,
            );
            topic.updated_at = chrono::DateTime::from_timestamp_millis(row.try_get("updated_at")?)
                .unwrap_or(created_at);
            topic.member_count = row.try_get::<i64, _>("member_count")? as u32;
            topic.post_count = row.try_get::<i64, _>("post_count")? as u32;
            topics.push(topic);
        }

        Ok(topics)
    }

    async fn get_joined_topics(&self, user_pubkey: &str) -> Result<Vec<Topic>, AppError> {
        // ユーザーが参加しているトピックを取得
        let rows = sqlx::query(
            r#"
            SELECT t.topic_id, t.name, t.description, t.created_at, t.updated_at, t.member_count, t.post_count
            FROM topics t
            INNER JOIN user_topics ut ON t.topic_id = ut.topic_id
            WHERE ut.is_joined = 1 AND ut.user_pubkey = ?
            ORDER BY t.created_at ASC
            "#,
        )
        .bind(user_pubkey)
        .fetch_all(self.pool.get_pool())
        .await?;

        let mut topics = Vec::new();
        for row in rows {
            use sqlx::Row;
            let created_at = chrono::DateTime::from_timestamp_millis(row.try_get("created_at")?)
                .unwrap_or_else(chrono::Utc::now);
            let mut topic = Topic::new_with_id(
                row.try_get("topic_id")?,
                row.try_get::<String, _>("name")?,
                row.try_get::<Option<String>, _>("description")?
                    .unwrap_or_default(),
                created_at,
            );
            topic.updated_at = chrono::DateTime::from_timestamp_millis(row.try_get("updated_at")?)
                .unwrap_or(created_at);
            topic.member_count = row.try_get::<i64, _>("member_count")? as u32;
            topic.post_count = row.try_get::<i64, _>("post_count")? as u32;
            topic.is_joined = true;
            topics.push(topic);
        }

        Ok(topics)
    }

    async fn update_topic(&self, topic: &Topic) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE topics 
            SET name = ?, description = ?, updated_at = ?
            WHERE topic_id = ?
            "#,
        )
        .bind(&topic.name)
        .bind(&topic.description)
        .bind(topic.updated_at.timestamp_millis())
        .bind(&topic.id)
        .execute(self.pool.get_pool())
        .await?;

        Ok(())
    }

    async fn delete_topic(&self, id: &str) -> Result<(), AppError> {
        // #publicトピックは削除できない
        if id == "public" {
            return Err("デフォルトトピックは削除できません".into());
        }

        sqlx::query(
            r#"
            DELETE FROM user_topics
            WHERE topic_id = ?
            "#,
        )
        .bind(id)
        .execute(self.pool.get_pool())
        .await?;

        sqlx::query(
            r#"
            DELETE FROM topics 
            WHERE topic_id = ?
            "#,
        )
        .bind(id)
        .execute(self.pool.get_pool())
        .await?;

        Ok(())
    }

    async fn join_topic(&self, topic_id: &str, user_pubkey: &str) -> Result<(), AppError> {
        let now = chrono::Utc::now().timestamp_millis();
        let mut tx = self.pool.get_pool().begin().await?;

        sqlx::query(
            r#"
            INSERT INTO user_topics (topic_id, user_pubkey, is_joined, joined_at, left_at)
            VALUES (?1, ?2, 1, ?3, NULL)
            ON CONFLICT(topic_id, user_pubkey) DO UPDATE SET
                is_joined = 1,
                joined_at = excluded.joined_at,
                left_at = NULL
            "#,
        )
        .bind(topic_id)
        .bind(user_pubkey)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        let member_count: i64 = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM user_topics
            WHERE topic_id = ?1 AND is_joined = 1
            "#,
        )
        .bind(topic_id)
        .fetch_one(&mut *tx)
        .await?
        .try_get("count")?;

        sqlx::query(
            r#"
            UPDATE topics
            SET member_count = ?1, updated_at = ?2
            WHERE topic_id = ?3
            "#,
        )
        .bind(member_count)
        .bind(now)
        .bind(topic_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn leave_topic(&self, topic_id: &str, user_pubkey: &str) -> Result<(), AppError> {
        // #publicトピックからは離脱できない
        if topic_id == "public" {
            return Err("デフォルトトピックから離脱することはできません".into());
        }

        let now = chrono::Utc::now().timestamp_millis();
        let mut tx = self.pool.get_pool().begin().await?;

        sqlx::query(
            r#"
            UPDATE user_topics 
            SET is_joined = 0, left_at = ?1
            WHERE topic_id = ?2 AND user_pubkey = ?3
            "#,
        )
        .bind(now)
        .bind(topic_id)
        .bind(user_pubkey)
        .execute(&mut *tx)
        .await?;

        let member_count: i64 = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM user_topics
            WHERE topic_id = ?1 AND is_joined = 1
            "#,
        )
        .bind(topic_id)
        .fetch_one(&mut *tx)
        .await?
        .try_get("count")?;

        sqlx::query(
            r#"
            UPDATE topics
            SET member_count = ?1, updated_at = ?2
            WHERE topic_id = ?3
            "#,
        )
        .bind(member_count)
        .bind(now)
        .bind(topic_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn update_topic_stats(
        &self,
        id: &str,
        member_count: u32,
        post_count: u32,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE topics 
            SET member_count = ?, post_count = ?, updated_at = ?
            WHERE topic_id = ?
            "#,
        )
        .bind(member_count as i64)
        .bind(post_count as i64)
        .bind(chrono::Utc::now().timestamp_millis())
        .bind(id)
        .execute(self.pool.get_pool())
        .await?;

        Ok(())
    }
}

#[async_trait]
impl UserRepository for SqliteRepository {
    async fn create_user(&self, user: &User) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO users (npub, pubkey, display_name, bio, avatar_url, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(user.npub())
        .bind(user.pubkey())
        .bind(&user.profile.display_name)
        .bind(&user.profile.bio)
        .bind(&user.profile.avatar_url)
        .bind(user.created_at.timestamp_millis())
        .bind(user.updated_at.timestamp_millis())
        .execute(self.pool.get_pool())
        .await?;

        Ok(())
    }

    async fn get_user(&self, npub: &str) -> Result<Option<User>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT npub, pubkey, display_name, bio, avatar_url, created_at, updated_at
            FROM users
            WHERE npub = ?
            "#,
        )
        .bind(npub)
        .fetch_optional(self.pool.get_pool())
        .await?;

        match row {
            Some(row) => {
                use crate::domain::entities::UserProfile;
                use sqlx::Row;

                let user = User::new_with_profile(
                    row.try_get("npub")?,
                    UserProfile {
                        display_name: row.try_get("display_name").unwrap_or_default(),
                        bio: row.try_get("bio").unwrap_or_default(),
                        avatar_url: row.try_get("avatar_url").ok(),
                    },
                );
                Ok(Some(user))
            }
            None => Ok(None),
        }
    }

    async fn get_user_by_pubkey(&self, pubkey: &str) -> Result<Option<User>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT npub, pubkey, display_name, bio, avatar_url, created_at, updated_at
            FROM users
            WHERE pubkey = ?
            "#,
        )
        .bind(pubkey)
        .fetch_optional(self.pool.get_pool())
        .await?;

        match row {
            Some(row) => {
                use crate::domain::entities::UserProfile;
                use sqlx::Row;

                let user = User::new_with_profile(
                    row.try_get("npub")?,
                    UserProfile {
                        display_name: row.try_get("display_name").unwrap_or_default(),
                        bio: row.try_get("bio").unwrap_or_default(),
                        avatar_url: row.try_get("avatar_url").ok(),
                    },
                );
                Ok(Some(user))
            }
            None => Ok(None),
        }
    }

    async fn update_user(&self, user: &User) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE users 
            SET display_name = ?, bio = ?, avatar_url = ?, updated_at = ?
            WHERE npub = ?
            "#,
        )
        .bind(&user.profile.display_name)
        .bind(&user.profile.bio)
        .bind(&user.profile.avatar_url)
        .bind(user.updated_at.timestamp_millis())
        .bind(user.npub())
        .execute(self.pool.get_pool())
        .await?;

        Ok(())
    }

    async fn delete_user(&self, npub: &str) -> Result<(), AppError> {
        sqlx::query(
            r#"
            DELETE FROM users 
            WHERE npub = ?
            "#,
        )
        .bind(npub)
        .execute(self.pool.get_pool())
        .await?;

        Ok(())
    }

    async fn get_followers(&self, npub: &str) -> Result<Vec<User>, AppError> {
        // フォロワーを取得（followsテーブルから）
        let rows = sqlx::query(
            r#"
            SELECT u.npub, u.pubkey, u.display_name, u.bio, u.avatar_url, u.created_at, u.updated_at
            FROM users u
            INNER JOIN follows f ON u.pubkey = f.follower_pubkey
            WHERE f.followed_pubkey = (SELECT pubkey FROM users WHERE npub = ?)
            "#,
        )
        .bind(npub)
        .fetch_all(self.pool.get_pool())
        .await?;

        let mut users = Vec::new();
        for row in rows {
            use crate::domain::entities::UserProfile;
            use sqlx::Row;

            let user = User::new_with_profile(
                row.try_get("npub")?,
                UserProfile {
                    display_name: row.try_get("display_name").unwrap_or_default(),
                    bio: row.try_get("bio").unwrap_or_default(),
                    avatar_url: row.try_get("avatar_url").ok(),
                },
            );
            users.push(user);
        }

        Ok(users)
    }

    async fn get_following(&self, npub: &str) -> Result<Vec<User>, AppError> {
        // フォロー中のユーザーを取得
        let rows = sqlx::query(
            r#"
            SELECT u.npub, u.pubkey, u.display_name, u.bio, u.avatar_url, u.created_at, u.updated_at
            FROM users u
            INNER JOIN follows f ON u.pubkey = f.followed_pubkey
            WHERE f.follower_pubkey = (SELECT pubkey FROM users WHERE npub = ?)
            "#,
        )
        .bind(npub)
        .fetch_all(self.pool.get_pool())
        .await?;

        let mut users = Vec::new();
        for row in rows {
            use crate::domain::entities::UserProfile;
            use sqlx::Row;

            let user = User::new_with_profile(
                row.try_get("npub")?,
                UserProfile {
                    display_name: row.try_get("display_name").unwrap_or_default(),
                    bio: row.try_get("bio").unwrap_or_default(),
                    avatar_url: row.try_get("avatar_url").ok(),
                },
            );
            users.push(user);
        }

        Ok(users)
    }
}

#[async_trait]
impl EventRepository for SqliteRepository {
    async fn create_event(&self, event: &Event) -> Result<(), AppError> {
        let tags_json = serde_json::to_string(&event.tags).unwrap_or_else(|_| "[]".to_string());

        sqlx::query(
            r#"
            INSERT INTO events (event_id, public_key, content, kind, tags, created_at, sig)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(event.id.to_string())
        .bind(&event.pubkey)
        .bind(&event.content)
        .bind(event.kind as i64)
        .bind(&tags_json)
        .bind(event.created_at.timestamp_millis())
        .bind(&event.sig)
        .execute(self.pool.get_pool())
        .await?;
        // タグからトピックIDを抽出してマッピング登録（冪等）
        // ルール: ["topic", <id>] または ["t", <id>] の2要素タグを優先採用
        for tag in &event.tags {
            if tag.len() >= 2 {
                let key = tag[0].to_lowercase();
                if (key == "topic" || key == "t") && !tag[1].is_empty() {
                    let _ = self.add_event_topic(&event.id, &tag[1]).await;
                }
            }
        }

        Ok(())
    }

    async fn get_event(&self, id: &str) -> Result<Option<Event>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT event_id, public_key, content, kind, tags, created_at, sig
            FROM events
            WHERE event_id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(self.pool.get_pool())
        .await?;

        match row {
            Some(row) => {
                use crate::domain::value_objects::EventId;
                use sqlx::Row;

                let event_id = EventId::from_hex(row.try_get::<String, _>("event_id")?.as_str())?;
                let tags_json: String = row.try_get("tags").unwrap_or_default();
                let tags: Vec<Vec<String>> = serde_json::from_str(&tags_json).unwrap_or_default();

                let event = Event::new_with_id(
                    event_id,
                    row.try_get("public_key")?,
                    row.try_get("content")?,
                    row.try_get::<i64, _>("kind")? as u32,
                    tags,
                    chrono::DateTime::from_timestamp_millis(row.try_get("created_at")?)
                        .unwrap_or_else(chrono::Utc::now),
                    row.try_get("sig")?,
                );

                Ok(Some(event))
            }
            None => Ok(None),
        }
    }

    async fn get_events_by_kind(&self, kind: u32, limit: usize) -> Result<Vec<Event>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT event_id, public_key, content, kind, tags, created_at, sig
            FROM events
            WHERE kind = ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(kind as i64)
        .bind(limit as i64)
        .fetch_all(self.pool.get_pool())
        .await?;

        let mut events = Vec::new();
        for row in rows {
            use crate::domain::value_objects::EventId;
            use sqlx::Row;

            let event_id = EventId::from_hex(row.try_get::<String, _>("event_id")?.as_str())?;
            let tags_json: String = row.try_get("tags").unwrap_or_default();
            let tags: Vec<Vec<String>> = serde_json::from_str(&tags_json).unwrap_or_default();

            let event = Event::new_with_id(
                event_id,
                row.try_get("public_key")?,
                row.try_get("content")?,
                row.try_get::<i64, _>("kind")? as u32,
                tags,
                chrono::DateTime::from_timestamp_millis(row.try_get("created_at")?)
                    .unwrap_or_else(chrono::Utc::now),
                row.try_get("sig")?,
            );
            events.push(event);
        }

        Ok(events)
    }

    async fn get_events_by_author(
        &self,
        pubkey: &str,
        limit: usize,
    ) -> Result<Vec<Event>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT event_id, public_key, content, kind, tags, created_at, sig
            FROM events
            WHERE public_key = ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(pubkey)
        .bind(limit as i64)
        .fetch_all(self.pool.get_pool())
        .await?;

        let mut events = Vec::new();
        for row in rows {
            use crate::domain::value_objects::EventId;
            use sqlx::Row;

            let event_id = EventId::from_hex(row.try_get::<String, _>("event_id")?.as_str())?;
            let tags_json: String = row.try_get("tags").unwrap_or_default();
            let tags: Vec<Vec<String>> = serde_json::from_str(&tags_json).unwrap_or_default();

            let event = Event::new_with_id(
                event_id,
                row.try_get("public_key")?,
                row.try_get("content")?,
                row.try_get::<i64, _>("kind")? as u32,
                tags,
                chrono::DateTime::from_timestamp_millis(row.try_get("created_at")?)
                    .unwrap_or_else(chrono::Utc::now),
                row.try_get("sig")?,
            );
            events.push(event);
        }

        Ok(events)
    }

    async fn delete_event(&self, id: &str) -> Result<(), AppError> {
        // Nostrでは削除イベント(Kind 5)を発行するが、元のイベントは残す
        sqlx::query(
            r#"
            UPDATE events 
            SET deleted = 1, updated_at = ?
            WHERE event_id = ?
            "#,
        )
        .bind(chrono::Utc::now().timestamp_millis())
        .bind(id)
        .execute(self.pool.get_pool())
        .await?;

        Ok(())
    }

    async fn get_unsync_events(&self) -> Result<Vec<Event>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT event_id, public_key, content, kind, tags, created_at, sig
            FROM events
            WHERE sync_status IS NULL OR sync_status = 0
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(self.pool.get_pool())
        .await?;

        let mut events = Vec::new();
        for row in rows {
            use crate::domain::value_objects::EventId;
            use sqlx::Row;

            let event_id = EventId::from_hex(row.try_get::<String, _>("event_id")?.as_str())?;
            let tags_json: String = row.try_get("tags").unwrap_or_default();
            let tags: Vec<Vec<String>> = serde_json::from_str(&tags_json).unwrap_or_default();

            let event = Event::new_with_id(
                event_id,
                row.try_get("public_key")?,
                row.try_get("content")?,
                row.try_get::<i64, _>("kind")? as u32,
                tags,
                chrono::DateTime::from_timestamp_millis(row.try_get("created_at")?)
                    .unwrap_or_else(chrono::Utc::now),
                row.try_get("sig")?,
            );
            events.push(event);
        }

        Ok(events)
    }

    async fn mark_event_synced(&self, id: &str) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE events 
            SET sync_status = 1, synced_at = ?
            WHERE event_id = ?
            "#,
        )
        .bind(chrono::Utc::now().timestamp_millis())
        .bind(id)
        .execute(self.pool.get_pool())
        .await?;

        Ok(())
    }

    async fn add_event_topic(&self, event_id: &str, topic_id: &str) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO event_topics (event_id, topic_id, created_at)
            VALUES (?1, ?2, ?3)
            "#,
        )
        .bind(event_id)
        .bind(topic_id)
        .bind(chrono::Utc::now().timestamp_millis())
        .execute(self.pool.get_pool())
        .await?;
        Ok(())
    }

    async fn get_event_topics(&self, event_id: &str) -> Result<Vec<String>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT topic_id FROM event_topics WHERE event_id = ?1
            "#,
        )
        .bind(event_id)
        .fetch_all(self.pool.get_pool())
        .await?;

        let mut topics = Vec::new();
        for row in rows {
            use sqlx::Row;
            topics.push(row.try_get::<String, _>("topic_id")?);
        }
        Ok(topics)
    }
}
