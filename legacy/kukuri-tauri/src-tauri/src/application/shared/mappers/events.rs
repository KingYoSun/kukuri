use crate::domain::entities::Event;
use crate::domain::value_objects::EventId;
use crate::shared::error::AppError;
use chrono::{DateTime, Utc};
use sqlx::{Row, sqlite::SqliteRow};

pub(crate) fn map_event_row(row: &SqliteRow) -> Result<Event, AppError> {
    let event_id_hex: String = row.try_get("event_id")?;
    let event_id = EventId::from_hex(event_id_hex.as_str())?;
    let kind = row.try_get::<i64, _>("kind")? as u32;
    let created_at =
        DateTime::from_timestamp_millis(row.try_get("created_at")?).unwrap_or_else(Utc::now);
    let tags_json: String = row.try_get("tags").unwrap_or_default();
    let tags = parse_event_tags(&tags_json);

    Ok(Event::new_with_id(
        event_id,
        row.try_get("public_key")?,
        row.try_get("content")?,
        kind,
        tags,
        created_at,
        row.try_get("sig")?,
    ))
}

pub(crate) fn parse_event_tags(tags_json: &str) -> Vec<Vec<String>> {
    serde_json::from_str(tags_json).unwrap_or_default()
}
