use async_trait::async_trait;

#[async_trait]
pub trait SecureStorage: Send + Sync {
    async fn store(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>>;
    async fn retrieve(&self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error>>;
    async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error>>;
    async fn exists(&self, key: &str) -> Result<bool, Box<dyn std::error::Error>>;
    async fn list_keys(&self) -> Result<Vec<String>, Box<dyn std::error::Error>>;
    async fn clear(&self) -> Result<(), Box<dyn std::error::Error>>;
}