use crate::application::ports::repositories::{BookmarkRepository, PostRepository};
use crate::application::services::event_service::EventServiceTrait;
use crate::domain::entities::{Post, User};
use crate::domain::value_objects::{EventId, PublicKey};
use crate::infrastructure::cache::PostCacheService;
use crate::shared::error::AppError;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::warn;

pub struct PostService {
    repository: Arc<dyn PostRepository>,
    bookmark_repository: Arc<dyn BookmarkRepository>,
    event_service: Arc<dyn EventServiceTrait>,
    cache: Arc<PostCacheService>,
}

impl PostService {
    pub fn new(
        repository: Arc<dyn PostRepository>,
        bookmark_repository: Arc<dyn BookmarkRepository>,
        event_service: Arc<dyn EventServiceTrait>,
    ) -> Self {
        Self {
            repository,
            bookmark_repository,
            event_service,
            cache: Arc::new(PostCacheService::new()),
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
        // TODO: トピック別の投稿キャッシュを実装
        // 現在は直接DBから取得（キャッシュの無効化が複雑なため）
        let posts = self.repository.get_posts_by_topic(topic_id, limit).await?;

        // 個別の投稿をキャッシュに保存
        for post in &posts {
            self.cache.add(post.clone()).await;
        }

        Ok(posts)
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
        self.repository.delete_post(id).await
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
        let event_id = EventId::from_hex(post_id).map_err(AppError::ValidationError)?;
        let public_key = PublicKey::from_hex_str(user_pubkey).map_err(AppError::ValidationError)?;

        self.bookmark_repository
            .create_bookmark(&public_key, &event_id)
            .await?;
        // キャッシュを無効化して次回取得時に最新状態を反映
        self.cache.remove(post_id).await;
        Ok(())
    }

    pub async fn unbookmark_post(&self, post_id: &str, user_pubkey: &str) -> Result<(), AppError> {
        let event_id = EventId::from_hex(post_id).map_err(AppError::ValidationError)?;
        let public_key = PublicKey::from_hex_str(user_pubkey).map_err(AppError::ValidationError)?;

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
        let public_key = PublicKey::from_hex_str(user_pubkey).map_err(AppError::ValidationError)?;

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
    use crate::application::ports::repositories::{BookmarkRepository, PostRepository};
    use crate::application::services::SubscriptionRecord;
    use crate::infrastructure::database::{
        connection_pool::ConnectionPool, sqlite_repository::SqliteRepository,
    };
    use crate::presentation::dto::event::NostrMetadataDto;
    use std::sync::Arc;

    #[derive(Default)]
    struct TestEventService;

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
            Ok(EventId::generate())
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

    async fn setup_post_service() -> PostService {
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
        let event_service: Arc<dyn EventServiceTrait> = Arc::new(TestEventService::default());

        PostService::new(
            Arc::clone(&repository) as Arc<dyn PostRepository>,
            Arc::clone(&repository) as Arc<dyn BookmarkRepository>,
            event_service,
        )
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
}
