use crate::application::ports::cache::PostCache;
use crate::domain::entities::Post;
use async_trait::async_trait;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

const MAX_TOPIC_CACHE: usize = 200;

#[derive(Clone)]
pub struct PostCacheService {
    inner: Arc<RwLock<PostCacheInner>>,
}

struct PostCacheInner {
    posts_by_id: HashMap<String, Post>,
    topic_index: HashMap<String, VecDeque<(String, i64)>>,
}

impl Default for PostCacheService {
    fn default() -> Self {
        Self::new()
    }
}

impl PostCacheService {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(PostCacheInner {
                posts_by_id: HashMap::new(),
                topic_index: HashMap::new(),
            })),
        }
    }

    pub async fn add(&self, post: Post) {
        Self::add_internal(&self.inner, post).await;
    }

    pub async fn add_many(&self, posts: Vec<Post>) {
        for post in posts {
            Self::add_internal(&self.inner, post).await;
        }
    }

    pub async fn get(&self, id: &str) -> Option<Post> {
        let inner = self.inner.read().await;
        inner.posts_by_id.get(id).cloned()
    }

    pub async fn get_many(&self, ids: &[String]) -> Vec<Post> {
        let inner = self.inner.read().await;
        ids.iter()
            .filter_map(|id| inner.posts_by_id.get(id).cloned())
            .collect()
    }

    pub async fn get_by_topic(&self, topic_id: &str, limit: usize) -> Vec<Post> {
        let inner = self.inner.read().await;
        inner
            .topic_index
            .get(topic_id)
            .into_iter()
            .flat_map(|entries| {
                entries
                    .iter()
                    .filter_map(|(post_id, _)| inner.posts_by_id.get(post_id))
                    .take(limit)
                    .cloned()
            })
            .collect()
    }

    pub async fn set_topic_posts(&self, topic_id: &str, posts: Vec<Post>) {
        let mut inner = self.inner.write().await;

        if let Some(entries) = inner.topic_index.remove(topic_id) {
            for (post_id, _) in entries {
                if let Some(existing) = inner.posts_by_id.get(&post_id) {
                    if existing.topic_id == topic_id {
                        inner.posts_by_id.remove(&post_id);
                    }
                } else {
                    inner.posts_by_id.remove(&post_id);
                }
            }
        }

        let mut sorted_posts = posts;
        sorted_posts.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let mut entries = VecDeque::new();
        for post in sorted_posts.into_iter().take(MAX_TOPIC_CACHE) {
            let timestamp = post.created_at.timestamp();
            let post_id = post.id.clone();
            inner.posts_by_id.insert(post_id.clone(), post);
            entries.push_back((post_id, timestamp));
        }

        inner.topic_index.insert(topic_id.to_string(), entries);
    }

    pub async fn invalidate_topic(&self, topic_id: &str) {
        let mut inner = self.inner.write().await;
        if let Some(entries) = inner.topic_index.remove(topic_id) {
            for (post_id, _) in entries {
                if let Some(existing) = inner.posts_by_id.get(&post_id) {
                    if existing.topic_id == topic_id {
                        inner.posts_by_id.remove(&post_id);
                    }
                } else {
                    inner.posts_by_id.remove(&post_id);
                }
            }
        }
    }

    pub async fn remove(&self, id: &str) -> Option<Post> {
        let mut inner = self.inner.write().await;
        let removed = inner.posts_by_id.remove(id);
        if removed.is_some() {
            for entries in inner.topic_index.values_mut() {
                entries.retain(|(post_id, _)| post_id != id);
            }
        }
        removed
    }

    pub async fn clear(&self) {
        let mut inner = self.inner.write().await;
        inner.posts_by_id.clear();
        inner.topic_index.clear();
    }

    pub async fn size(&self) -> usize {
        let inner = self.inner.read().await;
        inner.posts_by_id.len()
    }

    async fn add_internal(inner: &Arc<RwLock<PostCacheInner>>, post: Post) {
        let mut guard = inner.write().await;
        let topic_id = post.topic_id.clone();
        let post_id = post.id.clone();
        let timestamp = post.created_at.timestamp();

        guard.posts_by_id.insert(post_id.clone(), post);

        let entries = guard.topic_index.entry(topic_id).or_default();
        entries.retain(|(id, _)| id != &post_id);
        let position = entries.iter().position(|(_, ts)| *ts < timestamp);
        match position {
            Some(idx) => entries.insert(idx, (post_id, timestamp)),
            None => entries.push_back((post_id, timestamp)),
        }
        if entries.len() > MAX_TOPIC_CACHE {
            entries.truncate(MAX_TOPIC_CACHE);
        }
    }
}

