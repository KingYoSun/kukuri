use super::SqliteRepository;
use super::mapper::map_post_row;
use super::queries::{
    INSERT_POST_EVENT, MARK_POST_DELETED, MARK_POST_SYNCED, SELECT_EVENT_THREAD_BY_EVENT,
    SELECT_POST_BY_ID, SELECT_POSTS_BY_AUTHOR, SELECT_POSTS_BY_THREAD, SELECT_POSTS_BY_TOPIC,
    SELECT_RECENT_POSTS, SELECT_SYNC_EVENT_ID_BY_EVENT, SELECT_TOPIC_TIMELINE_SUMMARIES,
    SELECT_UNSYNC_POSTS, UPDATE_POST_CONTENT, UPSERT_EVENT_THREAD,
};
use crate::application::ports::repositories::{
    EventThreadRecord, PostFeedCursor, PostFeedPage, PostRepository, TopicTimelineSummaryRecord,
};
use crate::domain::entities::Post;
use crate::shared::error::AppError;
use async_trait::async_trait;
use chrono::Utc;
use sqlx::{QueryBuilder, Row, Sqlite};

fn serialize_topic_tags(post: &Post) -> String {
    let mut tags = vec![vec!["t".to_string(), post.topic_id.clone()]];
    if let Some(scope) = post.scope.as_ref() {
        tags.push(vec!["scope".to_string(), scope.clone()]);
    }
    if let Some(epoch) = post.epoch {
        tags.push(vec!["epoch".to_string(), epoch.to_string()]);
    }
    if let Some(thread_namespace) = post.thread_namespace.as_ref() {
        tags.push(vec!["thread".to_string(), thread_namespace.clone()]);
    }
    if let Some(thread_uuid) = post.thread_uuid.as_ref() {
        tags.push(vec!["thread_uuid".to_string(), thread_uuid.clone()]);
    }
    if let Some(thread_root_event_id) = post.thread_root_event_id.as_ref() {
        tags.push(vec![
            "thread_root_event_id".to_string(),
            thread_root_event_id.clone(),
        ]);
    }
    if let Some(thread_parent_event_id) = post.thread_parent_event_id.as_ref() {
        tags.push(vec![
            "thread_parent_event_id".to_string(),
            thread_parent_event_id.clone(),
        ]);
    }
    serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string())
}

