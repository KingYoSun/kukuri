use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPair {
    pub public_key: String,
    pub private_key: String,
    pub npub: String,
    pub nsec: String,
}

#[async_trait]
pub trait KeyManager: Send + Sync {
    async fn generate_keypair(&self) -> Result<KeyPair, Box<dyn std::error::Error>>;
    async fn import_private_key(&self, nsec: &str) -> Result<KeyPair, Box<dyn std::error::Error>>;
    async fn export_private_key(&self, npub: &str) -> Result<String, Box<dyn std::error::Error>>;
    async fn get_public_key(&self, npub: &str) -> Result<String, Box<dyn std::error::Error>>;
    async fn store_keypair(&self, keypair: &KeyPair) -> Result<(), Box<dyn std::error::Error>>;
    async fn delete_keypair(&self, npub: &str) -> Result<(), Box<dyn std::error::Error>>;
    async fn list_npubs(&self) -> Result<Vec<String>, Box<dyn std::error::Error>>;
}