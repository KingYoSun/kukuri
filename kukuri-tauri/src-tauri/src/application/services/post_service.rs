use crate::domain::entities::{Event, Post, User};
use crate::domain::value_objects::{EventId, PublicKey};
use crate::infrastructure::cache::PostCacheService;
use crate::infrastructure::database::{BookmarkRepository, PostRepository};
use crate::infrastructure::p2p::EventDistributor;
use crate::infrastructure::p2p::event_distributor::DistributionStrategy;
use crate::shared::error::AppError;
use nostr_sdk::prelude::*;
use std::sync::Arc;

pub struct PostService {
    repository: Arc<dyn PostRepository>,
    bookmark_repository: Arc<dyn BookmarkRepository>,
    distributor: Arc<dyn EventDistributor>,
    cache: Arc<PostCacheService>,
    keys: Option<Keys>,
}

impl PostService {
    pub fn new(
        repository: Arc<dyn PostRepository>,
        bookmark_repository: Arc<dyn BookmarkRepository>,
        distributor: Arc<dyn EventDistributor>,
    ) -> Self {
        Self {
            repository,
            bookmark_repository,
            distributor,
            cache: Arc::new(PostCacheService::new()),
            keys: None,
        }
    }

    pub fn with_keys(mut self, keys: Keys) -> Self {
        self.keys = Some(keys);
        self
    }

    pub async fn create_post(
        &self,
        content: String,
        author: User,
        topic_id: String,
    ) -> Result<Post, AppError> {
        let mut post = Post::new(content.clone(), author.clone(), topic_id.clone());

        // Save to database
        self.repository.create_post(&post).await?;

        // Convert to Nostr event and distribute
        if let Some(ref keys) = self.keys {
            // Create Nostr event with topic tag
            let tag = Tag::hashtag(topic_id.clone());

            let mut event_builder = EventBuilder::text_note(&content);
            event_builder = event_builder.tag(tag);
            let nostr_event = event_builder.sign_with_keys(keys)?;

            // Convert to domain Event
            let mut event = Event::new(
                1, // Kind 1 for text notes
                content,
                author.pubkey.clone(),
            );
            event.tags = vec![vec!["t".to_string(), topic_id]];

            // Distribute via P2P
            self.distributor
                .distribute(&event, DistributionStrategy::Hybrid)
                .await?;

            // Mark post as synced
            post.mark_as_synced(nostr_event.id.to_hex());
            self.repository.update_post(&post).await?;
        }

        // 新規作成した投稿をキャッシュに保存
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
        if let Some(mut post) = self.repository.get_post(post_id).await? {
            post.increment_likes();
            self.repository.update_post(&post).await?;

            // キャッシュを無効化
            self.cache.remove(post_id).await;

            // Send like event (Nostr reaction)
            if let Some(ref keys) = self.keys {
                let event_id = nostr_sdk::EventId::from_hex(post_id)?;
                // Create a simple reaction event
                let _reaction_event = EventBuilder::text_note("+")
                    .tag(Tag::event(event_id))
                    .sign_with_keys(keys)?;

                // Convert to domain Event and distribute
                let mut event = Event::new(
                    7, // Kind 7 for reactions
                    "+".to_string(),
                    keys.public_key().to_hex(),
                );
                event.tags = vec![vec!["e".to_string(), post_id.to_string()]];

                self.distributor
                    .distribute(&event, DistributionStrategy::Nostr)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn boost_post(&self, post_id: &str) -> Result<(), AppError> {
        if let Some(mut post) = self.repository.get_post(post_id).await? {
            post.increment_boosts();
            self.repository.update_post(&post).await?;

            // キャッシュを無効化
            self.cache.remove(post_id).await;

            // Send boost event (Nostr repost)
            if let Some(ref keys) = self.keys {
                let event_id = nostr_sdk::EventId::from_hex(post_id)?;
                // Create a repost event
                let _repost_event = EventBuilder::text_note("")
                    .tag(Tag::event(event_id))
                    .sign_with_keys(keys)?;

                // Convert to domain Event and distribute
                let mut event = Event::new(
                    6, // Kind 6 for reposts
                    "".to_string(),
                    keys.public_key().to_hex(),
                );
                event.tags = vec![vec!["e".to_string(), post_id.to_string()]];

                self.distributor
                    .distribute(&event, DistributionStrategy::Nostr)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn delete_post(&self, id: &str) -> Result<(), AppError> {
        // Send deletion event
        if let Some(ref keys) = self.keys {
            let event_id = nostr_sdk::EventId::from_hex(id)?;
            // Create a deletion event
            let _deletion_event = EventBuilder::text_note("Post deleted")
                .tag(Tag::event(event_id))
                .sign_with_keys(keys)?;

            // Convert to domain Event and distribute
            let mut event = Event::new(
                5, // Kind 5 for deletions
                "Post deleted".to_string(),
                keys.public_key().to_hex(),
            );
            event.tags = vec![vec!["e".to_string(), id.to_string()]];

            self.distributor
                .distribute(&event, DistributionStrategy::Nostr)
                .await?;
        }

        // Mark as deleted in database
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
        // TODO: Implement reaction logic
        if reaction == "+" {
            self.like_post(post_id).await
        } else {
            // Custom reaction
            Ok(())
        }
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
        let unsync_posts = self.repository.get_unsync_posts().await?;
        let mut synced_count = 0;

        for post in unsync_posts {
            // Convert to Event and distribute
            let mut event = Event::new(
                1, // Kind 1 for text notes
                post.content.clone(),
                post.author.pubkey.clone(), // Use pubkey from author
            );
            event.tags = vec![vec!["t".to_string(), post.topic_id.clone()]];

            // Try to distribute
            if self
                .distributor
                .distribute(&event, DistributionStrategy::Hybrid)
                .await
                .is_ok()
            {
                // Mark as synced
                self.repository
                    .mark_post_synced(&post.id, &event.id)
                    .await?;
                synced_count += 1;
            }
        }

        Ok(synced_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::database::{
        BookmarkRepository, PostRepository, connection_pool::ConnectionPool,
        sqlite_repository::SqliteRepository,
    };
    use async_trait::async_trait;
    use std::sync::Arc;

    struct NoopDistributor;

    #[async_trait]
    impl EventDistributor for NoopDistributor {
        async fn distribute(
            &self,
            _event: &Event,
            _strategy: DistributionStrategy,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }

        async fn receive(&self) -> Result<Option<Event>, Box<dyn std::error::Error + Send + Sync>> {
            Ok(None)
        }

        async fn set_strategy(&self, _strategy: DistributionStrategy) {}

        async fn get_pending_events(
            &self,
        ) -> Result<Vec<Event>, Box<dyn std::error::Error + Send + Sync>> {
            Ok(vec![])
        }

        async fn retry_failed(&self) -> Result<u32, Box<dyn std::error::Error + Send + Sync>> {
            Ok(0)
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
        let distributor: Arc<dyn EventDistributor> = Arc::new(NoopDistributor);

        PostService::new(
            Arc::clone(&repository) as Arc<dyn PostRepository>,
            Arc::clone(&repository) as Arc<dyn BookmarkRepository>,
            distributor,
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
