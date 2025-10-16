use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Clone)]
struct CacheEntry<T> {
    data: T,
    expires_at: Instant,
}

/// メモリキャッシュサービス
pub struct MemoryCacheService<T: Clone> {
    cache: Arc<RwLock<HashMap<String, CacheEntry<T>>>>,
    default_ttl: Duration,
}

impl<T> MemoryCacheService<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// 新しいキャッシュサービスを作成
    pub fn new(default_ttl_seconds: u64) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            default_ttl: Duration::from_secs(default_ttl_seconds),
        }
    }

    /// キャッシュにデータを保存
    pub async fn set(&self, key: String, value: T) {
        self.set_with_ttl(key, value, self.default_ttl).await;
    }

    /// 指定したTTLでキャッシュに保存
    pub async fn set_with_ttl(&self, key: String, value: T, ttl: Duration) {
        let entry = CacheEntry {
            data: value,
            expires_at: Instant::now() + ttl,
        };

        let mut cache = self.cache.write().await;
        cache.insert(key, entry);
    }

    /// キャッシュからデータを取得
    pub async fn get(&self, key: &str) -> Option<T> {
        let cache = self.cache.read().await;

        if let Some(entry) = cache.get(key) {
            if entry.expires_at > Instant::now() {
                return Some(entry.data.clone());
            }
        }

        None
    }

    /// 複数のキーからデータを取得
    pub async fn get_many(&self, keys: &[String]) -> HashMap<String, T> {
        let cache = self.cache.read().await;
        let now = Instant::now();

        let mut results = HashMap::new();
        for key in keys {
            if let Some(entry) = cache.get(key) {
                if entry.expires_at > now {
                    results.insert(key.clone(), entry.data.clone());
                }
            }
        }

        results
    }

    /// キャッシュから削除
    pub async fn delete(&self, key: &str) {
        let mut cache = self.cache.write().await;
        cache.remove(key);
    }

    /// パターンにマッチするキーを削除
    pub async fn delete_pattern(&self, pattern: &str) {
        let mut cache = self.cache.write().await;
        let keys_to_remove: Vec<String> = cache
            .keys()
            .filter(|k| k.contains(pattern))
            .cloned()
            .collect();

        for key in keys_to_remove {
            cache.remove(&key);
        }
    }

    /// キャッシュをクリア
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// 期限切れのエントリを削除
    pub async fn cleanup_expired(&self) {
        let mut cache = self.cache.write().await;
        let now = Instant::now();

        cache.retain(|_, entry| entry.expires_at > now);
    }

    /// キャッシュサイズを取得
    pub async fn size(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }
}

// 投稿用の特殊化されたキャッシュサービス
pub struct PostCacheService {
    cache: MemoryCacheService<crate::domain::entities::post::Post>,
}

impl PostCacheService {
    pub fn new() -> Self {
        Self {
            // 投稿は5分間キャッシュ
            cache: MemoryCacheService::new(300),
        }
    }

    pub async fn cache_post(&self, post: crate::domain::entities::post::Post) {
        let key = format!("post:{}", post.id);
        self.cache.set(key, post).await;
    }

    pub async fn get_post(&self, post_id: &str) -> Option<crate::domain::entities::post::Post> {
        let key = format!("post:{post_id}");
        self.cache.get(&key).await
    }

    pub async fn cache_posts_by_topic(
        &self,
        topic_id: &str,
        posts: Vec<crate::domain::entities::post::Post>,
    ) {
        let key = format!("topic_posts:{topic_id}");
        // トピック別の投稿は短めのTTL（1分）
        for post in posts {
            self.cache_post(post).await;
        }
    }

    pub async fn invalidate_topic_posts(&self, topic_id: &str) {
        let pattern = format!("topic_posts:{topic_id}");
        self.cache.delete_pattern(&pattern).await;
    }

    pub async fn invalidate_post(&self, post_id: &str) {
        let key = format!("post:{post_id}");
        self.cache.delete(&key).await;
    }
}

// ユーザー用のキャッシュサービス
pub struct UserCacheService {
    cache: MemoryCacheService<crate::domain::entities::user::User>,
}

impl UserCacheService {
    pub fn new() -> Self {
        Self {
            // ユーザー情報は10分間キャッシュ
            cache: MemoryCacheService::new(600),
        }
    }

    pub async fn cache_user(&self, user: crate::domain::entities::user::User) {
        let key = format!("user:{}", user.pubkey);
        self.cache.set(key, user).await;
    }

    pub async fn get_user(&self, pubkey: &str) -> Option<crate::domain::entities::user::User> {
        let key = format!("user:{pubkey}");
        self.cache.get(&key).await
    }

    pub async fn invalidate_user(&self, pubkey: &str) {
        let key = format!("user:{pubkey}");
        self.cache.delete(&key).await;
    }
}

// トピック用のキャッシュサービス
pub struct TopicCacheService {
    cache: MemoryCacheService<crate::domain::entities::topic::Topic>,
}

impl TopicCacheService {
    pub fn new() -> Self {
        Self {
            // トピック情報は30分間キャッシュ
            cache: MemoryCacheService::new(1800),
        }
    }

    pub async fn cache_topic(&self, topic: crate::domain::entities::topic::Topic) {
        let key = format!("topic:{}", topic.id);
        self.cache.set(key, topic).await;
    }

    pub async fn get_topic(&self, topic_id: &str) -> Option<crate::domain::entities::topic::Topic> {
        let key = format!("topic:{topic_id}");
        self.cache.get(&key).await
    }

    pub async fn cache_all_topics(&self, topics: Vec<crate::domain::entities::topic::Topic>) {
        for topic in topics {
            self.cache_topic(topic).await;
        }
    }

    pub async fn invalidate_topic(&self, topic_id: &str) {
        let key = format!("topic:{topic_id}");
        self.cache.delete(&key).await;
    }
}
