use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub data: T,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

#[async_trait]
pub trait CacheStorage: Send + Sync {
    async fn set<T: Serialize + Send + Sync>(
        &self,
        key: &str,
        value: T,
        ttl: Option<Duration>,
    ) -> Result<(), Box<dyn std::error::Error>>;
    async fn get<T: for<'de> Deserialize<'de>>(
        &self,
        key: &str,
    ) -> Result<Option<T>, Box<dyn std::error::Error>>;
    async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error>>;
    async fn clear(&self) -> Result<(), Box<dyn std::error::Error>>;
    async fn exists(&self, key: &str) -> Result<bool, Box<dyn std::error::Error>>;
    async fn cleanup_expired(&self) -> Result<u32, Box<dyn std::error::Error>>;
}
