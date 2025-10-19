use crate::domain::entities::{User, UserProfile};
use crate::shared::error::AppError;
use sqlx::{Row, sqlite::SqliteRow};

pub(crate) fn map_user_row(row: &SqliteRow) -> Result<User, AppError> {
    let profile = UserProfile {
        display_name: row.try_get("display_name").unwrap_or_default(),
        bio: row.try_get("bio").unwrap_or_default(),
        avatar_url: row.try_get("avatar_url").ok(),
    };

    let user = User::new_with_profile(row.try_get("npub")?, profile);
    Ok(user)
}
