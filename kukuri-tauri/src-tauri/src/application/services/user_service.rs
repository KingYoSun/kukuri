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
        if follower_npub == target_npub {
            return Err(AppError::ValidationError(
                "Cannot follow yourself".to_string(),
            ));
        }

        let follower = self.resolve_user_by_npub(follower_npub).await?;
        let target = self.resolve_user_by_npub(target_npub).await?;

        self.repository
            .add_follow_relation(follower.pubkey(), target.pubkey())
            .await?;

        Ok(())
    }

    pub async fn unfollow_user(
        &self,
        follower_npub: &str,
        target_npub: &str,
    ) -> Result<(), AppError> {
        let follower = self.resolve_user_by_npub(follower_npub).await?;
        let target = self.resolve_user_by_npub(target_npub).await?;

        let removed = self
            .repository
            .remove_follow_relation(follower.pubkey(), target.pubkey())
            .await?;

        if removed {
            Ok(())
        } else {
            Err(AppError::NotFound(format!(
                "{} is not following {}",
                follower_npub, target_npub
            )))
        }
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

    async fn resolve_user_by_npub(&self, npub: &str) -> Result<User, AppError> {
        self.repository
            .get_user(npub)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("User not found: {npub}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::database::UserRepository;
    use async_trait::async_trait;
    use std::collections::{HashMap, HashSet};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    const ALICE_NPUB: &str = "npub1alice";
    const ALICE_PUB: &str = "alice_pub";
    const BOB_NPUB: &str = "npub1bob";
    const BOB_PUB: &str = "bob_pub";

    #[derive(Default)]
    struct InMemoryUserRepository {
        users: RwLock<HashMap<String, User>>,
        follows: RwLock<HashSet<(String, String)>>,
    }

    #[async_trait]
    impl UserRepository for InMemoryUserRepository {
        async fn create_user(&self, user: &User) -> Result<(), AppError> {
            let mut users = self.users.write().await;
            users.insert(user.npub.clone(), user.clone());
            Ok(())
        }

        async fn get_user(&self, npub: &str) -> Result<Option<User>, AppError> {
            let users = self.users.read().await;
            Ok(users.get(npub).cloned())
        }

        async fn get_user_by_pubkey(&self, pubkey: &str) -> Result<Option<User>, AppError> {
            let users = self.users.read().await;
            Ok(users.values().find(|u| u.pubkey == pubkey).cloned())
        }

        async fn update_user(&self, user: &User) -> Result<(), AppError> {
            let mut users = self.users.write().await;
            users.insert(user.npub.clone(), user.clone());
            Ok(())
        }

        async fn delete_user(&self, npub: &str) -> Result<(), AppError> {
            let mut users = self.users.write().await;
            if let Some(user) = users.remove(npub) {
                let mut follows = self.follows.write().await;
                follows.retain(|(follower, followed)| {
                    follower != &user.pubkey && followed != &user.pubkey
                });
            }
            Ok(())
        }

        async fn get_followers(&self, npub: &str) -> Result<Vec<User>, AppError> {
            let users = self.users.read().await;
            let target_pubkey = match users.get(npub) {
                Some(user) => user.pubkey.clone(),
                None => return Ok(vec![]),
            };
            let follows = self.follows.read().await;
            let mut result = Vec::new();
            for (follower, followed) in follows.iter() {
                if followed == &target_pubkey {
                    if let Some(user) = users.values().find(|u| u.pubkey == *follower) {
                        result.push(user.clone());
                    }
                }
            }
            Ok(result)
        }

        async fn get_following(&self, npub: &str) -> Result<Vec<User>, AppError> {
            let users = self.users.read().await;
            let follower_pubkey = match users.get(npub) {
                Some(user) => user.pubkey.clone(),
                None => return Ok(vec![]),
            };
            let follows = self.follows.read().await;
            let mut result = Vec::new();
            for (follower, followed) in follows.iter() {
                if follower == &follower_pubkey {
                    if let Some(user) = users.values().find(|u| u.pubkey == *followed) {
                        result.push(user.clone());
                    }
                }
            }
            Ok(result)
        }

        async fn add_follow_relation(
            &self,
            follower_pubkey: &str,
            followed_pubkey: &str,
        ) -> Result<bool, AppError> {
            let mut follows = self.follows.write().await;
            Ok(follows.insert((follower_pubkey.to_string(), followed_pubkey.to_string())))
        }

        async fn remove_follow_relation(
            &self,
            follower_pubkey: &str,
            followed_pubkey: &str,
        ) -> Result<bool, AppError> {
            let mut follows = self.follows.write().await;
            Ok(follows.remove(&(follower_pubkey.to_string(), followed_pubkey.to_string())))
        }
    }

    async fn setup_service() -> UserService {
        let repository: Arc<dyn UserRepository> = Arc::new(InMemoryUserRepository::default());
        UserService::new(repository)
    }

    async fn seed_user(service: &UserService, npub: &str, pubkey: &str) {
        service
            .create_user(npub.to_string(), pubkey.to_string())
            .await
            .expect("create user");
    }

    #[tokio::test]
    async fn follow_and_unfollow_flow() {
        let service = setup_service().await;
        seed_user(&service, ALICE_NPUB, ALICE_PUB).await;
        seed_user(&service, BOB_NPUB, BOB_PUB).await;

        service
            .follow_user(ALICE_NPUB, BOB_NPUB)
            .await
            .expect("follow");

        let following = service
            .get_following(ALICE_NPUB)
            .await
            .expect("following list");
        assert_eq!(following.len(), 1);
        assert_eq!(following[0].npub, BOB_NPUB);

        let followers = service
            .get_followers(BOB_NPUB)
            .await
            .expect("followers list");
        assert_eq!(followers.len(), 1);
        assert_eq!(followers[0].npub, ALICE_NPUB);

        service
            .unfollow_user(ALICE_NPUB, BOB_NPUB)
            .await
            .expect("unfollow");

        assert!(
            service
                .get_following(ALICE_NPUB)
                .await
                .expect("following after unfollow")
                .is_empty()
        );
        assert!(
            service
                .get_followers(BOB_NPUB)
                .await
                .expect("followers after unfollow")
                .is_empty()
        );
    }

    #[tokio::test]
    async fn follow_user_validates_identity() {
        let service = setup_service().await;
        seed_user(&service, ALICE_NPUB, ALICE_PUB).await;

        let err = service
            .follow_user(ALICE_NPUB, ALICE_NPUB)
            .await
            .expect_err("self follow should fail");
        assert!(matches!(err, AppError::ValidationError(_)));
    }

    #[tokio::test]
    async fn follow_user_requires_existing_target() {
        let service = setup_service().await;
        seed_user(&service, ALICE_NPUB, ALICE_PUB).await;

        let err = service
            .follow_user(ALICE_NPUB, BOB_NPUB)
            .await
            .expect_err("missing target");
        assert!(matches!(err, AppError::NotFound(_)));
    }
}
