use super::SqliteRepository;
use super::mapper::{map_joined_topic_row, map_topic_row};
use super::queries::{
    DELETE_TOPIC, DELETE_USER_TOPICS_BY_TOPIC, INSERT_TOPIC, MARK_TOPIC_LEFT, SELECT_ALL_TOPICS,
    SELECT_JOINED_TOPICS, SELECT_TOPIC_BY_ID, SELECT_TOPIC_MEMBER_COUNT, UPDATE_TOPIC,
    UPDATE_TOPIC_MEMBER_COUNT, UPDATE_TOPIC_STATS, UPSERT_USER_TOPIC,
};
use crate::application::ports::repositories::TopicRepository;
use crate::domain::constants::{DEFAULT_PUBLIC_TOPIC_ID, LEGACY_PUBLIC_TOPIC_ID};
use crate::domain::entities::Topic;
use crate::shared::error::AppError;
use async_trait::async_trait;
use chrono::Utc;
use sqlx::Row;

#[async_trait]
impl TopicRepository for SqliteRepository {
    async fn create_topic(&self, topic: &Topic) -> Result<(), AppError> {
        sqlx::query(INSERT_TOPIC)
            .bind(&topic.id)
            .bind(&topic.name)
            .bind(&topic.description)
            .bind(topic.created_at.timestamp_millis())
            .bind(topic.updated_at.timestamp_millis())
            .bind(topic.visibility.as_str())
            .execute(self.pool.get_pool())
            .await?;

        Ok(())
    }

    async fn get_topic(&self, id: &str) -> Result<Option<Topic>, AppError> {
        let row = sqlx::query(SELECT_TOPIC_BY_ID)
            .bind(id)
            .fetch_optional(self.pool.get_pool())
            .await?;

        match row {
            Some(row) => Ok(Some(map_topic_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn get_all_topics(&self) -> Result<Vec<Topic>, AppError> {
        let rows = sqlx::query(SELECT_ALL_TOPICS)
            .fetch_all(self.pool.get_pool())
            .await?;

        let mut topics = Vec::with_capacity(rows.len());
        for row in rows {
            let topic = map_topic_row(&row)?;
            topics.push(topic);
        }

        Ok(topics)
    }

    async fn get_joined_topics(&self, user_pubkey: &str) -> Result<Vec<Topic>, AppError> {
        let rows = sqlx::query(SELECT_JOINED_TOPICS)
            .bind(user_pubkey)
            .fetch_all(self.pool.get_pool())
            .await?;

        let mut topics = Vec::with_capacity(rows.len());
        for row in rows {
            let topic = map_joined_topic_row(&row)?;
            topics.push(topic);
        }

        Ok(topics)
    }

    async fn update_topic(&self, topic: &Topic) -> Result<(), AppError> {
        sqlx::query(UPDATE_TOPIC)
            .bind(&topic.name)
            .bind(&topic.description)
            .bind(topic.updated_at.timestamp_millis())
            .bind(&topic.id)
            .execute(self.pool.get_pool())
            .await?;

        Ok(())
    }

    async fn delete_topic(&self, id: &str) -> Result<(), AppError> {
        let normalized = if id == LEGACY_PUBLIC_TOPIC_ID {
            DEFAULT_PUBLIC_TOPIC_ID
        } else {
            id
        };

        if normalized == DEFAULT_PUBLIC_TOPIC_ID {
            return Err("デフォルトトピックは削除できません".into());
        }

        sqlx::query(DELETE_USER_TOPICS_BY_TOPIC)
            .bind(normalized)
            .execute(self.pool.get_pool())
            .await?;

        sqlx::query(DELETE_TOPIC)
            .bind(normalized)
            .execute(self.pool.get_pool())
            .await?;

        Ok(())
    }

    async fn join_topic(&self, topic_id: &str, user_pubkey: &str) -> Result<(), AppError> {
        let now = Utc::now().timestamp_millis();
        let mut tx = self.pool.get_pool().begin().await?;

        sqlx::query(UPSERT_USER_TOPIC)
            .bind(topic_id)
            .bind(user_pubkey)
            .bind(now)
            .execute(&mut *tx)
            .await?;

        let member_count: i64 = sqlx::query(SELECT_TOPIC_MEMBER_COUNT)
            .bind(topic_id)
            .fetch_one(&mut *tx)
            .await?
            .try_get("count")?;

        sqlx::query(UPDATE_TOPIC_MEMBER_COUNT)
            .bind(member_count)
            .bind(now)
            .bind(topic_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn leave_topic(&self, topic_id: &str, user_pubkey: &str) -> Result<(), AppError> {
        let normalized = if topic_id == LEGACY_PUBLIC_TOPIC_ID {
            DEFAULT_PUBLIC_TOPIC_ID
        } else {
            topic_id
        };

        if normalized == DEFAULT_PUBLIC_TOPIC_ID {
            return Err("デフォルトトピックから離脱することはできません".into());
        }

        let now = Utc::now().timestamp_millis();
        let mut tx = self.pool.get_pool().begin().await?;

        sqlx::query(MARK_TOPIC_LEFT)
            .bind(now)
            .bind(normalized)
            .bind(user_pubkey)
            .execute(&mut *tx)
            .await?;

        let member_count: i64 = sqlx::query(SELECT_TOPIC_MEMBER_COUNT)
            .bind(normalized)
            .fetch_one(&mut *tx)
            .await?
            .try_get("count")?;

        sqlx::query(UPDATE_TOPIC_MEMBER_COUNT)
            .bind(member_count)
            .bind(now)
            .bind(normalized)
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
        sqlx::query(UPDATE_TOPIC_STATS)
            .bind(member_count as i64)
            .bind(post_count as i64)
            .bind(Utc::now().timestamp_millis())
            .bind(id)
            .execute(self.pool.get_pool())
            .await?;

        Ok(())
    }
}
