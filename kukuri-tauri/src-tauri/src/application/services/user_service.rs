use crate::application::ports::repositories::{UserCursorPage, UserRepository};
use crate::domain::entities::{User, UserMetadata};
use crate::shared::{AppError, ValidationFailureKind};
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

    pub async fn search_users(&self, query: &str, limit: usize) -> Result<Vec<User>, AppError> {
        if query.trim().is_empty() {
            return Ok(vec![]);
        }
        self.repository.search_users(query, limit).await
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
            return Err(AppError::validation(
                ValidationFailureKind::Generic,
                "Cannot follow yourself",
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
                "{follower_npub} is not following {target_npub}"
            )))
        }
    }

    pub async fn get_followers_paginated(
        &self,
        npub: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<UserCursorPage, AppError> {
        self.repository
            .get_followers_paginated(npub, cursor, limit)
            .await
    }

    pub async fn get_following_paginated(
        &self,
        npub: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<UserCursorPage, AppError> {
        self.repository
            .get_following_paginated(npub, cursor, limit)
            .await
    }

    pub async fn get_followers(&self, npub: &str) -> Result<Vec<User>, AppError> {
        let mut all_users = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let UserCursorPage {
                users,
                next_cursor,
                has_more,
            } = self
                .get_followers_paginated(npub, cursor.as_deref(), 100)
                .await?;

            all_users.extend(users.into_iter());

            if !has_more || next_cursor.is_none() {
                break;
            }

            cursor = next_cursor;
        }

        Ok(all_users)
    }

    pub async fn get_following(&self, npub: &str) -> Result<Vec<User>, AppError> {
        let mut all_users = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let UserCursorPage {
                users,
                next_cursor,
                has_more,
            } = self
                .get_following_paginated(npub, cursor.as_deref(), 100)
                .await?;

            all_users.extend(users.into_iter());

            if !has_more || next_cursor.is_none() {
                break;
            }

            cursor = next_cursor;
        }

        Ok(all_users)
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
    use crate::application::ports::repositories::UserRepository;
    use async_trait::async_trait;
    use std::collections::{HashMap, HashSet};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    const ALICE_NPUB: &str = "npub1alice";
    const ALICE_PUB: &str = "alice_pub";
    const BOB_NPUB: &str = "npub1bob";
    const BOB_PUB: &str = "bob_pub";

    fn parse_cursor(cursor: &str) -> Result<(i64, String), AppError> {
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
        Ok((timestamp, pubkey))
    }
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

        async fn search_users(&self, query: &str, limit: usize) -> Result<Vec<User>, AppError> {
            if query.trim().is_empty() {
                return Ok(vec![]);
            }

            let query_lower = query.to_lowercase();
            let users = self.users.read().await;
            let mut results: Vec<User> = users
                .values()
                .filter(|user| {
                    let display_name = user.profile.display_name.to_lowercase();
                    let bio = user.profile.bio.to_lowercase();
                    display_name.contains(&query_lower)
                        || bio.contains(&query_lower)
                        || user.npub.to_lowercase().contains(&query_lower)
                        || user.pubkey.to_lowercase().contains(&query_lower)
                })
                .cloned()
                .collect();

            results.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            if results.len() > limit {
                results.truncate(limit);
            }

            Ok(results)
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

        async fn get_followers_paginated(
            &self,
            npub: &str,
            cursor: Option<&str>,
            limit: usize,
        ) -> Result<UserCursorPage, AppError> {
            let users = self.users.read().await;
            let target_pubkey = match users.get(npub) {
                Some(user) => user.pubkey.clone(),
                None => {
                    return Ok(UserCursorPage {
                        users: vec![],
                        next_cursor: None,
                        has_more: false,
                    });
                }
            };
            let follows = self.follows.read().await;
            let mut follower_pubkeys: Vec<String> = follows
                .iter()
                .filter_map(|(follower, followed)| {
                    if followed == &target_pubkey {
                        Some(follower.clone())
                    } else {
                        None
                    }
                })
                .collect();

            follower_pubkeys.sort();
            follower_pubkeys.reverse();

            let mut start_index = 0usize;
            if let Some(cursor) = cursor {
                let (_, cursor_pubkey) = parse_cursor(cursor)?;
                if let Some(position) = follower_pubkeys
                    .iter()
                    .position(|pubkey| pubkey == &cursor_pubkey)
                {
                    start_index = position.saturating_add(1);
                }
            }

            let mut items = Vec::new();
            let mut next_cursor = None;
            for (index, pubkey) in follower_pubkeys.into_iter().enumerate().skip(start_index) {
                if items.len() >= limit {
                    next_cursor = Some(format!("{index}:{pubkey}"));
                    break;
                }
                if let Some(user) = users.values().find(|u| u.pubkey == pubkey) {
                    items.push(user.clone());
                }
            }

            let has_more = next_cursor.is_some();

            Ok(UserCursorPage {
                users: items,
                next_cursor,
                has_more,
            })
        }

        async fn get_following_paginated(
            &self,
            npub: &str,
            cursor: Option<&str>,
            limit: usize,
        ) -> Result<UserCursorPage, AppError> {
            let users = self.users.read().await;
            let follower_pubkey = match users.get(npub) {
                Some(user) => user.pubkey.clone(),
                None => {
                    return Ok(UserCursorPage {
                        users: vec![],
                        next_cursor: None,
                        has_more: false,
                    });
                }
            };
            let follows = self.follows.read().await;
            let mut followed_pubkeys: Vec<String> = follows
                .iter()
                .filter_map(|(follower, followed)| {
                    if follower == &follower_pubkey {
                        Some(followed.clone())
                    } else {
                        None
                    }
                })
                .collect();

            followed_pubkeys.sort();
            followed_pubkeys.reverse();

            let mut start_index = 0usize;
            if let Some(cursor) = cursor {
                let (_, cursor_pubkey) = parse_cursor(cursor)?;
                if let Some(position) = followed_pubkeys
                    .iter()
                    .position(|pubkey| pubkey == &cursor_pubkey)
                {
                    start_index = position.saturating_add(1);
                }
            }

            let mut items = Vec::new();
            let mut next_cursor = None;
            for (index, pubkey) in followed_pubkeys.into_iter().enumerate().skip(start_index) {
                if items.len() >= limit {
                    next_cursor = Some(format!("{index}:{pubkey}"));
                    break;
                }
                if let Some(user) = users.values().find(|u| u.pubkey == pubkey) {
                    items.push(user.clone());
                }
            }

            let has_more = next_cursor.is_some();

            Ok(UserCursorPage {
                users: items,
                next_cursor,
                has_more,
            })
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
        assert!(matches!(err, AppError::ValidationError { .. }));
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

    #[tokio::test]
    async fn search_users_returns_matching_records() {
        let service = setup_service().await;
        seed_user(&service, ALICE_NPUB, ALICE_PUB).await;
        seed_user(&service, BOB_NPUB, BOB_PUB).await;

        service
            .update_profile(
                ALICE_NPUB,
                UserMetadata {
                    name: Some("alice".to_string()),
                    display_name: Some("Alice Wonderland".to_string()),
                    about: Some("nostr developer".to_string()),
                    picture: None,
                    banner: None,
                    nip05: None,
                    lud16: None,
                },
            )
            .await
            .expect("update alice profile");

        service
            .update_profile(
                BOB_NPUB,
                UserMetadata {
                    name: Some("bob".to_string()),
                    display_name: Some("Bob Smith".to_string()),
                    about: Some("bitcoin enthusiast".to_string()),
                    picture: None,
                    banner: None,
                    nip05: None,
                    lud16: None,
                },
            )
            .await
            .expect("update bob profile");

        let results = service
            .search_users("alice", 10)
            .await
            .expect("search users");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].npub, ALICE_NPUB);

        let bio_match = service
            .search_users("bitcoin", 10)
            .await
            .expect("search by bio");
        assert_eq!(bio_match.len(), 1);
        assert_eq!(bio_match[0].npub, BOB_NPUB);

        let none = service
            .search_users("charlie", 10)
            .await
            .expect("empty search");
        assert!(none.is_empty());
    }
}
