use crate::domain::entities::{User, UserProfile};
use crate::shared::error::AppError;
use chrono::{DateTime, Utc};
use sqlx::{Row, sqlite::SqliteRow};

pub(crate) fn map_user_row(row: &SqliteRow) -> Result<User, AppError> {
    let profile = UserProfile {
        display_name: row.try_get("display_name").unwrap_or_default(),
        bio: row.try_get("bio").unwrap_or_default(),
        avatar_url: row.try_get("avatar_url").ok(),
    };

    let mut user = User::new_with_profile(row.try_get("npub")?, profile);
    user.pubkey = row.try_get("pubkey")?;
    user.public_profile = row.try_get::<i64, _>("is_profile_public").unwrap_or(1) != 0;
    user.show_online_status = row.try_get::<i64, _>("show_online_status").unwrap_or(0) != 0;

    if let Ok(created_at_ms) = row.try_get::<i64, _>("created_at") {
        if let Some(timestamp) = DateTime::<Utc>::from_timestamp_millis(created_at_ms) {
            user.created_at = timestamp;
        }
    }
    if let Ok(updated_at_ms) = row.try_get::<i64, _>("updated_at") {
        if let Some(timestamp) = DateTime::<Utc>::from_timestamp_millis(updated_at_ms) {
            user.updated_at = timestamp;
        }
    }

    Ok(user)
}
