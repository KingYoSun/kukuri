use super::SqliteRepository;
use super::mapper::map_event_row;
use super::queries::{
    INSERT_EVENT, INSERT_EVENT_TOPIC, MARK_EVENT_DELETED, MARK_EVENT_SYNCED, SELECT_EVENT_BY_ID,
    SELECT_EVENT_THREAD_BY_EVENT, SELECT_EVENT_TOPICS, SELECT_EVENTS_BY_AUTHOR,
    SELECT_EVENTS_BY_KIND, SELECT_UNSYNC_EVENTS, UPSERT_EVENT_THREAD,
};
use crate::application::ports::repositories::EventRepository;
use crate::domain::entities::Event;
use crate::shared::error::AppError;
use async_trait::async_trait;
use chrono::Utc;
use sqlx::Row;

fn find_tag_value(tags: &[Vec<String>], key: &str) -> Option<String> {
    tags.iter()
        .find(|tag| tag.len() >= 2 && tag[0].eq_ignore_ascii_case(key))
        .map(|tag| tag[1].clone())
}

fn find_tag_value_with_marker(tags: &[Vec<String>], key: &str, marker: &str) -> Option<String> {
    tags.iter()
        .find(|tag| {
            tag.len() >= 2
                && tag[0].eq_ignore_ascii_case(key)
                && tag.get(3).map(|value| value.as_str()) == Some(marker)
        })
        .map(|tag| tag[1].clone())
}

fn extract_topic_id(tags: &[Vec<String>]) -> Option<String> {
    find_tag_value(tags, "topic").or_else(|| find_tag_value(tags, "t"))
}

fn extract_thread_uuid(tags: &[Vec<String>]) -> Option<String> {
    find_tag_value(tags, "thread_uuid")
}

fn extract_thread_namespace(tags: &[Vec<String>]) -> Option<String> {
    find_tag_value(tags, "thread")
}

fn extract_parent_event_id(tags: &[Vec<String>]) -> Option<String> {
    find_tag_value(tags, "thread_parent_event_id")
        .or_else(|| find_tag_value(tags, "reply"))
        .or_else(|| find_tag_value_with_marker(tags, "e", "reply"))
}

fn extract_root_event_id(tags: &[Vec<String>]) -> Option<String> {
    find_tag_value(tags, "thread_root_event_id")
        .or_else(|| find_tag_value_with_marker(tags, "e", "root"))
}

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

        if let (Some(topic_id), Some(thread_uuid)) = (
            extract_topic_id(&event.tags),
            extract_thread_uuid(&event.tags),
        ) {
            let thread_namespace = extract_thread_namespace(&event.tags)
                .unwrap_or_else(|| format!("{topic_id}/threads/{thread_uuid}"));
            let parent_event_id = extract_parent_event_id(&event.tags);
            let root_from_tag = extract_root_event_id(&event.tags);

            let root_event_id = if let Some(root_event_id) = root_from_tag {
                root_event_id
            } else if let Some(parent_event_id) = parent_event_id.as_deref() {
                let parent = sqlx::query(SELECT_EVENT_THREAD_BY_EVENT)
                    .bind(&topic_id)
                    .bind(parent_event_id)
                    .fetch_optional(self.pool.get_pool())
                    .await?;
                parent
                    .and_then(|row| row.try_get::<String, _>("root_event_id").ok())
                    .unwrap_or_else(|| parent_event_id.to_string())
            } else {
                event.id.clone()
            };

            sqlx::query(UPSERT_EVENT_THREAD)
                .bind(&event.id)
                .bind(topic_id)
                .bind(thread_namespace)
                .bind(thread_uuid)
                .bind(root_event_id)
                .bind(parent_event_id)
                .bind(event.created_at.timestamp_millis())
                .execute(self.pool.get_pool())
                .await?;
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
