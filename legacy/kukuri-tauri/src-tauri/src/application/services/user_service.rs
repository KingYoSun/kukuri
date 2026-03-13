use crate::application::ports::repositories::{FollowListSort, UserCursorPage, UserRepository};
use crate::domain::entities::{User, UserMetadata};
use crate::shared::{AppError, ValidationFailureKind};
use chrono::Utc;
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
        let mut user = self
            .repository
            .get_user(npub)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("User not found: {npub}")))?;
        user.update_metadata(metadata);
        self.repository.update_user(&user).await
    }

    pub async fn update_privacy_settings(
        &self,
        npub: &str,
        public_profile: bool,
        show_online_status: bool,
    ) -> Result<(), AppError> {
        let mut user = self
            .repository
            .get_user(npub)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("User not found: {npub}")))?;

        user.public_profile = public_profile;
        user.show_online_status = show_online_status;
        user.updated_at = Utc::now();

        self.repository.update_user(&user).await
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
        sort: FollowListSort,
        search: Option<&str>,
        viewer_npub: Option<&str>,
    ) -> Result<UserCursorPage, AppError> {
        self.ensure_profile_visibility(npub, viewer_npub).await?;
        self.repository
            .get_followers_paginated(npub, cursor, limit, sort, search)
            .await
    }

    pub async fn get_following_paginated(
        &self,
        npub: &str,
        cursor: Option<&str>,
        limit: usize,
        sort: FollowListSort,
        search: Option<&str>,
        viewer_npub: Option<&str>,
    ) -> Result<UserCursorPage, AppError> {
        self.ensure_profile_visibility(npub, viewer_npub).await?;
        self.repository
            .get_following_paginated(npub, cursor, limit, sort, search)
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
                total_count: _,
            } = self
                .get_followers_paginated(
                    npub,
                    cursor.as_deref(),
                    100,
                    FollowListSort::Recent,
                    None,
                    Some(npub),
                )
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
                total_count: _,
            } = self
                .get_following_paginated(
                    npub,
                    cursor.as_deref(),
                    100,
                    FollowListSort::Recent,
                    None,
                    Some(npub),
                )
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

    async fn ensure_profile_visibility(
        &self,
        npub: &str,
        viewer_npub: Option<&str>,
    ) -> Result<(), AppError> {
        let user = self
            .repository
            .get_user(npub)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("User not found: {npub}")))?;

        if !user.public_profile {
            let viewer_matches = viewer_npub
                .map(|viewer| viewer == user.npub)
                .unwrap_or(false);
            if !viewer_matches {
                return Err(AppError::Unauthorized(format!("Profile {npub} is private")));
            }
        }

        Ok(())
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

    fn parse_offset_cursor(cursor: &str) -> Result<usize, AppError> {
        cursor
            .strip_prefix("offset:")
            .ok_or_else(|| AppError::InvalidInput("Invalid cursor format".into()))?
            .parse::<usize>()
            .map_err(|_| AppError::InvalidInput("Invalid cursor offset".into()))
    }

    fn name_key(user: &User) -> String {
        let display_name = user.profile.display_name.trim();
        if display_name.is_empty() {
            user.npub.to_lowercase()
        } else {
            display_name.to_lowercase()
        }
    }
    #[derive(Default)]
    struct InMemoryUserRepository {
        users: RwLock<HashMap<String, User>>,
        follows: RwLock<HashSet<(String, String)>>,
    }

    enum FollowRelationKind {
        Followers,
        Following,
    }

    impl InMemoryUserRepository {
        async fn paginate_relation(
            &self,
            npub: &str,
            cursor: Option<&str>,
            limit: usize,
            sort: FollowListSort,
            search: Option<&str>,
            kind: FollowRelationKind,
        ) -> Result<UserCursorPage, AppError> {
            let users = self.users.read().await;
            let target_pubkey = match users.get(npub) {
                Some(user) => user.pubkey.clone(),
                None => {
                    return Ok(UserCursorPage {
                        users: vec![],
                        next_cursor: None,
                        has_more: false,
                        total_count: 0,
                    });
                }
            };
            let follows = self.follows.read().await;
            let mut entries: Vec<User> = follows
                .iter()
                .filter_map(|(follower, followed)| match kind {
                    FollowRelationKind::Followers if followed == &target_pubkey => {
                        Some(follower.clone())
                    }
                    FollowRelationKind::Following if follower == &target_pubkey => {
                        Some(followed.clone())
                    }
                    _ => None,
                })
                .filter_map(|pubkey| users.values().find(|u| u.pubkey == pubkey).cloned())
                .collect();

            let search_lower = search.map(|s| s.to_lowercase());
            if let Some(search_value) = search_lower.as_ref() {
                entries.retain(|user| {
                    let display = user.profile.display_name.to_lowercase();
                    let npub_value = user.npub.to_lowercase();
                    let pubkey = user.pubkey.to_lowercase();
                    display.contains(search_value)
                        || npub_value.contains(search_value)
                        || pubkey.contains(search_value)
                });
            }

            match sort {
                FollowListSort::Recent => {
                    entries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
                }
                FollowListSort::Oldest => {
                    entries.sort_by(|a, b| a.updated_at.cmp(&b.updated_at));
                }
                FollowListSort::NameAsc => {
                    entries.sort_by_key(name_key);
                }
                FollowListSort::NameDesc => {
                    entries.sort_by_key(|entry| std::cmp::Reverse(name_key(entry)));
                }
            }

            let total_entries = entries.len();
            let mut offset = 0usize;
            if let Some(cursor) = cursor {
                offset = parse_offset_cursor(cursor)?;
            }
            offset = offset.min(total_entries);

            let end = offset.saturating_add(limit);
            let has_more = end < total_entries;
            let next_cursor = if has_more {
                Some(format!("offset:{}", end))
            } else {
                None
            };

            let items: Vec<User> = entries.into_iter().skip(offset).take(limit).collect();

            Ok(UserCursorPage {
                users: items,
                next_cursor,
                has_more,
                total_count: total_entries as u64,
            })
        }
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
            sort: FollowListSort,
            search: Option<&str>,
        ) -> Result<UserCursorPage, AppError> {
            self.paginate_relation(
                npub,
                cursor,
                limit,
                sort,
                search,
                FollowRelationKind::Followers,
            )
            .await
        }

        async fn get_following_paginated(
            &self,
            npub: &str,
            cursor: Option<&str>,
            limit: usize,
            sort: FollowListSort,
            search: Option<&str>,
        ) -> Result<UserCursorPage, AppError> {
            self.paginate_relation(
                npub,
                cursor,
                limit,
                sort,
                search,
                FollowRelationKind::Following,
            )
            .await
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

        async fn list_following_pubkeys(
            &self,
            follower_pubkey: &str,
        ) -> Result<Vec<String>, AppError> {
            let follows = self.follows.read().await;
            Ok(follows
                .iter()
                .filter(|(follower, _)| follower == follower_pubkey)
                .map(|(_, followed)| followed.clone())
                .collect())
        }

        async fn list_follower_pubkeys(
            &self,
            followed_pubkey: &str,
        ) -> Result<Vec<String>, AppError> {
            let follows = self.follows.read().await;
            Ok(follows
                .iter()
                .filter(|(_, target)| target == followed_pubkey)
                .map(|(follower, _)| follower.clone())
                .collect())
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
    async fn followers_paginated_respects_cursor_boundaries() {
        let service = setup_service().await;
        seed_user(&service, ALICE_NPUB, ALICE_PUB).await;
        seed_user(&service, BOB_NPUB, BOB_PUB).await;
        seed_user(&service, "npub1carol", "carol_pub").await;
        seed_user(&service, "npub1dave", "dave_pub").await;

        service
            .follow_user(BOB_NPUB, ALICE_NPUB)
            .await
            .expect("bob follows");
        service
            .follow_user("npub1carol", ALICE_NPUB)
            .await
            .expect("carol follows");
        service
            .follow_user("npub1dave", ALICE_NPUB)
            .await
            .expect("dave follows");

        let page1 = service
            .get_followers_paginated(
                ALICE_NPUB,
                None,
                2,
                FollowListSort::NameAsc,
                None,
                Some(ALICE_NPUB),
            )
            .await
            .expect("page1");
        assert_eq!(page1.users.len(), 2);
        assert_eq!(page1.total_count, 3);
        assert!(page1.has_more);
        let cursor = page1.next_cursor.as_deref().expect("cursor present");

        let page2 = service
            .get_followers_paginated(
                ALICE_NPUB,
                Some(cursor),
                2,
                FollowListSort::NameAsc,
                None,
                Some(ALICE_NPUB),
            )
            .await
            .expect("page2");
        assert_eq!(page2.users.len(), 1);
        assert_eq!(page2.total_count, 3);
        assert!(!page2.has_more);
        assert!(page2.next_cursor.is_none());
    }

    #[tokio::test]
    async fn followers_paginated_private_profile_requires_matching_viewer() {
        let service = setup_service().await;
        seed_user(&service, ALICE_NPUB, ALICE_PUB).await;
        seed_user(&service, BOB_NPUB, BOB_PUB).await;

        service
            .follow_user(BOB_NPUB, ALICE_NPUB)
            .await
            .expect("bob follows");

        service
            .update_privacy_settings(ALICE_NPUB, false, true)
            .await
            .expect("set private");

        let err = service
            .get_followers_paginated(
                ALICE_NPUB,
                None,
                10,
                FollowListSort::Recent,
                None,
                Some(BOB_NPUB),
            )
            .await
            .expect_err("private profile should reject viewer");
        assert!(matches!(err, AppError::Unauthorized(_)));

        let owner_view = service
            .get_followers_paginated(
                ALICE_NPUB,
                None,
                10,
                FollowListSort::Recent,
                None,
                Some(ALICE_NPUB),
            )
            .await
            .expect("owner can view");
        assert_eq!(owner_view.total_count, 1);
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
                    public_profile: None,
                    show_online_status: None,
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
                    public_profile: None,
                    show_online_status: None,
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

    #[tokio::test]
    async fn update_privacy_settings_persists_flags() {
        let service = setup_service().await;
        seed_user(&service, ALICE_NPUB, ALICE_PUB).await;

        service
            .update_privacy_settings(ALICE_NPUB, false, true)
            .await
            .expect("update privacy settings");

        let updated = service
            .get_user(ALICE_NPUB)
            .await
            .expect("fetch user")
            .expect("user exists");

        assert!(!updated.public_profile);
        assert!(updated.show_online_status);
    }

    #[tokio::test]
    async fn update_privacy_settings_missing_user_returns_error() {
        let service = setup_service().await;

        let err = service
            .update_privacy_settings("npub1missing", true, false)
            .await
            .expect_err("missing user should error");

        assert!(matches!(err, AppError::NotFound(_)));
    }
}
