use super::SqliteRepository;
use super::mapper::map_user_row;
use super::queries::{
    DELETE_FOLLOW_RELATION, DELETE_USER, INSERT_USER, SEARCH_USERS, SELECT_USER_BY_NPUB,
    SELECT_USER_BY_PUBKEY, UPDATE_USER, UPSERT_FOLLOW_RELATION,
};
use crate::application::ports::repositories::{FollowListSort, UserCursorPage, UserRepository};
use crate::domain::entities::User;
use crate::shared::error::AppError;
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use sqlx::{QueryBuilder, Row, Sqlite};

const SORT_KEY_EXPR: &str = "COALESCE(NULLIF(TRIM(u.display_name), ''), u.npub)";
const SORT_KEY_LOWER_EXPR: &str = "LOWER(COALESCE(NULLIF(TRIM(u.display_name), ''), u.npub))";

fn encode_follow_cursor(sort: FollowListSort, primary: &str, pubkey: &str) -> String {
    let encoded_primary = URL_SAFE_NO_PAD.encode(primary.as_bytes());
    format!("{}|{}|{}", sort.as_str(), encoded_primary, pubkey)
}

fn decode_follow_cursor(
    cursor: &str,
    expected_sort: FollowListSort,
) -> Result<(String, String), AppError> {
    let mut parts = cursor.splitn(3, '|');
    let sort_part = parts
        .next()
        .ok_or_else(|| AppError::InvalidInput("Invalid cursor format".into()))?;
    if sort_part != expected_sort.as_str() {
        return Err(AppError::InvalidInput("Cursor sort mismatch".into()));
    }
    let primary_encoded = parts
        .next()
        .ok_or_else(|| AppError::InvalidInput("Invalid cursor format".into()))?;
    let pubkey = parts
        .next()
        .ok_or_else(|| AppError::InvalidInput("Invalid cursor format".into()))?;
    let primary_bytes = URL_SAFE_NO_PAD
        .decode(primary_encoded.as_bytes())
        .map_err(|_| AppError::InvalidInput("Invalid cursor payload".into()))?;
    let primary = String::from_utf8(primary_bytes)
        .map_err(|_| AppError::InvalidInput("Invalid cursor payload".into()))?;
    if pubkey.is_empty() {
        return Err(AppError::InvalidInput("Invalid cursor pubkey".into()));
    }
    Ok((primary, pubkey.to_string()))
}

#[async_trait]
impl UserRepository for SqliteRepository {
    async fn create_user(&self, user: &User) -> Result<(), AppError> {
        sqlx::query(INSERT_USER)
            .bind(user.npub())
            .bind(user.pubkey())
            .bind(&user.profile.display_name)
            .bind(&user.profile.bio)
            .bind(&user.profile.avatar_url)
            .bind(user.created_at.timestamp_millis())
            .bind(user.updated_at.timestamp_millis())
            .execute(self.pool.get_pool())
            .await?;

        Ok(())
    }

