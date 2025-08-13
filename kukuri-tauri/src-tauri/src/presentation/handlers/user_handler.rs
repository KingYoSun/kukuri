use crate::{
    application::services::UserService,
    presentation::dto::{
        user_dto::UserProfile,
        ApiResponse,
    },
    shared::error::AppError,
};
use std::sync::Arc;

pub struct UserHandler {
    user_service: Arc<UserService>,
}

impl UserHandler {
    pub fn new(user_service: Arc<UserService>) -> Self {
        Self { user_service }
    }

    pub async fn get_user_profile(&self, npub: String) -> Result<UserProfile, AppError> {
        let user = self.user_service.get_user(&npub).await?;

        Ok(UserProfile {
            npub: user.npub,
            pubkey: user.pubkey,
            name: user.name,
            display_name: user.display_name,
            about: user.about,
            picture: user.picture,
            banner: user.banner,
            website: user.website,
            nip05: user.nip05,
        })
    }

    pub async fn update_user_profile(&self, profile: UserProfile) -> Result<(), AppError> {
        let user = crate::domain::entities::user::User {
            npub: profile.npub,
            pubkey: profile.pubkey,
            name: profile.name,
            display_name: profile.display_name,
            about: profile.about,
            picture: profile.picture,
            banner: profile.banner,
            website: profile.website,
            nip05: profile.nip05,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        self.user_service.update_user(user).await
    }

    pub async fn get_followers(&self, npub: String) -> Result<Vec<UserProfile>, AppError> {
        let followers = self.user_service.get_followers(&npub).await?;

        Ok(followers
            .into_iter()
            .map(|user| UserProfile {
                npub: user.npub,
                pubkey: user.pubkey,
                name: user.name,
                display_name: user.display_name,
                about: user.about,
                picture: user.picture,
                banner: user.banner,
                website: user.website,
                nip05: user.nip05,
            })
            .collect())
    }

    pub async fn get_following(&self, npub: String) -> Result<Vec<UserProfile>, AppError> {
        let following = self.user_service.get_following(&npub).await?;

        Ok(following
            .into_iter()
            .map(|user| UserProfile {
                npub: user.npub,
                pubkey: user.pubkey,
                name: user.name,
                display_name: user.display_name,
                about: user.about,
                picture: user.picture,
                banner: user.banner,
                website: user.website,
                nip05: user.nip05,
            })
            .collect())
    }
}