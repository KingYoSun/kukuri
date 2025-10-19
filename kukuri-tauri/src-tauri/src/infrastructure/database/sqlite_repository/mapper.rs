use crate::domain::entities::{Event, Post, Topic, User, UserProfile};
use crate::domain::value_objects::EventId;
use crate::shared::error::AppError;
use chrono::{DateTime, Utc};
use sqlx::{Row, sqlite::SqliteRow};

pub(super) fn map_post_row(
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

pub(super) fn extract_topic_from_tags(tags_json: &str) -> Option<String> {
    let tags = serde_json::from_str::<Vec<Vec<String>>>(tags_json).ok()?;
    tags.into_iter().find_map(|tag| match tag.as_slice() {
        [key, value, ..] if key == "t" => Some(value.clone()),
        _ => None,
    })
}

pub(super) fn map_topic_row(row: &SqliteRow) -> Result<Topic, AppError> {
    base_topic_from_row(row)
}

pub(super) fn map_joined_topic_row(row: &SqliteRow) -> Result<Topic, AppError> {
    let mut topic = base_topic_from_row(row)?;
    topic.is_joined = true;
    Ok(topic)
}

pub(super) fn map_user_row(row: &SqliteRow) -> Result<User, AppError> {
    let profile = UserProfile {
        display_name: row.try_get("display_name").unwrap_or_default(),
        bio: row.try_get("bio").unwrap_or_default(),
        avatar_url: row.try_get("avatar_url").ok(),
    };

    let user = User::new_with_profile(row.try_get("npub")?, profile);
    Ok(user)
}

pub(super) fn map_event_row(row: &SqliteRow) -> Result<Event, AppError> {
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

fn base_topic_from_row(row: &SqliteRow) -> Result<Topic, AppError> {
    let created_at =
        DateTime::from_timestamp_millis(row.try_get("created_at")?).unwrap_or_else(Utc::now);
    let description = row
        .try_get::<Option<String>, _>("description")?
        .unwrap_or_default();
    let mut topic = Topic::new_with_id(
        row.try_get("topic_id")?,
        row.try_get("name")?,
        description,
        created_at,
    );
    topic.updated_at =
        DateTime::from_timestamp_millis(row.try_get("updated_at")?).unwrap_or(created_at);
    topic.member_count = row.try_get::<i64, _>("member_count")? as u32;
    topic.post_count = row.try_get::<i64, _>("post_count")? as u32;
    Ok(topic)
}

fn parse_event_tags(tags_json: &str) -> Vec<Vec<String>> {
    serde_json::from_str(tags_json).unwrap_or_default()
}
