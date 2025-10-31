use crate::application::ports::cache::PostCache;
use crate::application::ports::repositories::{BookmarkRepository, PostRepository};
use crate::application::services::event_service::EventServiceTrait;
use crate::domain::entities::{Post, User};
use crate::domain::value_objects::{EventId, PublicKey};
use crate::shared::{AppError, ValidationFailureKind};
use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::Arc;
use tracing::warn;

pub struct PostService {
    repository: Arc<dyn PostRepository>,
    bookmark_repository: Arc<dyn BookmarkRepository>,
    event_service: Arc<dyn EventServiceTrait>,
    cache: Arc<dyn PostCache>,
}

impl PostService {
    pub fn new(
        repository: Arc<dyn PostRepository>,
        bookmark_repository: Arc<dyn BookmarkRepository>,
        event_service: Arc<dyn EventServiceTrait>,
        cache: Arc<dyn PostCache>,
    ) -> Self {
        Self {
            repository,
            bookmark_repository,
            event_service,
            cache,
        }
    }

    pub async fn create_post(
        &self,
        content: String,
        author: User,
        topic_id: String,
    ) -> Result<Post, AppError> {
        let mut post = Post::new(content.clone(), author, topic_id.clone());
        self.repository.create_post(&post).await?;

        match self
            .event_service
            .publish_topic_post(&topic_id, &content, None)
            .await
        {
            Ok(event_id) => {
                let event_hex = event_id.to_string();
                post.mark_as_synced(event_hex.clone());
                self.repository
                    .mark_post_synced(&post.id, &event_hex)
                    .await?;
            }
            Err(err) => {
                self.cache.add(post.clone()).await;
                return Err(err);
            }
        }

        self.cache.add(post.clone()).await;
        Ok(post)
    }

    pub async fn get_post(&self, id: &str) -> Result<Option<Post>, AppError> {
        // キャッシュから取得を試みる
        if let Some(post) = self.cache.get(id).await {
            return Ok(Some(post));
        }

        // キャッシュにない場合はDBから取得
        let post = self.repository.get_post(id).await?;

        // キャッシュに保存
        if let Some(ref p) = post {
            self.cache.add(p.clone()).await;
        }

        Ok(post)
    }

    pub async fn get_posts_by_topic(
        &self,
        topic_id: &str,
        limit: usize,
    ) -> Result<Vec<Post>, AppError> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let cached_all = self.cache.get_by_topic(topic_id, usize::MAX).await;
        if cached_all.len() >= limit {
            return Ok(cached_all.into_iter().take(limit).collect());
        }

        let mut posts = self.repository.get_posts_by_topic(topic_id, limit).await?;

        if !cached_all.is_empty() {
            let mut seen: HashSet<String> = posts.iter().map(|post| post.id.clone()).collect();
            for cached in cached_all {
                if seen.insert(cached.id.clone()) {
                    posts.push(cached);
                }
            }
        }

        posts.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        // キャッシュにも最新の取得結果を反映
        self.cache.set_topic_posts(topic_id, posts.clone()).await;

