use super::SqliteRepository;
use super::mapper::map_event_row;
use super::queries::{
    INSERT_EVENT, INSERT_EVENT_TOPIC, MARK_EVENT_DELETED, MARK_EVENT_SYNCED, SELECT_EVENT_BY_ID,
    SELECT_EVENT_TOPICS, SELECT_EVENTS_BY_AUTHOR, SELECT_EVENTS_BY_KIND, SELECT_UNSYNC_EVENTS,
};
use crate::application::ports::repositories::EventRepository;
use crate::domain::entities::Event;
use crate::shared::error::AppError;
use async_trait::async_trait;
use chrono::Utc;
use sqlx::Row;

#[async_trait]
impl EventRepository for SqliteRepository {
    async fn create_event(&self, event: &Event) -> Result<(), AppError> {
        let tags_json = serde_json::to_string(&event.tags).unwrap_or_else(|_| "[]".to_string());

        sqlx::query(INSERT_EVENT)
            .bind(event.id.to_string())
            .bind(&event.pubkey)
            .bind(&event.content)
            .bind(event.kind as i64)
            .bind(&tags_json)
            .bind(event.created_at.timestamp_millis())
            .bind(&event.sig)
            .execute(self.pool.get_pool())
            .await?;

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
        let row = sqlx::query(SELECT_EVENT_BY_ID)
            .bind(id)
            .fetch_optional(self.pool.get_pool())
            .await?;

        match row {
            Some(row) => Ok(Some(map_event_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn get_events_by_kind(&self, kind: u32, limit: usize) -> Result<Vec<Event>, AppError> {
        let rows = sqlx::query(SELECT_EVENTS_BY_KIND)
            .bind(kind as i64)
            .bind(limit as i64)
            .fetch_all(self.pool.get_pool())
            .await?;

        let mut events = Vec::with_capacity(rows.len());
        for row in rows {
            let event = map_event_row(&row)?;
            events.push(event);
        }

        Ok(events)
    }

    async fn get_events_by_author(
        &self,
        pubkey: &str,
        limit: usize,
    ) -> Result<Vec<Event>, AppError> {
        let rows = sqlx::query(SELECT_EVENTS_BY_AUTHOR)
            .bind(pubkey)
            .bind(limit as i64)
            .fetch_all(self.pool.get_pool())
            .await?;

        let mut events = Vec::with_capacity(rows.len());
        for row in rows {
            let event = map_event_row(&row)?;
            events.push(event);
        }

        Ok(events)
    }

    async fn delete_event(&self, id: &str) -> Result<(), AppError> {
        sqlx::query(MARK_EVENT_DELETED)
            .bind(Utc::now().timestamp_millis())
            .bind(id)
            .execute(self.pool.get_pool())
            .await?;

        Ok(())
    }

    async fn get_unsync_events(&self) -> Result<Vec<Event>, AppError> {
        let rows = sqlx::query(SELECT_UNSYNC_EVENTS)
            .fetch_all(self.pool.get_pool())
            .await?;

        let mut events = Vec::with_capacity(rows.len());
        for row in rows {
            let event = map_event_row(&row)?;
            events.push(event);
        }

        Ok(events)
    }

    async fn mark_event_synced(&self, id: &str) -> Result<(), AppError> {
        sqlx::query(MARK_EVENT_SYNCED)
            .bind(Utc::now().timestamp_millis())
            .bind(id)
            .execute(self.pool.get_pool())
            .await?;

        Ok(())
    }

    async fn add_event_topic(&self, event_id: &str, topic_id: &str) -> Result<(), AppError> {
        sqlx::query(INSERT_EVENT_TOPIC)
            .bind(event_id)
            .bind(topic_id)
            .bind(Utc::now().timestamp_millis())
            .execute(self.pool.get_pool())
            .await?;
        Ok(())
    }

    async fn get_event_topics(&self, event_id: &str) -> Result<Vec<String>, AppError> {
        let rows = sqlx::query(SELECT_EVENT_TOPICS)
            .bind(event_id)
            .fetch_all(self.pool.get_pool())
            .await?;

        let mut topics = Vec::with_capacity(rows.len());
        for row in rows {
            topics.push(row.try_get::<String, _>("topic_id")?);
        }

        Ok(topics)
    }
}
