use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use crate::domain::entities::Post;

/// 投稿キャッシュサービス
#[derive(Clone)]
pub struct PostCacheService {
    cache: Arc<RwLock<HashMap<String, Post>>>,
}

impl PostCacheService {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 投稿をキャッシュに追加
    pub async fn add(&self, post: Post) {
        let mut cache = self.cache.write().await;
        cache.insert(post.id.clone(), post);
    }

    /// 複数の投稿をキャッシュに追加
    pub async fn add_many(&self, posts: Vec<Post>) {
        let mut cache = self.cache.write().await;
        for post in posts {
            cache.insert(post.id.clone(), post);
        }
    }

    /// IDで投稿を取得
    pub async fn get(&self, id: &str) -> Option<Post> {
        let cache = self.cache.read().await;
        cache.get(id).cloned()
    }

    /// 複数のIDで投稿を取得
    pub async fn get_many(&self, ids: &[String]) -> Vec<Post> {
        let cache = self.cache.read().await;
        ids.iter()
            .filter_map(|id| cache.get(id).cloned())
            .collect()
    }

    /// トピックIDで投稿を取得
    pub async fn get_by_topic(&self, topic_id: &str) -> Vec<Post> {
        let cache = self.cache.read().await;
        cache
            .values()
            .filter(|post| post.topic_id == topic_id)
            .cloned()
            .collect()
    }

    /// 投稿を削除
    pub async fn remove(&self, id: &str) -> Option<Post> {
        let mut cache = self.cache.write().await;
        cache.remove(id)
    }

    /// キャッシュをクリア
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// キャッシュサイズを取得
    pub async fn size(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }
}

impl Default for PostCacheService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_post(id: &str, topic_id: &str) -> Post {
        Post {
            id: id.to_string(),
            pubkey: "test_pubkey".to_string(),
            content: "Test content".to_string(),
            created_at: chrono::Utc::now().timestamp(),
            topic_id: topic_id.to_string(),
            reply_to: None,
            is_synced: true,
            reactions: 0,
            replies: 0,
            reposts: 0,
        }
    }

    #[tokio::test]
    async fn test_add_and_get() {
        let cache = PostCacheService::new();
        let post = create_test_post("1", "topic1");
        
        cache.add(post.clone()).await;
        let retrieved = cache.get("1").await;
        
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "1");
    }

    #[tokio::test]
    async fn test_add_many() {
        let cache = PostCacheService::new();
        let posts = vec![
            create_test_post("1", "topic1"),
            create_test_post("2", "topic1"),
            create_test_post("3", "topic2"),
        ];
        
        cache.add_many(posts).await;
        assert_eq!(cache.size().await, 3);
    }

    #[tokio::test]
    async fn test_get_by_topic() {
        let cache = PostCacheService::new();
        let posts = vec![
            create_test_post("1", "topic1"),
            create_test_post("2", "topic1"),
            create_test_post("3", "topic2"),
        ];
        
        cache.add_many(posts).await;
        let topic1_posts = cache.get_by_topic("topic1").await;
        
        assert_eq!(topic1_posts.len(), 2);
        assert!(topic1_posts.iter().all(|p| p.topic_id == "topic1"));
    }

    #[tokio::test]
    async fn test_remove() {
        let cache = PostCacheService::new();
        let post = create_test_post("1", "topic1");
        
        cache.add(post.clone()).await;
        let removed = cache.remove("1").await;
        
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().id, "1");
        assert!(cache.get("1").await.is_none());
    }

    #[tokio::test]
    async fn test_clear() {
        let cache = PostCacheService::new();
        let posts = vec![
            create_test_post("1", "topic1"),
            create_test_post("2", "topic1"),
            create_test_post("3", "topic2"),
        ];
        
        cache.add_many(posts).await;
        assert_eq!(cache.size().await, 3);
        
        cache.clear().await;
        assert_eq!(cache.size().await, 0);
    }
}