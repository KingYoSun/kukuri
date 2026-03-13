use crate::domain::entities::{Topic, TopicVisibility};
use crate::shared::error::AppError;
use chrono::{DateTime, Utc};
use sqlx::{Row, sqlite::SqliteRow};

pub(crate) fn map_topic_row(row: &SqliteRow) -> Result<Topic, AppError> {
    base_topic_from_row(row)
}

pub(crate) fn map_joined_topic_row(row: &SqliteRow) -> Result<Topic, AppError> {
    let mut topic = base_topic_from_row(row)?;
    topic.is_joined = true;
    Ok(topic)
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
    topic.visibility = match row.try_get::<String, _>("visibility")?.as_str() {
        "private" => TopicVisibility::Private,
        _ => TopicVisibility::Public,
    };
    Ok(topic)
}
