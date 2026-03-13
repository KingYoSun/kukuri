use crate::shared::error::AppError;
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
    async fn generate_keypair(&self) -> Result<KeyPair, AppError>;
    async fn import_private_key(&self, nsec: &str) -> Result<KeyPair, AppError>;
    async fn export_private_key(&self, npub: &str) -> Result<String, AppError>;
    async fn get_public_key(&self, npub: &str) -> Result<String, AppError>;
    async fn store_keypair(&self, keypair: &KeyPair) -> Result<(), AppError>;
    async fn delete_keypair(&self, npub: &str) -> Result<(), AppError>;
    async fn list_npubs(&self) -> Result<Vec<String>, AppError>;
    async fn current_keypair(&self) -> Result<KeyPair, AppError>;
}

#[async_trait]
pub trait KeyMaterialStore: Send + Sync {
    async fn save_keypair(&self, keypair: &KeyPair) -> Result<(), AppError>;
    async fn delete_keypair(&self, npub: &str) -> Result<(), AppError>;
    async fn get_keypair(&self, npub: &str) -> Result<Option<KeyPair>, AppError>;
    async fn list_keypairs(&self) -> Result<Vec<KeyPair>, AppError>;
    async fn set_current(&self, npub: &str) -> Result<(), AppError>;
    async fn current_keypair(&self) -> Result<Option<KeyPair>, AppError>;
}
