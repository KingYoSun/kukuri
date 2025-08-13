use crate::domain::entities::{Post, User};
use crate::infrastructure::database::PostRepository;
use crate::infrastructure::p2p::EventDistributor;
use std::sync::Arc;

pub struct PostService {
    repository: Arc<dyn PostRepository>,
    distributor: Arc<dyn EventDistributor>,
}

impl PostService {
    pub fn new(repository: Arc<dyn PostRepository>, distributor: Arc<dyn EventDistributor>) -> Self {
        Self {
            repository,
            distributor,
        }
    }

    pub async fn create_post(&self, content: String, author: User, topic_id: String) -> Result<Post, Box<dyn std::error::Error>> {
        let mut post = Post::new(content, author, topic_id);
        
        // Save to database
        self.repository.create_post(&post).await?;
        
        // Convert to event and distribute
        // TODO: Convert post to Nostr event
        // self.distributor.distribute(&event, DistributionStrategy::Hybrid).await?;
        
        post.mark_as_synced(post.id.clone());
        self.repository.update_post(&post).await?;
        
        Ok(post)
    }

    pub async fn get_post(&self, id: &str) -> Result<Option<Post>, Box<dyn std::error::Error>> {
        self.repository.get_post(id).await
    }

    pub async fn get_posts_by_topic(&self, topic_id: &str, limit: usize) -> Result<Vec<Post>, Box<dyn std::error::Error>> {
        self.repository.get_posts_by_topic(topic_id, limit).await
    }

    pub async fn like_post(&self, post_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(mut post) = self.repository.get_post(post_id).await? {
            post.increment_likes();
            self.repository.update_post(&post).await?;
            
            // TODO: Send like event
        }
        Ok(())
    }

    pub async fn boost_post(&self, post_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(mut post) = self.repository.get_post(post_id).await? {
            post.increment_boosts();
            self.repository.update_post(&post).await?;
            
            // TODO: Send boost event
        }
        Ok(())
    }

    pub async fn delete_post(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.repository.delete_post(id).await
    }

    pub async fn sync_pending_posts(&self) -> Result<u32, Box<dyn std::error::Error>> {
        let unsync_posts = self.repository.get_unsync_posts().await?;
        let mut synced_count = 0;
        
        for post in unsync_posts {
            // TODO: Convert and distribute
            synced_count += 1;
        }
        
        Ok(synced_count)
    }
}