use crate::domain::entities::{Post, User};
use crate::shared::error::AppError;
use chrono::{DateTime, Utc};
use sqlx::{Row, sqlite::SqliteRow};

pub(crate) fn map_post_row(
    row: &SqliteRow,
    fallback_topic: Option<&str>,
) -> Result<Post, AppError> {
    let event_id: String = row.try_get("event_id")?;
    let public_key: String = row.try_get("public_key")?;
    let content: String = row.try_get("content")?;
    let created_at: i64 = row.try_get("created_at")?;
    let tags_json: String = row.try_get("tags").unwrap_or_default();

    let topic_id = fallback_topic
        .map(|id| id.to_string())
        .or_else(|| extract_topic_from_tags(&tags_json))
        .unwrap_or_default();

    let user = User::from_pubkey(&public_key);
    let created_at = DateTime::from_timestamp_millis(created_at).unwrap_or_else(Utc::now);

    Ok(Post::new_with_id(
        event_id, content, user, topic_id, created_at,
    ))
}

pub(crate) fn extract_topic_from_tags(tags_json: &str) -> Option<String> {
    let tags = serde_json::from_str::<Vec<Vec<String>>>(tags_json).ok()?;
    tags.into_iter().find_map(|tag| match tag.as_slice() {
        [key, value, ..] if key == "t" => Some(value.clone()),
        _ => None,
    })
}