#[async_trait]
impl PostCache for PostCacheService {
    async fn add(&self, post: Post) {
        PostCacheService::add(self, post).await;
    }

    async fn get(&self, id: &str) -> Option<Post> {
        PostCacheService::get(self, id).await
    }

    async fn remove(&self, id: &str) -> Option<Post> {
        PostCacheService::remove(self, id).await
    }

    async fn get_by_topic(&self, topic_id: &str, limit: usize) -> Vec<Post> {
        PostCacheService::get_by_topic(self, topic_id, limit).await
    }

    async fn set_topic_posts(&self, topic_id: &str, posts: Vec<Post>) {
        PostCacheService::set_topic_posts(self, topic_id, posts).await;
    }

    async fn invalidate_topic(&self, topic_id: &str) {
        PostCacheService::invalidate_topic(self, topic_id).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn create_test_post(id: &str, topic_id: &str, ts: i64) -> Post {
        use crate::domain::entities::user::User;

        let author = User {
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
            public_profile: true,
            show_online_status: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        Post {
            id: id.to_string(),
            content: "Test content".to_string(),
            author,
            topic_id: topic_id.to_string(),
            created_at: Utc.timestamp_opt(ts, 0).unwrap(),
            tags: Vec::new(),
            likes: 0,
            boosts: 0,
            replies: Vec::new(),
            is_synced: true,
            is_boosted: false,
            is_bookmarked: false,
            local_id: None,
            event_id: None,
        }
    }

    #[tokio::test]
    async fn test_add_and_get() {
        let cache = PostCacheService::new();
        let post = create_test_post("1", "topic1", 1);

        cache.add(post.clone()).await;
        let retrieved = cache.get("1").await;

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "1");
    }

    #[tokio::test]
    async fn test_topic_ordering_and_limit() {
        let cache = PostCacheService::new();
        for i in 0..5 {
            let post = create_test_post(&format!("p{i}"), "topic", i);
            cache.add(post).await;
        }

        let posts = cache.get_by_topic("topic", 3).await;
        assert_eq!(posts.len(), 3);
        assert_eq!(posts[0].id, "p4");
        assert_eq!(posts[1].id, "p3");
        assert_eq!(posts[2].id, "p2");
    }

    #[tokio::test]
    async fn test_set_topic_posts_replaces_existing() {
        let cache = PostCacheService::new();

        let initial = vec![
            create_test_post("old1", "topic", 1),
            create_test_post("old2", "topic", 2),
        ];
        cache.set_topic_posts("topic", initial).await;

        let replacement = vec![
            create_test_post("new1", "topic", 10),
            create_test_post("new2", "topic", 11),
        ];
        cache.set_topic_posts("topic", replacement.clone()).await;

        let posts = cache.get_by_topic("topic", 10).await;
        assert_eq!(posts.len(), 2);
        assert_eq!(posts[0].id, "new2");
        assert_eq!(posts[1].id, "new1");

        assert!(cache.get("old1").await.is_none());
        assert!(cache.get("old2").await.is_none());
        assert!(cache.get("new1").await.is_some());
        assert!(cache.get("new2").await.is_some());
    }

    #[tokio::test]
    async fn test_invalidate_topic() {
        let cache = PostCacheService::new();
        let posts = vec![
            create_test_post("1", "topic1", 1),
            create_test_post("2", "topic1", 2),
        ];

        cache.set_topic_posts("topic1", posts).await;
        cache.invalidate_topic("topic1").await;

        assert!(cache.get_by_topic("topic1", 10).await.is_empty());
        assert!(cache.get("1").await.is_none());
        assert!(cache.get("2").await.is_none());
    }

    #[tokio::test]
    async fn test_remove_post() {
        let cache = PostCacheService::new();
        let post = create_test_post("1", "topic1", 1);

        cache.add(post.clone()).await;
        let removed = cache.remove("1").await;

        assert!(removed.is_some());
        assert_eq!(removed.unwrap().id, "1");
        assert!(cache.get_by_topic("topic1", 10).await.is_empty());
    }

    #[tokio::test]
    async fn test_clear_resets_cache() {
        let cache = PostCacheService::new();
        cache
            .add_many(vec![
                create_test_post("1", "topic1", 1),
                create_test_post("2", "topic2", 2),
            ])
            .await;

        assert_eq!(cache.size().await, 2);
        cache.clear().await;
        assert_eq!(cache.size().await, 0);
    }
}
