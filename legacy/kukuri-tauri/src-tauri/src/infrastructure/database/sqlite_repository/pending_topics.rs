use super::SqliteRepository;
use super::queries::{
    DELETE_PENDING_TOPIC, INSERT_PENDING_TOPIC, SELECT_PENDING_TOPIC_BY_ID,
    SELECT_PENDING_TOPICS_BY_USER, UPDATE_PENDING_TOPIC_STATUS,
};
use crate::application::ports::repositories::PendingTopicRepository;
use crate::domain::entities::{PendingTopic, PendingTopicStatus};
use crate::shared::error::AppError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::Row;

fn map_pending_topic(row: &sqlx::sqlite::SqliteRow) -> Result<PendingTopic, AppError> {
    let created_at = DateTime::<Utc>::from_timestamp_millis(row.try_get::<i64, _>("created_at")?)
        .ok_or_else(|| AppError::Internal("Invalid created_at timestamp".into()))?;
    let updated_at = DateTime::<Utc>::from_timestamp_millis(row.try_get::<i64, _>("updated_at")?)
        .ok_or_else(|| AppError::Internal("Invalid updated_at timestamp".into()))?;
    let status_value: String = row.try_get("status")?;

    Ok(PendingTopic::new(
        row.try_get("pending_id")?,
        row.try_get("user_pubkey")?,
        row.try_get("name")?,
        row.try_get::<Option<String>, _>("description")?,
        PendingTopicStatus::from_value(status_value.as_str()),
        row.try_get("offline_action_id")?,
        row.try_get::<Option<String>, _>("synced_topic_id")?,
        row.try_get::<Option<String>, _>("error_message")?,
        created_at,
        updated_at,
    ))
}

#[async_trait]
impl PendingTopicRepository for SqliteRepository {
    async fn insert_pending_topic(&self, topic: &PendingTopic) -> Result<(), AppError> {
        sqlx::query(INSERT_PENDING_TOPIC)
            .bind(&topic.pending_id)
            .bind(&topic.user_pubkey)
            .bind(&topic.name)
            .bind(&topic.description)
            .bind(topic.status.as_str())
            .bind(&topic.offline_action_id)
            .bind(&topic.synced_topic_id)
            .bind(&topic.error_message)
            .bind(topic.created_at.timestamp_millis())
            .bind(topic.updated_at.timestamp_millis())
            .execute(self.pool.get_pool())
            .await?;
        Ok(())
    }

    async fn list_pending_topics(&self, user_pubkey: &str) -> Result<Vec<PendingTopic>, AppError> {
        let rows = sqlx::query(SELECT_PENDING_TOPICS_BY_USER)
            .bind(user_pubkey)
            .fetch_all(self.pool.get_pool())
            .await?;

        rows.iter().map(map_pending_topic).collect()
    }

    async fn get_pending_topic(&self, pending_id: &str) -> Result<Option<PendingTopic>, AppError> {
        let row = sqlx::query(SELECT_PENDING_TOPIC_BY_ID)
            .bind(pending_id)
            .fetch_optional(self.pool.get_pool())
            .await?;

        match row {
            Some(row) => map_pending_topic(&row).map(Some),
            None => Ok(None),
        }
    }

    async fn update_pending_topic_status(
        &self,
        pending_id: &str,
        status: PendingTopicStatus,
        synced_topic_id: Option<&str>,
        error_message: Option<&str>,
    ) -> Result<(), AppError> {
        let now = Utc::now().timestamp_millis();
        sqlx::query(UPDATE_PENDING_TOPIC_STATUS)
            .bind(pending_id)
            .bind(status.as_str())
            .bind(synced_topic_id)
            .bind(error_message)
            .bind(now)
            .execute(self.pool.get_pool())
            .await?;
        Ok(())
    }

    async fn delete_pending_topic(&self, pending_id: &str) -> Result<(), AppError> {
        sqlx::query(DELETE_PENDING_TOPIC)
            .bind(pending_id)
            .execute(self.pool.get_pool())
            .await?;
        Ok(())
    }
}