        Ok(posts.into_iter().take(limit).collect())
    }

    pub async fn like_post(&self, post_id: &str) -> Result<(), AppError> {
        self.react_to_post(post_id, "+").await
    }

    pub async fn boost_post(&self, post_id: &str) -> Result<(), AppError> {
        self.event_service.boost_post(post_id).await?;

        if let Some(mut post) = self.repository.get_post(post_id).await? {
            post.increment_boosts();
            self.repository.update_post(&post).await?;
            self.cache.remove(post_id).await;
        }

        Ok(())
    }

    pub async fn delete_post(&self, id: &str) -> Result<(), AppError> {
        self.event_service
            .delete_events(vec![id.to_string()], Some("Post deleted".to_string()))
            .await?;
        self.repository.delete_post(id).await?;
        self.cache.remove(id).await;
        Ok(())
    }

    pub async fn get_posts_by_author(
        &self,
        author_pubkey: &str,
        limit: usize,
    ) -> Result<Vec<Post>, AppError> {
        self.repository
            .get_posts_by_author(author_pubkey, limit)
            .await
    }

    pub async fn get_recent_posts(&self, limit: usize) -> Result<Vec<Post>, AppError> {
        self.repository.get_recent_posts(limit).await
    }

    pub async fn react_to_post(&self, post_id: &str, reaction: &str) -> Result<(), AppError> {
        self.event_service.send_reaction(post_id, reaction).await?;

        if reaction == "+" {
            if let Some(mut post) = self.repository.get_post(post_id).await? {
                post.increment_likes();
                self.repository.update_post(&post).await?;
                self.cache.remove(post_id).await;
            }
        }

        Ok(())
    }

    pub async fn bookmark_post(&self, post_id: &str, user_pubkey: &str) -> Result<(), AppError> {
        let event_id = EventId::from_hex(post_id)
            .map_err(|err| AppError::validation(ValidationFailureKind::Generic, err))?;
        let public_key = PublicKey::from_hex_str(user_pubkey)
            .map_err(|err| AppError::validation(ValidationFailureKind::Generic, err))?;

        self.bookmark_repository
            .create_bookmark(&public_key, &event_id)
            .await?;
        // キャッシュを無効化して次回取得時に最新状態を反映
        self.cache.remove(post_id).await;
        Ok(())
    }

    pub async fn unbookmark_post(&self, post_id: &str, user_pubkey: &str) -> Result<(), AppError> {
        let event_id = EventId::from_hex(post_id)
            .map_err(|err| AppError::validation(ValidationFailureKind::Generic, err))?;
        let public_key = PublicKey::from_hex_str(user_pubkey)
            .map_err(|err| AppError::validation(ValidationFailureKind::Generic, err))?;

        self.bookmark_repository
            .delete_bookmark(&public_key, &event_id)
            .await?;
        // キャッシュを無効化して次回取得時に最新状態を反映
        self.cache.remove(post_id).await;
        Ok(())
    }

    pub async fn get_bookmarked_post_ids(
        &self,
        user_pubkey: &str,
    ) -> Result<Vec<String>, AppError> {
        let public_key = PublicKey::from_hex_str(user_pubkey)
            .map_err(|err| AppError::validation(ValidationFailureKind::Generic, err))?;

        let bookmarks = self.bookmark_repository.list_bookmarks(&public_key).await?;

        Ok(bookmarks
            .into_iter()
            .map(|bookmark| bookmark.post_id().as_str().to_string())
            .collect())
    }

    pub async fn sync_pending_posts(&self) -> Result<u32, AppError> {
        let unsynced_posts = self.repository.get_unsync_posts().await?;
        let mut synced_count = 0;

        for mut post in unsynced_posts {
            match self
                .event_service
                .publish_topic_post(&post.topic_id, &post.content, None)
                .await
            {
                Ok(event_id) => {
                    let event_hex = event_id.to_string();
                    post.mark_as_synced(event_hex.clone());
                    self.repository
                        .mark_post_synced(&post.id, &event_hex)
                        .await?;
                    synced_count += 1;
                }
                Err(err) => {
                    warn!("failed to sync post {post_id}: {err}", post_id = post.id);
                }
            }
        }

        Ok(synced_count)
    }
}

#[async_trait]
impl super::sync_service::SyncParticipant for PostService {
    async fn sync_pending(&self) -> Result<u32, AppError> {
        self.sync_pending_posts().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::cache::PostCache;
    use crate::application::ports::repositories::{BookmarkRepository, PostRepository};
    use crate::application::services::SubscriptionRecord;
    use crate::infrastructure::cache::PostCacheService;
    use crate::infrastructure::database::Repository;
    use crate::infrastructure::database::{
        connection_pool::ConnectionPool, sqlite_repository::SqliteRepository,
    };
    use crate::presentation::dto::event::NostrMetadataDto;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    struct TestEventService {
        publish_topic_post_result: Mutex<Option<Result<EventId, AppError>>>,
    }

    impl TestEventService {
        fn new() -> Self {
            Self {
                publish_topic_post_result: Mutex::new(None),
            }
        }

        fn with_publish_result(result: Result<EventId, AppError>) -> Self {
            Self {
                publish_topic_post_result: Mutex::new(Some(result)),
            }
        }

        async fn next_publish_result(&self) -> Result<EventId, AppError> {
            let mut guard = self.publish_topic_post_result.lock().await;
            guard.take().unwrap_or_else(|| Ok(EventId::generate()))
        }
    }

    impl Default for TestEventService {
        fn default() -> Self {
            Self::new()
        }
    }

    #[async_trait]
    impl EventServiceTrait for TestEventService {
        async fn initialize(&self) -> Result<(), AppError> {
            Ok(())
        }
        async fn publish_text_note(&self, _: &str) -> Result<EventId, AppError> {
            Ok(EventId::generate())
        }
        async fn publish_topic_post(
            &self,
            _: &str,
            _: &str,
            _: Option<&str>,
        ) -> Result<EventId, AppError> {
            self.next_publish_result().await
        }
        async fn send_reaction(&self, _: &str, _: &str) -> Result<EventId, AppError> {
            Ok(EventId::generate())
        }
        async fn update_metadata(&self, _: NostrMetadataDto) -> Result<EventId, AppError> {
            Ok(EventId::generate())
        }
        async fn subscribe_to_topic(&self, _: &str) -> Result<(), AppError> {
            Ok(())
        }
        async fn subscribe_to_user(&self, _: &str) -> Result<(), AppError> {
            Ok(())
        }
        async fn get_public_key(&self) -> Result<Option<String>, AppError> {
            Ok(None)
        }
        async fn boost_post(&self, _: &str) -> Result<EventId, AppError> {
            Ok(EventId::generate())
        }
        async fn delete_events(
            &self,
            _: Vec<String>,
            _: Option<String>,
        ) -> Result<EventId, AppError> {
            Ok(EventId::generate())
        }
        async fn disconnect(&self) -> Result<(), AppError> {
            Ok(())
        }
        async fn set_default_p2p_topic(&self, _: &str) -> Result<(), AppError> {
            Ok(())
        }
        async fn list_subscriptions(&self) -> Result<Vec<SubscriptionRecord>, AppError> {
            Ok(vec![])
        }
    }

    async fn setup_post_service_with_deps(
        event_service: Arc<dyn EventServiceTrait>,
    ) -> (PostService, Arc<SqliteRepository>, Arc<PostCacheService>) {
        let pool = ConnectionPool::new("sqlite::memory:?cache=shared")
            .await
            .expect("failed to create pool");

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS bookmarks (
                id TEXT PRIMARY KEY,
                user_pubkey TEXT NOT NULL,
                post_id TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                UNIQUE(user_pubkey, post_id)
            )
            "#,
        )
        .execute(pool.get_pool())
        .await
        .expect("failed to create bookmarks table");

        let repository = Arc::new(SqliteRepository::new(pool));
        repository
            .initialize()
            .await
            .expect("failed to initialize repository schema");
        let cache = Arc::new(PostCacheService::new());

        let service = PostService::new(
            Arc::clone(&repository) as Arc<dyn PostRepository>,
            Arc::clone(&repository) as Arc<dyn BookmarkRepository>,
            event_service,
            Arc::clone(&cache) as Arc<dyn PostCache>,
        );

        (service, repository, cache)
    }

