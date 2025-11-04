use super::SqliteRepository;
use super::mapper::map_user_row;
use super::queries::{
    DELETE_FOLLOW_RELATION, DELETE_USER, INSERT_USER, SEARCH_USERS, SELECT_USER_BY_NPUB,
    SELECT_USER_BY_PUBKEY, UPDATE_USER, UPSERT_FOLLOW_RELATION,
};
use crate::application::ports::repositories::{UserCursorPage, UserRepository};
use crate::domain::entities::User;
use crate::shared::error::AppError;
use async_trait::async_trait;
use sqlx::{QueryBuilder, Row, Sqlite};

fn parse_follow_cursor(cursor: &str) -> Result<(i64, String), AppError> {
    let mut parts = cursor.splitn(2, ':');
    let timestamp = parts
        .next()
        .ok_or_else(|| AppError::InvalidInput("Invalid cursor format".into()))?
        .parse::<i64>()
        .map_err(|_| AppError::InvalidInput("Invalid cursor timestamp".into()))?;
    let pubkey = parts
        .next()
        .ok_or_else(|| AppError::InvalidInput("Invalid cursor format".into()))?
        .to_string();
    if pubkey.is_empty() {
        return Err(AppError::InvalidInput("Invalid cursor pubkey".into()));
    }
    Ok((timestamp, pubkey))
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
    ) -> Result<UserCursorPage, AppError> {
        let limit = limit.clamp(1, 100);
        let fetch_limit = limit + 1;

        let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(
            "SELECT u.npub, u.pubkey, u.display_name, u.bio, u.avatar_url, u.created_at, u.updated_at, \
                f.created_at AS relation_created_at, f.follower_pubkey AS relation_pubkey \
             FROM users u \
             INNER JOIN follows f ON u.pubkey = f.follower_pubkey \
             WHERE f.followed_pubkey = (SELECT pubkey FROM users WHERE npub = ?)",
        );
        builder.push_bind(npub);

        if let Some(cursor) = cursor {
            let (timestamp, pubkey) = parse_follow_cursor(cursor)?;
            builder.push(" AND (f.created_at < ? OR (f.created_at = ? AND f.follower_pubkey < ?))");
            builder.push_bind(timestamp);
            builder.push_bind(timestamp);
            builder.push_bind(pubkey);
        }

        builder.push(" ORDER BY f.created_at DESC, f.follower_pubkey DESC LIMIT ?");
        builder.push_bind(fetch_limit as i64);

        let rows = builder.build().fetch_all(self.pool.get_pool()).await?;

        let mut users = Vec::with_capacity(rows.len().min(limit));
        let mut next_cursor = None;

        for (index, row) in rows.into_iter().enumerate() {
            if index >= limit {
                let timestamp: i64 = row.try_get("relation_created_at")?;
                let pubkey: String = row.try_get("relation_pubkey")?;
                next_cursor = Some(format!("{timestamp}:{pubkey}"));
                break;
            }
            users.push(map_user_row(&row)?);
        }

        Ok(UserCursorPage {
            users,
            next_cursor,
            has_more: next_cursor.is_some(),
        })
    }

    async fn get_following_paginated(
        &self,
        npub: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<UserCursorPage, AppError> {
        let limit = limit.clamp(1, 100);
        let fetch_limit = limit + 1;

        let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(
            "SELECT u.npub, u.pubkey, u.display_name, u.bio, u.avatar_url, u.created_at, u.updated_at, \
                f.created_at AS relation_created_at, f.followed_pubkey AS relation_pubkey \
             FROM users u \
             INNER JOIN follows f ON u.pubkey = f.followed_pubkey \
             WHERE f.follower_pubkey = (SELECT pubkey FROM users WHERE npub = ?)",
        );
        builder.push_bind(npub);

        if let Some(cursor) = cursor {
            let (timestamp, pubkey) = parse_follow_cursor(cursor)?;
            builder.push(" AND (f.created_at < ? OR (f.created_at = ? AND f.followed_pubkey < ?))");
            builder.push_bind(timestamp);
            builder.push_bind(timestamp);
            builder.push_bind(pubkey);
        }

        builder.push(" ORDER BY f.created_at DESC, f.followed_pubkey DESC LIMIT ?");
        builder.push_bind(fetch_limit as i64);

        let rows = builder.build().fetch_all(self.pool.get_pool()).await?;

        let mut users = Vec::with_capacity(rows.len().min(limit));
        let mut next_cursor = None;

        for (index, row) in rows.into_iter().enumerate() {
            if index >= limit {
                let timestamp: i64 = row.try_get("relation_created_at")?;
                let pubkey: String = row.try_get("relation_pubkey")?;
                next_cursor = Some(format!("{timestamp}:{pubkey}"));
                break;
            }
            users.push(map_user_row(&row)?);
        }

        Ok(UserCursorPage {
            users,
            next_cursor,
            has_more: next_cursor.is_some(),
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
