use crate::domain::entities::{User, UserMetadata};
use crate::infrastructure::database::UserRepository;
use crate::shared::error::AppError;
use std::sync::Arc;

pub struct UserService {
    repository: Arc<dyn UserRepository>,
}

impl UserService {
    pub fn new(repository: Arc<dyn UserRepository>) -> Self {
        Self { repository }
    }

    pub async fn create_user(&self, npub: String, pubkey: String) -> Result<User, AppError> {
        let user = User::new(npub, pubkey);
        self.repository.create_user(&user).await?;
        Ok(user)
    }

    pub async fn get_user(&self, npub: &str) -> Result<Option<User>, AppError> {
        self.repository.get_user(npub).await
    }

    pub async fn get_user_by_pubkey(&self, pubkey: &str) -> Result<Option<User>, AppError> {
        self.repository.get_user_by_pubkey(pubkey).await
    }

    pub async fn update_profile(&self, npub: &str, metadata: UserMetadata) -> Result<(), AppError> {
        if let Some(mut user) = self.repository.get_user(npub).await? {
            user.update_metadata(metadata);
            self.repository.update_user(&user).await?;
        }
        Ok(())
    }

    pub async fn update_user(&self, user: User) -> Result<(), AppError> {
        self.repository.update_user(&user).await
    }

    pub async fn follow_user(
        &self,
        follower_npub: &str,
        target_npub: &str,
    ) -> Result<(), AppError> {
        // TODO: Implement follow relationship
        Ok(())
    }

    pub async fn unfollow_user(
        &self,
        follower_npub: &str,
        target_npub: &str,
    ) -> Result<(), AppError> {
        // TODO: Implement unfollow
        Ok(())
    }

    pub async fn get_followers(&self, npub: &str) -> Result<Vec<User>, AppError> {
        self.repository.get_followers(npub).await
    }

    pub async fn get_following(&self, npub: &str) -> Result<Vec<User>, AppError> {
        self.repository.get_following(npub).await
    }

    pub async fn delete_user(&self, npub: &str) -> Result<(), AppError> {
        self.repository.delete_user(npub).await
    }
}
