use crate::domain::entities::{Event, Post, User};
use crate::domain::value_objects::EventId;
use crate::infrastructure::database::PostRepository;
use crate::infrastructure::p2p::{EventDistributor, DistributionStrategy};
use crate::infrastructure::cache::PostCacheService;
use nostr_sdk::prelude::*;
use std::sync::Arc;

pub struct PostService {
    repository: Arc<dyn PostRepository>,
    distributor: Arc<dyn EventDistributor>,
    cache: Arc<PostCacheService>,
    keys: Option<Keys>,
}

impl PostService {
    pub fn new(repository: Arc<dyn PostRepository>, distributor: Arc<dyn EventDistributor>) -> Self {
        Self {
            repository,
            distributor,
            cache: Arc::new(PostCacheService::new()),
            keys: None,
        }
    }
    
    pub fn with_keys(mut self, keys: Keys) -> Self {
        self.keys = Some(keys);
        self
    }

    pub async fn create_post(&self, content: String, author: User, topic_id: String) -> Result<Post, Box<dyn std::error::Error>> {
        let mut post = Post::new(content.clone(), author.clone(), topic_id.clone());
        
        // Save to database
        self.repository.create_post(&post).await?;
        
        // Convert to Nostr event and distribute
        if let Some(ref keys) = self.keys {
            // Create Nostr event with topic tag
            let tags = vec![
                Tag::custom(TagKind::Custom("t".to_string()), vec![topic_id.clone()]),
            ];
            
            let event_builder = EventBuilder::text_note(&content, tags);
            let nostr_event = event_builder.sign_with_keys(keys)?;
            
            // Convert to domain Event
            let event = Event::new(
                author.pubkey(),
                content,
                1, // Kind 1 for text notes
                vec![vec!["t".to_string(), topic_id]],
            );
            
            // Distribute via P2P
            self.distributor.distribute(&event, DistributionStrategy::Hybrid).await?;
            
            // Mark post as synced
            post.mark_as_synced(nostr_event.id.to_hex());
            self.repository.update_post(&post).await?;
        }
        
        // 新規作成した投稿をキャッシュに保存
        self.cache.cache_post(post.clone()).await;
        
        Ok(post)
    }

    pub async fn get_post(&self, id: &str) -> Result<Option<Post>, Box<dyn std::error::Error>> {
        // キャッシュから取得を試みる
        if let Some(post) = self.cache.get_post(id).await {
            return Ok(Some(post));
        }
        
        // キャッシュにない場合はDBから取得
        let post = self.repository.get_post(id).await?;
        
        // キャッシュに保存
        if let Some(ref p) = post {
            self.cache.cache_post(p.clone()).await;
        }
        
        Ok(post)
    }

    pub async fn get_posts_by_topic(&self, topic_id: &str, limit: usize) -> Result<Vec<Post>, Box<dyn std::error::Error>> {
        // TODO: トピック別の投稿キャッシュを実装
        // 現在は直接DBから取得（キャッシュの無効化が複雑なため）
        let posts = self.repository.get_posts_by_topic(topic_id, limit).await?;
        
        // 個別の投稿をキャッシュに保存
        for post in &posts {
            self.cache.cache_post(post.clone()).await;
        }
        
        Ok(posts)
    }

    pub async fn like_post(&self, post_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(mut post) = self.repository.get_post(post_id).await? {
            post.increment_likes();
            self.repository.update_post(&post).await?;
            
            // キャッシュを無効化
            self.cache.invalidate_post(post_id).await;
            
            // Send like event (Nostr reaction)
            if let Some(ref keys) = self.keys {
                let event_id = nostr_sdk::EventId::from_hex(post_id)?;
                let reaction_event = EventBuilder::reaction(event_id, "+")
                    .sign_with_keys(keys)?;
                
                // Convert to domain Event and distribute
                let event = Event::new(
                    keys.public_key().to_hex(),
                    "+".to_string(),
                    7, // Kind 7 for reactions
                    vec![vec!["e".to_string(), post_id.to_string()]],
                );
                
                self.distributor.distribute(&event, DistributionStrategy::Nostr).await?;
            }
        }
        Ok(())
    }

    pub async fn boost_post(&self, post_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(mut post) = self.repository.get_post(post_id).await? {
            post.increment_boosts();
            self.repository.update_post(&post).await?;
            
            // キャッシュを無効化
            self.cache.invalidate_post(post_id).await;
            
            // Send boost event (Nostr repost)
            if let Some(ref keys) = self.keys {
                let event_id = nostr_sdk::EventId::from_hex(post_id)?;
                let repost_event = EventBuilder::repost(event_id, None)
                    .sign_with_keys(keys)?;
                
                // Convert to domain Event and distribute
                let event = Event::new(
                    keys.public_key().to_hex(),
                    "".to_string(),
                    6, // Kind 6 for reposts
                    vec![vec!["e".to_string(), post_id.to_string()]],
                );
                
                self.distributor.distribute(&event, DistributionStrategy::Nostr).await?;
            }
        }
        Ok(())
    }

    pub async fn delete_post(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Send deletion event
        if let Some(ref keys) = self.keys {
            let event_id = nostr_sdk::EventId::from_hex(id)?;
            let deletion_event = EventBuilder::delete(vec![event_id], Some("Post deleted"))
                .sign_with_keys(keys)?;
            
            // Convert to domain Event and distribute
            let event = Event::new(
                keys.public_key().to_hex(),
                "Post deleted".to_string(),
                5, // Kind 5 for deletions
                vec![vec!["e".to_string(), id.to_string()]],
            );
            
            self.distributor.distribute(&event, DistributionStrategy::Nostr).await?;
        }
        
        // Mark as deleted in database
        self.repository.delete_post(id).await
    }

    pub async fn sync_pending_posts(&self) -> Result<u32, Box<dyn std::error::Error>> {
        let unsync_posts = self.repository.get_unsync_posts().await?;
        let mut synced_count = 0;
        
        for post in unsync_posts {
            // Convert to Event and distribute
            let event = Event::new(
                post.author.pubkey(),
                post.content.clone(),
                1, // Kind 1 for text notes
                vec![vec!["t".to_string(), post.topic_id.clone()]],
            );
            
            // Try to distribute
            if self.distributor.distribute(&event, DistributionStrategy::Hybrid).await.is_ok() {
                // Mark as synced
                self.repository.mark_post_synced(&post.id, &event.id.to_string()).await?;
                synced_count += 1;
            }
        }
        
        Ok(synced_count)
    }
}