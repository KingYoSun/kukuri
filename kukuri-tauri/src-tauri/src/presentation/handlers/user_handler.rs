use crate::{
    application::services::UserService, domain::entities::User,
    presentation::dto::user_dto::UserProfile, shared::error::AppError,
};
use std::sync::Arc;

pub struct UserHandler {
    user_service: Arc<UserService>,
}

impl UserHandler {
    pub fn new(user_service: Arc<UserService>) -> Self {
        Self { user_service }
    }

    fn map_user_to_profile(user: User) -> UserProfile {
        UserProfile {
            npub: user.npub,
            pubkey: user.pubkey,
            name: user.name,
            display_name: Some(user.profile.display_name),
            about: Some(user.profile.bio),
            picture: user.profile.avatar_url,
            banner: None,
            website: None,
            nip05: user.nip05,
            is_profile_public: Some(user.public_profile),
            show_online_status: Some(user.show_online_status),
        }
    }

    pub async fn get_user_profile(&self, npub: String) -> Result<UserProfile, AppError> {
        let user = self
            .user_service
            .get_user(&npub)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("User not found: {npub}")))?;

        Ok(UserProfile {
            npub: user.npub.clone(),
            pubkey: user.pubkey.clone(),
            name: user.name.clone(),
            display_name: user.profile.display_name.clone().into(),
            about: user.profile.bio.clone().into(),
            picture: user.profile.avatar_url.clone(),
            banner: None,
            website: None,
            nip05: user.nip05.clone(),
            is_profile_public: Some(user.public_profile),
            show_online_status: Some(user.show_online_status),
        })
    }

    pub async fn search_users(
        &self,
        query: String,
        limit: Option<usize>,
    ) -> Result<Vec<UserProfile>, AppError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(vec![]);
        }

        let limit = limit.unwrap_or(20).min(100);
        let users = self.user_service.search_users(trimmed, limit).await?;

        Ok(users.into_iter().map(Self::map_user_to_profile).collect())
    }

    pub async fn update_user_profile(&self, profile: UserProfile) -> Result<(), AppError> {
        let user = crate::domain::entities::user::User {
            npub: profile.npub,
            pubkey: profile.pubkey,
            profile: crate::domain::entities::user::UserProfile {
                display_name: profile.display_name.unwrap_or_default(),
                bio: profile.about.unwrap_or_default(),
                avatar_url: profile.picture,
            },
            name: profile.name,
            nip05: profile.nip05,
            lud16: None,
            public_profile: profile.is_profile_public.unwrap_or(true),
            show_online_status: profile.show_online_status.unwrap_or(false),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        self.user_service.update_user(user).await?;
        Ok(())
    }

    pub async fn get_followers(&self, npub: String) -> Result<Vec<UserProfile>, AppError> {
        let followers = self.user_service.get_followers(&npub).await?;

        Ok(followers
            .into_iter()
            .map(Self::map_user_to_profile)
            .collect())
    }

    pub async fn get_following(&self, npub: String) -> Result<Vec<UserProfile>, AppError> {
        let following = self.user_service.get_following(&npub).await?;

        Ok(following
            .into_iter()
            .map(Self::map_user_to_profile)
            .collect())
    }
}
