use super::SqliteRepository;
use super::mapper::map_user_row;
use super::queries::{
    DELETE_FOLLOW_RELATION, DELETE_USER, INSERT_USER, SEARCH_USERS, SELECT_FOLLOWERS,
    SELECT_FOLLOWING, SELECT_USER_BY_NPUB, SELECT_USER_BY_PUBKEY, UPDATE_USER,
    UPSERT_FOLLOW_RELATION,
};
use crate::application::ports::repositories::UserRepository;
use crate::domain::entities::User;
use crate::shared::error::AppError;
use async_trait::async_trait;

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

    async fn get_followers(&self, npub: &str) -> Result<Vec<User>, AppError> {
        let rows = sqlx::query(SELECT_FOLLOWERS)
            .bind(npub)
            .fetch_all(self.pool.get_pool())
            .await?;

        let mut users = Vec::with_capacity(rows.len());
        for row in rows {
            let user = map_user_row(&row)?;
            users.push(user);
        }

        Ok(users)
    }

    async fn get_following(&self, npub: &str) -> Result<Vec<User>, AppError> {
        let rows = sqlx::query(SELECT_FOLLOWING)
            .bind(npub)
            .fetch_all(self.pool.get_pool())
            .await?;

        let mut users = Vec::with_capacity(rows.len());
        for row in rows {
            let user = map_user_row(&row)?;
            users.push(user);
        }

        Ok(users)
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
