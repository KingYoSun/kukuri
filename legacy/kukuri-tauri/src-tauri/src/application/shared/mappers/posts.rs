use crate::domain::entities::{Post, User};
use crate::shared::error::AppError;
use chrono::{DateTime, Utc};
use sqlx::{Row, sqlite::SqliteRow};

fn is_nostr_event_id(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit())
}

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

    let scope = extract_scope_from_tags(&tags_json);
    let epoch = extract_epoch_from_tags(&tags_json);
    let thread_namespace = row
        .try_get::<Option<String>, _>("thread_namespace")
        .ok()
        .flatten()
        .or_else(|| extract_thread_namespace_from_tags(&tags_json));
    let thread_uuid = row
        .try_get::<Option<String>, _>("thread_uuid")
        .ok()
        .flatten()
        .or_else(|| extract_thread_uuid_from_tags(&tags_json));
    let thread_root_event_id = row
        .try_get::<Option<String>, _>("thread_root_event_id")
        .ok()
        .flatten()
        .or_else(|| extract_thread_root_event_id_from_tags(&tags_json))
        .or_else(|| thread_uuid.as_ref().map(|_| event_id.clone()));
    let thread_parent_event_id = row
        .try_get::<Option<String>, _>("thread_parent_event_id")
        .ok()
        .flatten()
        .or_else(|| extract_thread_parent_event_id_from_tags(&tags_json));
    let sync_status = row
        .try_get::<Option<i64>, _>("sync_status")
        .ok()
        .flatten()
        .unwrap_or(0);
    let sync_event_id = row
        .try_get::<Option<String>, _>("sync_event_id")
        .ok()
        .flatten()
        .filter(|value| !value.trim().is_empty());

    let mut post = Post::new_with_id(event_id, content, user, topic_id, created_at);
    post.scope = scope;
    post.epoch = epoch;
    post.thread_namespace = thread_namespace;
    post.thread_uuid = thread_uuid;
    post.thread_root_event_id = thread_root_event_id;
    post.thread_parent_event_id = thread_parent_event_id;
    post.event_id = sync_event_id.clone();
    post.is_synced = sync_status > 0 || sync_event_id.is_some() || is_nostr_event_id(&post.id);
    Ok(post)
}

pub(crate) fn extract_topic_from_tags(tags_json: &str) -> Option<String> {
    let tags = serde_json::from_str::<Vec<Vec<String>>>(tags_json).ok()?;
    tags.into_iter().find_map(|tag| match tag.as_slice() {
        [key, value, ..] if key == "t" => Some(value.clone()),
        _ => None,
    })
}

fn extract_tag_value(tags_json: &str, target: &str) -> Option<String> {
    let tags = serde_json::from_str::<Vec<Vec<String>>>(tags_json).ok()?;
    tags.into_iter().find_map(|tag| match tag.as_slice() {
        [key, value, ..] if key == target => Some(value.clone()),
        _ => None,
    })
}

fn extract_scope_from_tags(tags_json: &str) -> Option<String> {
    extract_tag_value(tags_json, "scope")
}

fn extract_epoch_from_tags(tags_json: &str) -> Option<i64> {
    extract_tag_value(tags_json, "epoch").and_then(|value| value.parse::<i64>().ok())
}

fn extract_thread_namespace_from_tags(tags_json: &str) -> Option<String> {
    extract_tag_value(tags_json, "thread")
}

fn extract_thread_uuid_from_tags(tags_json: &str) -> Option<String> {
    extract_tag_value(tags_json, "thread_uuid")
}

fn extract_thread_root_event_id_from_tags(tags_json: &str) -> Option<String> {
    extract_tag_value(tags_json, "thread_root_event_id")
}

fn extract_thread_parent_event_id_from_tags(tags_json: &str) -> Option<String> {
    extract_tag_value(tags_json, "thread_parent_event_id")
}
