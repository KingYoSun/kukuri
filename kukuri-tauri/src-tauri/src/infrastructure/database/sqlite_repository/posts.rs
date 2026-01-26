use super::SqliteRepository;
use super::mapper::map_post_row;
use super::queries::{
    INSERT_POST_EVENT, MARK_POST_DELETED, MARK_POST_SYNCED, SELECT_POST_BY_ID,
    SELECT_POSTS_BY_AUTHOR, SELECT_POSTS_BY_TOPIC, SELECT_RECENT_POSTS, SELECT_UNSYNC_POSTS,
    UPDATE_POST_CONTENT,
};
use crate::application::ports::repositories::{PostFeedCursor, PostFeedPage, PostRepository};
use crate::domain::entities::Post;
use crate::shared::error::AppError;
use async_trait::async_trait;
use chrono::Utc;
use sqlx::{QueryBuilder, Sqlite};

fn serialize_topic_tags(post: &Post) -> String {
    let mut tags = vec![vec!["t".to_string(), post.topic_id.clone()]];
    if let Some(scope) = post.scope.as_ref() {
        tags.push(vec!["scope".to_string(), scope.clone()]);
    }
    if let Some(epoch) = post.epoch {
        tags.push(vec!["epoch".to_string(), epoch.to_string()]);
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

        sqlx::query(INSERT_POST_EVENT)
            .bind(&post.id)
            .bind(post.author.pubkey())
            .bind(&post.content)
            .bind(1)
            .bind(&tags_json)
            .bind(post.created_at.timestamp_millis())
            .execute(self.pool.get_pool())
            .await?;

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
            "SELECT e.event_id, e.public_key, e.content, e.tags, e.created_at \
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