    async fn get_user(&self, npub: &str) -> Result<Option<User>, AppError> {
        let row = sqlx::query(SELECT_USER_BY_NPUB)
            .bind(npub)
            .fetch_optional(self.pool.get_pool())
            .await?;

        match row {
            Some(row) => Ok(Some(map_user_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn get_user_by_pubkey(&self, pubkey: &str) -> Result<Option<User>, AppError> {
        let row = sqlx::query(SELECT_USER_BY_PUBKEY)
            .bind(pubkey)
            .fetch_optional(self.pool.get_pool())
            .await?;

        match row {
            Some(row) => Ok(Some(map_user_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn search_users(&self, query: &str, limit: usize) -> Result<Vec<User>, AppError> {
        if query.trim().is_empty() {
            return Ok(vec![]);
        }

        let rows = sqlx::query(SEARCH_USERS)
            .bind(query)
            .bind(limit as i64)
            .fetch_all(self.pool.get_pool())
            .await?;

        let mut users = Vec::with_capacity(rows.len());
        for row in rows {
            users.push(map_user_row(&row)?);
        }

        Ok(users)
    }

    async fn update_user(&self, user: &User) -> Result<(), AppError> {
        sqlx::query(UPDATE_USER)
            .bind(&user.profile.display_name)
            .bind(&user.profile.bio)
            .bind(&user.profile.avatar_url)
            .bind(user.updated_at.timestamp_millis())
            .bind(user.npub())
            .execute(self.pool.get_pool())
            .await?;

        Ok(())
    }

    async fn delete_user(&self, npub: &str) -> Result<(), AppError> {
        sqlx::query(DELETE_USER)
            .bind(npub)
            .execute(self.pool.get_pool())
            .await?;

        Ok(())
    }

    async fn get_followers_paginated(
        &self,
        npub: &str,
        cursor: Option<&str>,
        limit: usize,
        sort: FollowListSort,
        search: Option<&str>,
    ) -> Result<UserCursorPage, AppError> {
        let limit = limit.clamp(1, 100);
        let fetch_limit = limit + 1;
        let normalized_search = search.map(|s| s.to_lowercase());

        let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(&format!(
            "SELECT u.npub, u.pubkey, u.display_name, u.bio, u.avatar_url, u.created_at, u.updated_at, \
                    f.created_at AS relation_created_at, f.follower_pubkey AS relation_pubkey, \
                    {sort_key_lower} AS sort_key_normalized \
                 FROM users u \
                 INNER JOIN follows f ON u.pubkey = f.follower_pubkey \
                 WHERE f.followed_pubkey = (SELECT pubkey FROM users WHERE npub = ?)",
            sort_key_lower = SORT_KEY_LOWER_EXPR
        ));
        builder.push_bind(npub);

        let mut count_builder: QueryBuilder<Sqlite> = QueryBuilder::new(&format!(
            "SELECT COUNT(*) as total_count \
                 FROM users u \
                 INNER JOIN follows f ON u.pubkey = f.follower_pubkey \
                 WHERE f.followed_pubkey = (SELECT pubkey FROM users WHERE npub = ?)",
        ));
        count_builder.push_bind(npub);

        if let Some(search_value) = normalized_search.as_ref() {
            let pattern = format!("%{}%", search_value);
            builder.push(" AND (");
            builder.push(SORT_KEY_LOWER_EXPR);
            builder.push(" LIKE ? OR LOWER(u.npub) LIKE ? OR LOWER(u.pubkey) LIKE ?)");
            builder.push_bind(pattern.clone());
            builder.push_bind(pattern.clone());
            builder.push_bind(pattern.clone());

            count_builder.push(" AND (");
            count_builder.push(SORT_KEY_LOWER_EXPR);
            count_builder.push(" LIKE ? OR LOWER(u.npub) LIKE ? OR LOWER(u.pubkey) LIKE ?)");
            count_builder.push_bind(pattern.clone());
            count_builder.push_bind(pattern.clone());
            count_builder.push_bind(pattern);
        }

        if let Some(cursor_str) = cursor {
            let (primary_str, cursor_pubkey) = decode_follow_cursor(cursor_str, sort)?;
            match sort {
                FollowListSort::Recent => {
                    let timestamp = primary_str
                        .parse::<i64>()
                        .map_err(|_| AppError::InvalidInput("Invalid cursor timestamp".into()))?;
                    builder.push(
                        " AND (f.created_at < ? OR (f.created_at = ? AND f.follower_pubkey < ?))",
                    );
                    builder.push_bind(timestamp);
                    builder.push_bind(timestamp);
                    builder.push_bind(cursor_pubkey.clone());
                }
                FollowListSort::Oldest => {
                    let timestamp = primary_str
                        .parse::<i64>()
                        .map_err(|_| AppError::InvalidInput("Invalid cursor timestamp".into()))?;
                    builder.push(
                        " AND (f.created_at > ? OR (f.created_at = ? AND f.follower_pubkey > ?))",
                    );
                    builder.push_bind(timestamp);
                    builder.push_bind(timestamp);
                    builder.push_bind(cursor_pubkey.clone());
                }
                FollowListSort::NameAsc => {
                    builder.push(" AND (");
                    builder.push(SORT_KEY_LOWER_EXPR);
                    builder.push(" > ? OR (");
                    builder.push(SORT_KEY_LOWER_EXPR);
                    builder.push(" = ? AND f.follower_pubkey > ?))");
                    builder.push_bind(primary_str.clone());
                    builder.push_bind(primary_str.clone());
                    builder.push_bind(cursor_pubkey.clone());
                }
                FollowListSort::NameDesc => {
                    builder.push(" AND (");
                    builder.push(SORT_KEY_LOWER_EXPR);
                    builder.push(" < ? OR (");
                    builder.push(SORT_KEY_LOWER_EXPR);
                    builder.push(" = ? AND f.follower_pubkey < ?))");
                    builder.push_bind(primary_str.clone());
                    builder.push_bind(primary_str);
                    builder.push_bind(cursor_pubkey.clone());
                }
            }
        }

        match sort {
            FollowListSort::Recent => {
                builder.push(" ORDER BY f.created_at DESC, f.follower_pubkey DESC");
            }
            FollowListSort::Oldest => {
                builder.push(" ORDER BY f.created_at ASC, f.follower_pubkey ASC");
            }
            FollowListSort::NameAsc => {
                builder.push(" ORDER BY sort_key_normalized ASC, f.follower_pubkey ASC");
            }
            FollowListSort::NameDesc => {
                builder.push(" ORDER BY sort_key_normalized DESC, f.follower_pubkey DESC");
            }
        }
        builder.push(" LIMIT ?");
        builder.push_bind(fetch_limit as i64);

        let rows = builder.build().fetch_all(self.pool.get_pool()).await?;
        let count_row = count_builder
            .build()
            .fetch_one(self.pool.get_pool())
            .await?;
        let total_count: i64 = count_row.try_get("total_count")?;
        let total_count = total_count.max(0) as u64;

        let mut users = Vec::with_capacity(rows.len().min(limit));
        let mut next_cursor = None;

        for (index, row) in rows.into_iter().enumerate() {
            if index >= limit {
                let relation_pubkey: String = row.try_get("relation_pubkey")?;
                let primary_value = match sort {
                    FollowListSort::Recent | FollowListSort::Oldest => {
                        let timestamp: i64 = row.try_get("relation_created_at")?;
                        timestamp.to_string()
                    }
                    FollowListSort::NameAsc | FollowListSort::NameDesc => {
                        row.try_get::<String, _>("sort_key_normalized")?
                    }
                };
                next_cursor = Some(encode_follow_cursor(sort, &primary_value, &relation_pubkey));
                break;
            }
            users.push(map_user_row(&row)?);
        }

        let has_more = next_cursor.is_some();

        Ok(UserCursorPage {
            users,
            next_cursor,
            has_more,
            total_count,
        })
    }

    async fn get_following_paginated(
        &self,
        npub: &str,
        cursor: Option<&str>,
        limit: usize,
        sort: FollowListSort,
        search: Option<&str>,
    ) -> Result<UserCursorPage, AppError> {
        let limit = limit.clamp(1, 100);
        let fetch_limit = limit + 1;
        let normalized_search = search.map(|s| s.to_lowercase());

        let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(&format!(
            "SELECT u.npub, u.pubkey, u.display_name, u.bio, u.avatar_url, u.created_at, u.updated_at, \
                    f.created_at AS relation_created_at, f.followed_pubkey AS relation_pubkey, \
                    {sort_key_lower} AS sort_key_normalized \
                 FROM users u \
                 INNER JOIN follows f ON u.pubkey = f.followed_pubkey \
                 WHERE f.follower_pubkey = (SELECT pubkey FROM users WHERE npub = ?)",
            sort_key_lower = SORT_KEY_LOWER_EXPR
        ));
        builder.push_bind(npub);

        let mut count_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
            "SELECT COUNT(*) as total_count \
             FROM users u \
             INNER JOIN follows f ON u.pubkey = f.followed_pubkey \
             WHERE f.follower_pubkey = (SELECT pubkey FROM users WHERE npub = ?)",
        );
        count_builder.push_bind(npub);

        if let Some(search_value) = normalized_search.as_ref() {
            let pattern = format!("%{}%", search_value);
            builder.push(" AND (");
            builder.push(SORT_KEY_LOWER_EXPR);
            builder.push(" LIKE ? OR LOWER(u.npub) LIKE ? OR LOWER(u.pubkey) LIKE ?)");
            builder.push_bind(pattern.clone());
            builder.push_bind(pattern.clone());
            builder.push_bind(pattern.clone());

            count_builder.push(" AND (");
            count_builder.push(SORT_KEY_LOWER_EXPR);
            count_builder.push(" LIKE ? OR LOWER(u.npub) LIKE ? OR LOWER(u.pubkey) LIKE ?)");
            count_builder.push_bind(pattern.clone());
            count_builder.push_bind(pattern.clone());
            count_builder.push_bind(pattern);
        }

        if let Some(cursor_str) = cursor {
            let (primary_str, cursor_pubkey) = decode_follow_cursor(cursor_str, sort)?;
            match sort {
                FollowListSort::Recent => {
                    let timestamp = primary_str
                        .parse::<i64>()
                        .map_err(|_| AppError::InvalidInput("Invalid cursor timestamp".into()))?;
                    builder.push(
                        " AND (f.created_at < ? OR (f.created_at = ? AND f.followed_pubkey < ?))",
                    );
                    builder.push_bind(timestamp);
                    builder.push_bind(timestamp);
                    builder.push_bind(cursor_pubkey.clone());
                }
                FollowListSort::Oldest => {
                    let timestamp = primary_str
                        .parse::<i64>()
                        .map_err(|_| AppError::InvalidInput("Invalid cursor timestamp".into()))?;
                    builder.push(
                        " AND (f.created_at > ? OR (f.created_at = ? AND f.followed_pubkey > ?))",
                    );
                    builder.push_bind(timestamp);
                    builder.push_bind(timestamp);
                    builder.push_bind(cursor_pubkey.clone());
                }
                FollowListSort::NameAsc => {
                    builder.push(" AND (");
                    builder.push(SORT_KEY_LOWER_EXPR);
                    builder.push(" > ? OR (");
                    builder.push(SORT_KEY_LOWER_EXPR);
                    builder.push(" = ? AND f.followed_pubkey > ?))");
                    builder.push_bind(primary_str.clone());
                    builder.push_bind(primary_str.clone());
                    builder.push_bind(cursor_pubkey.clone());
                }
                FollowListSort::NameDesc => {
                    builder.push(" AND (");
                    builder.push(SORT_KEY_LOWER_EXPR);
                    builder.push(" < ? OR (");
                    builder.push(SORT_KEY_LOWER_EXPR);
                    builder.push(" = ? AND f.followed_pubkey < ?))");
                    builder.push_bind(primary_str.clone());
                    builder.push_bind(primary_str);
                    builder.push_bind(cursor_pubkey.clone());
                }
            }
        }

        match sort {
            FollowListSort::Recent => {
                builder.push(" ORDER BY f.created_at DESC, f.followed_pubkey DESC");
            }
            FollowListSort::Oldest => {
                builder.push(" ORDER BY f.created_at ASC, f.followed_pubkey ASC");
            }
            FollowListSort::NameAsc => {
                builder.push(" ORDER BY sort_key_normalized ASC, f.followed_pubkey ASC");
            }
            FollowListSort::NameDesc => {
                builder.push(" ORDER BY sort_key_normalized DESC, f.followed_pubkey DESC");
            }
        }
        builder.push(" LIMIT ?");
        builder.push_bind(fetch_limit as i64);

        let rows = builder.build().fetch_all(self.pool.get_pool()).await?;
        let count_row = count_builder
            .build()
            .fetch_one(self.pool.get_pool())
            .await?;
        let total_count: i64 = count_row.try_get("total_count")?;
        let total_count = total_count.max(0) as u64;

        let mut users = Vec::with_capacity(rows.len().min(limit));
        let mut next_cursor = None;

        for (index, row) in rows.into_iter().enumerate() {
            if index >= limit {
                let relation_pubkey: String = row.try_get("relation_pubkey")?;
                let primary_value = match sort {
                    FollowListSort::Recent | FollowListSort::Oldest => {
                        let timestamp: i64 = row.try_get("relation_created_at")?;
                        timestamp.to_string()
                    }
                    FollowListSort::NameAsc | FollowListSort::NameDesc => {
                        row.try_get::<String, _>("sort_key_normalized")?
                    }
                };
                next_cursor = Some(encode_follow_cursor(sort, &primary_value, &relation_pubkey));
                break;
            }
            users.push(map_user_row(&row)?);
        }

        let has_more = next_cursor.is_some();

        Ok(UserCursorPage {
            users,
            next_cursor,
            has_more,
            total_count,
        })
    }

    async fn add_follow_relation(
        &self,
        follower_pubkey: &str,
        followed_pubkey: &str,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(UPSERT_FOLLOW_RELATION)
            .bind(follower_pubkey)
            .bind(followed_pubkey)
            .execute(self.pool.get_pool())
            .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn remove_follow_relation(
        &self,
        follower_pubkey: &str,
        followed_pubkey: &str,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(DELETE_FOLLOW_RELATION)
            .bind(follower_pubkey)
            .bind(followed_pubkey)
            .execute(self.pool.get_pool())
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