    async fn setup_post_service() -> PostService {
        let event_service: Arc<dyn EventServiceTrait> = Arc::new(TestEventService::default());
        setup_post_service_with_deps(event_service).await.0
    }

    fn sample_user() -> User {
        User {
            npub: "npub1test".to_string(),
            pubkey: "test_pubkey".to_string(),
            profile: crate::domain::entities::user::UserProfile {
                display_name: "Test User".to_string(),
                bio: "Test bio".to_string(),
                avatar_url: None,
            },
            name: Some("Test User".to_string()),
            nip05: None,
            lud16: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    const SAMPLE_PUBKEY: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[tokio::test]
    async fn bookmark_flow_roundtrip() {
        let service = setup_post_service().await;
        let event_id = EventId::generate();
        let event_hex = event_id.to_hex();

        service
            .bookmark_post(&event_hex, SAMPLE_PUBKEY)
            .await
            .expect("bookmark should succeed");

        let bookmarked = service
            .get_bookmarked_post_ids(SAMPLE_PUBKEY)
            .await
            .expect("list bookmarks");
        assert_eq!(bookmarked, vec![event_hex.clone()]);

        service
            .unbookmark_post(&event_hex, SAMPLE_PUBKEY)
            .await
            .expect("unbookmark should succeed");

        let bookmarked = service
            .get_bookmarked_post_ids(SAMPLE_PUBKEY)
            .await
            .expect("list bookmarks after removal");
        assert!(bookmarked.is_empty());
    }

    #[tokio::test]
    async fn create_post_marks_synced_after_publish() {
        let expected_event = EventId::generate();
        let event_service: Arc<dyn EventServiceTrait> = Arc::new(
            TestEventService::with_publish_result(Ok(expected_event.clone())),
        );

        let (service, repository, cache) = setup_post_service_with_deps(event_service).await;
        let expected_event_hex = expected_event.to_string();

        let post = service
            .create_post("hello world".into(), sample_user(), "topic-sync".into())
            .await
            .expect("post creation succeeds");

        assert!(post.is_synced);
        assert_eq!(post.event_id.as_deref(), Some(expected_event_hex.as_str()));

        let stored = repository
            .get_post(&post.id)
            .await
            .expect("db query succeeds")
            .expect("post present in db");
        assert_eq!(stored.id, post.id);

        let unsynced = repository
            .get_unsync_posts()
            .await
            .expect("unsynced query succeeds");
        assert!(unsynced.is_empty(), "all posts should be marked synced");

        let cached = cache
            .get(&post.id)
            .await
            .expect("post cached after creation");
        assert_eq!(cached.event_id.as_deref(), post.event_id.as_deref());
    }

    #[tokio::test]
    async fn create_post_caches_on_publish_failure() {
        let event_service: Arc<dyn EventServiceTrait> = Arc::new(
            TestEventService::with_publish_result(Err(AppError::NostrError("failed".into()))),
        );
        let (service, repository, cache) = setup_post_service_with_deps(event_service).await;

        let err = service
            .create_post("offline".into(), sample_user(), "topic-offline".into())
            .await
            .expect_err("publish failure propagates");
        assert!(matches!(err, AppError::NostrError(_)));

        let stored = repository
            .get_posts_by_topic("topic-offline", 10)
            .await
            .expect("query succeeds");
        assert_eq!(stored.len(), 1);
        let stored_post = &stored[0];
        assert!(!stored_post.is_synced);
        assert!(stored_post.event_id.is_none());

        let unsynced = repository
            .get_unsync_posts()
            .await
            .expect("unsynced query succeeds");
        assert_eq!(
            unsynced.len(),
            1,
            "failed publish should remain in unsynced queue"
        );

        let cached = cache
            .get(&stored_post.id)
            .await
            .expect("unsynced post cached for retry");
        assert_eq!(cached.id, stored_post.id);
        assert!(!cached.is_synced);
    }

    #[tokio::test]
    async fn get_posts_by_topic_prefers_cache_when_available() {
        let event_service: Arc<dyn EventServiceTrait> = Arc::new(TestEventService::default());
        let (service, repository, cache) = setup_post_service_with_deps(event_service).await;
        let topic_id = "topic-cache";

        let mut posts = Vec::new();
        for (idx, ts) in [10_i64, 20, 30].into_iter().enumerate() {
            let mut post = Post::new(format!("post-{idx}"), sample_user(), topic_id.to_string());
            post.created_at = Utc.timestamp_opt(ts, 0).unwrap();
            post.is_synced = true;
            posts.push(post);
        }

        for post in &posts {
            repository.create_post(post).await.expect("seed repository");
        }

        let initial = service
            .get_posts_by_topic(topic_id, 3)
            .await
            .expect("initial fetch");
        assert_eq!(initial.len(), 3);

        // DBから削除してもキャッシュから取得できることを確認
        for post in &initial {
            repository
                .delete_post(&post.id)
                .await
                .expect("delete seeded post");
        }

        let cached = service
            .get_posts_by_topic(topic_id, 2)
            .await
            .expect("fetch from cache");
        assert_eq!(cached.len(), 2);
        assert_eq!(cached[0].id, initial[0].id);
        assert_eq!(cached[1].id, initial[1].id);

        let cached_full = cache.get_by_topic(topic_id, 5).await;
        assert_eq!(cached_full.len(), 3);
    }

    #[tokio::test]
    async fn get_posts_by_topic_merges_cached_entries_not_in_db() {
        let event_service: Arc<dyn EventServiceTrait> = Arc::new(TestEventService::default());
        let (service, repository, cache) = setup_post_service_with_deps(event_service).await;
        let topic_id = "topic-cache-merge";

        let mut db_post = Post::new("db-post".into(), sample_user(), topic_id.to_string());
        db_post.created_at = Utc.timestamp_opt(50, 0).unwrap();
        db_post.is_synced = true;
        repository
            .create_post(&db_post)
            .await
            .expect("seed repository with db post");

        let mut cached_only = Post::new("cached-only".into(), sample_user(), topic_id.to_string());
        cached_only.created_at = Utc.timestamp_opt(200, 0).unwrap();
        cache.add(cached_only.clone()).await;

        let result = service
            .get_posts_by_topic(topic_id, 2)
            .await
            .expect("fetch posts should include cached-only entry");

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].content, "cached-only");
        assert_eq!(result[1].content, "db-post");

        let cached_snapshot = cache.get_by_topic(topic_id, 5).await;
        assert!(
            cached_snapshot.iter().any(|post| post.id == cached_only.id),
            "cached-only post should remain cached after merge"
        );
    }

    #[tokio::test]
    async fn delete_post_removes_from_cache() {
        let (service, _repository, cache) =
            setup_post_service_with_deps(Arc::new(TestEventService::default())).await;

        let post = service
            .create_post("to delete".into(), sample_user(), "topic-del".into())
            .await
            .expect("post creation succeeds");

        assert!(
            cache.get(&post.id).await.is_some(),
            "post should be present in cache after creation"
        );

        service
            .delete_post(&post.id)
            .await
            .expect("delete_post should succeed");

        assert!(
            cache.get(&post.id).await.is_none(),
            "post should be evicted from cache after deletion"
        );
    }
}