fn topic_tag_like(topic_id: &str) -> String {
    format!(r#"["t","{topic_id}"]"#)
}

#[async_trait]
impl PostRepository for SqliteRepository {
    async fn create_post(&self, post: &Post) -> Result<(), AppError> {
        let tags_json = serialize_topic_tags(post);
        let created_at = post.created_at.timestamp_millis();

        sqlx::query(INSERT_POST_EVENT)
            .bind(&post.id)
            .bind(post.author.pubkey())
            .bind(&post.content)
            .bind(1)
            .bind(&tags_json)
            .bind(created_at)
            .execute(self.pool.get_pool())
            .await?;

        if let (Some(thread_namespace), Some(thread_uuid), Some(root_event_id)) = (
            post.thread_namespace.as_deref(),
            post.thread_uuid.as_deref(),
            post.thread_root_event_id.as_deref(),
        ) {
            sqlx::query(UPSERT_EVENT_THREAD)
                .bind(&post.id)
                .bind(&post.topic_id)
                .bind(thread_namespace)
                .bind(thread_uuid)
                .bind(root_event_id)
                .bind(post.thread_parent_event_id.as_deref())
                .bind(created_at)
                .execute(self.pool.get_pool())
                .await?;
        }

        Ok(())
    }

    async fn get_post(&self, id: &str) -> Result<Option<Post>, AppError> {
        let row = sqlx::query(SELECT_POST_BY_ID)
            .bind(id)
            .fetch_optional(self.pool.get_pool())
            .await?;

        match row {
            Some(row) => Ok(Some(map_post_row(&row, None)?)),
            None => Ok(None),
        }
    }

    async fn get_posts_by_topic(
        &self,
        topic_id: &str,
        limit: usize,
    ) -> Result<Vec<Post>, AppError> {
        let rows = sqlx::query(SELECT_POSTS_BY_TOPIC)
            .bind(topic_tag_like(topic_id))
            .bind(limit as i64)
            .fetch_all(self.pool.get_pool())
            .await?;

        let mut posts = Vec::with_capacity(rows.len());
        for row in rows {
            let post = map_post_row(&row, Some(topic_id))?;
            posts.push(post);
        }

        Ok(posts)
    }

    async fn get_topic_timeline(
        &self,
        topic_id: &str,
        limit: usize,
    ) -> Result<Vec<TopicTimelineSummaryRecord>, AppError> {
        let rows = sqlx::query(SELECT_TOPIC_TIMELINE_SUMMARIES)
            .bind(topic_id)
            .bind(limit as i64)
            .fetch_all(self.pool.get_pool())
            .await?;

        let mut summaries = Vec::with_capacity(rows.len());
        for row in rows {
            let reply_count_raw: i64 = row.try_get("reply_count")?;
            summaries.push(TopicTimelineSummaryRecord {
                thread_uuid: row.try_get("thread_uuid")?,
                root_event_id: row.try_get("root_event_id")?,
                first_reply_event_id: row.try_get("first_reply_event_id")?,
                reply_count: u32::try_from(reply_count_raw.max(0)).unwrap_or(u32::MAX),
                last_activity_at: row.try_get("last_activity_at")?,
            });
        }

        Ok(summaries)
    }

    async fn get_posts_by_thread(
        &self,
        topic_id: &str,
        thread_uuid: &str,
        limit: usize,
    ) -> Result<Vec<Post>, AppError> {
        let rows = sqlx::query(SELECT_POSTS_BY_THREAD)
            .bind(topic_id)
            .bind(thread_uuid)
            .bind(limit as i64)
            .fetch_all(self.pool.get_pool())
            .await?;

        let mut posts = Vec::with_capacity(rows.len());
        for row in rows {
            let post = map_post_row(&row, Some(topic_id))?;
            posts.push(post);
        }

        Ok(posts)
    }

    async fn get_event_thread(
        &self,
        topic_id: &str,
        event_id: &str,
    ) -> Result<Option<EventThreadRecord>, AppError> {
        let row = sqlx::query(SELECT_EVENT_THREAD_BY_EVENT)
            .bind(topic_id)
            .bind(event_id)
            .fetch_optional(self.pool.get_pool())
            .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(EventThreadRecord {
            event_id: row.try_get("event_id")?,
            topic_id: row.try_get("topic_id")?,
            thread_namespace: row.try_get("thread_namespace")?,
            thread_uuid: row.try_get("thread_uuid")?,
            root_event_id: row.try_get("root_event_id")?,
            parent_event_id: row.try_get("parent_event_id")?,
        }))
    }

    async fn get_sync_event_id(&self, event_id: &str) -> Result<Option<String>, AppError> {
        let row = sqlx::query(SELECT_SYNC_EVENT_ID_BY_EVENT)
            .bind(event_id)
            .fetch_optional(self.pool.get_pool())
            .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        Ok(row.try_get::<Option<String>, _>("sync_event_id")?)
    }

    async fn update_post(&self, post: &Post) -> Result<(), AppError> {
        sqlx::query(UPDATE_POST_CONTENT)
            .bind(&post.content)
            .bind(Utc::now().timestamp_millis())
            .bind(&post.id)
            .execute(self.pool.get_pool())
            .await?;

        Ok(())
    }

    async fn delete_post(&self, id: &str) -> Result<(), AppError> {
        sqlx::query(MARK_POST_DELETED)
            .bind(Utc::now().timestamp_millis())
            .bind(id)
            .execute(self.pool.get_pool())
            .await?;

        Ok(())
    }

    async fn get_unsync_posts(&self) -> Result<Vec<Post>, AppError> {
        let rows = sqlx::query(SELECT_UNSYNC_POSTS)
            .fetch_all(self.pool.get_pool())
            .await?;

        let mut posts = Vec::with_capacity(rows.len());
        for row in rows {
            let mut post = map_post_row(&row, None)?;
            post.mark_as_unsynced();
            posts.push(post);
        }

        Ok(posts)
    }

    async fn mark_post_synced(&self, id: &str, event_id: &str) -> Result<(), AppError> {
        sqlx::query(MARK_POST_SYNCED)
            .bind(event_id)
            .bind(Utc::now().timestamp_millis())
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
        let rows = sqlx::query(SELECT_POSTS_BY_AUTHOR)
            .bind(author_pubkey)
            .bind(limit as i64)
            .fetch_all(self.pool.get_pool())
            .await?;

        let mut posts = Vec::with_capacity(rows.len());
        for row in rows {
            let post = map_post_row(&row, None)?;
            posts.push(post);
        }

        Ok(posts)
    }

    async fn get_recent_posts(&self, limit: usize) -> Result<Vec<Post>, AppError> {
        let rows = sqlx::query(SELECT_RECENT_POSTS)
            .bind(limit as i64)
            .fetch_all(self.pool.get_pool())
            .await?;

        let mut posts = Vec::with_capacity(rows.len());
        for row in rows {
            let post = map_post_row(&row, None)?;
            posts.push(post);
        }

        Ok(posts)
    }

    async fn list_following_feed(
        &self,
        follower_pubkey: &str,
        cursor: Option<PostFeedCursor>,
        limit: usize,
    ) -> Result<PostFeedPage, AppError> {
        let limit = limit.clamp(1, 100);
        let fetch_limit = limit + 1;

        let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(
            "SELECT e.event_id, e.public_key, e.content, e.tags, e.created_at, e.sync_status, e.sync_event_id \
             FROM events e \
             INNER JOIN follows f ON f.followed_pubkey = e.public_key \
             WHERE f.follower_pubkey = ",
        );
        builder.push_bind(follower_pubkey);
        builder.push(" AND e.kind = 1 AND e.deleted = 0");

        if let Some(cursor) = cursor {
            builder.push(" AND (e.created_at < ");
            builder.push_bind(cursor.created_at);
            builder.push(" OR (e.created_at = ");
            builder.push_bind(cursor.created_at);
            builder.push(" AND e.event_id < ");
            builder.push_bind(cursor.event_id);
            builder.push("))");
        }

        builder.push(" ORDER BY e.created_at DESC, e.event_id DESC LIMIT ");
        builder.push_bind(fetch_limit as i64);

        let rows = builder.build().fetch_all(self.pool.get_pool()).await?;

        let mut rows_iter = rows.into_iter();
        let mut posts = Vec::with_capacity(limit.min(fetch_limit));

        for _ in 0..limit {
            if let Some(row) = rows_iter.next() {
                let post = map_post_row(&row, None)?;
                posts.push(post);
            } else {
                break;
            }
        }

        let has_more = rows_iter.next().is_some();
        let next_cursor = if has_more {
            posts.last().map(|post| {
                PostFeedCursor {
                    created_at: post.created_at.timestamp_millis(),
                    event_id: post.id.clone(),
                }
                .to_string()
            })
        } else {
            None
        };

        Ok(PostFeedPage {
            items: posts,
            next_cursor,
            has_more,
        })
    }
}
