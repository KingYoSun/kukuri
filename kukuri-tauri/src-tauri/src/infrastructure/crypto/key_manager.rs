use crate::shared::error::AppError;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use nostr_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPair {
    pub public_key: String,
    pub private_key: String,
    pub npub: String,
    pub nsec: String,
}

/// 鍵管理のトレイト
#[async_trait]
pub trait KeyManager: Send + Sync {
    async fn generate_keypair(&self) -> Result<KeyPair, AppError>;
    async fn import_private_key(&self, nsec: &str) -> Result<KeyPair, AppError>;
    async fn export_private_key(&self, npub: &str) -> Result<String, AppError>;
    async fn get_public_key(&self, npub: &str) -> Result<String, AppError>;
    async fn store_keypair(&self, keypair: &KeyPair) -> Result<(), AppError>;
    async fn delete_keypair(&self, npub: &str) -> Result<(), AppError>;
    async fn list_npubs(&self) -> Result<Vec<String>, AppError>;
}

/// デフォルトのKeyManager実装
#[derive(Clone)]
pub struct DefaultKeyManager {
    inner: Arc<RwLock<KeyManagerInner>>,
}

struct KeyManagerInner {
    keys: Option<Keys>,
    stored_keys: std::collections::HashMap<String, KeyPair>,
}

impl DefaultKeyManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(KeyManagerInner {
                keys: None,
                stored_keys: std::collections::HashMap::new(),
            })),
        }
    }

    /// 旧インターフェース用: 新しいキーペアを生成（タプル形式）
    pub async fn generate(&self) -> Result<(String, String, String)> {
        let keys = Keys::generate();
        let public_key = keys.public_key().to_hex();
        let secret_key = keys.secret_key().to_bech32()?;
        let npub = keys.public_key().to_bech32()?;

        // Save generated keys
        let mut inner = self.inner.write().await;
        inner.keys = Some(keys);

        Ok((public_key, secret_key, npub))
    }

    /// 旧インターフェース用: nsecでログイン
    pub async fn login(&self, nsec: &str) -> Result<(String, String)> {
        let secret_key = SecretKey::from_bech32(nsec)?;
        let keys = Keys::new(secret_key);

        let public_key = keys.public_key().to_hex();
        let npub = keys.public_key().to_bech32()?;

        // Save keys
        let mut inner = self.inner.write().await;
        inner.keys = Some(keys);

        Ok((public_key, npub))
    }

    /// 旧インターフェース用: ログアウト
    pub async fn logout(&self) -> Result<()> {
        let mut inner = self.inner.write().await;
        inner.keys = None;
        Ok(())
    }

    /// 旧インターフェース用: 現在の鍵を取得
    pub async fn get_keys(&self) -> Result<Keys> {
        let inner = self.inner.read().await;
        inner.keys.clone().ok_or_else(|| anyhow!("No keys loaded"))
    }
}

impl Default for DefaultKeyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl KeyManager for DefaultKeyManager {
    async fn generate_keypair(&self) -> Result<KeyPair, AppError> {
        let keys = Keys::generate();
        let public_key = keys.public_key().to_hex();
        let private_key = keys.secret_key().display_secret().to_string();
        let npub = keys
            .public_key()
            .to_bech32()
            .map_err(|e| AppError::Crypto(format!("Failed to convert to bech32: {:?}", e)))?;
        let nsec = keys
            .secret_key()
            .to_bech32()
            .map_err(|e| AppError::Crypto(format!("Failed to convert to bech32: {:?}", e)))?;

        let keypair = KeyPair {
            public_key,
            private_key,
            npub: npub.clone(),
            nsec,
        };

        // Store in memory
        let mut inner = self.inner.write().await;
        inner.keys = Some(keys);
        inner.stored_keys.insert(npub, keypair.clone());

        Ok(keypair)
    }

    async fn import_private_key(&self, nsec: &str) -> Result<KeyPair, AppError> {
        let secret_key = SecretKey::from_bech32(nsec)
            .map_err(|e| AppError::Crypto(format!("Invalid nsec: {:?}", e)))?;
        let keys = Keys::new(secret_key);

        let public_key = keys.public_key().to_hex();
        let private_key = keys.secret_key().display_secret().to_string();
        let npub = keys
            .public_key()
            .to_bech32()
            .map_err(|e| AppError::Crypto(format!("Failed to convert to bech32: {:?}", e)))?;

        let keypair = KeyPair {
            public_key,
            private_key,
            npub: npub.clone(),
            nsec: nsec.to_string(),
        };

        // Store in memory
        let mut inner = self.inner.write().await;
        inner.keys = Some(keys);
        inner.stored_keys.insert(npub, keypair.clone());

        Ok(keypair)
    }

    async fn export_private_key(&self, npub: &str) -> Result<String, AppError> {
        let inner = self.inner.read().await;
        inner
            .stored_keys
            .get(npub)
            .map(|kp| kp.nsec.clone())
            .ok_or_else(|| AppError::NotFound(format!("Key not found: {}", npub)))
    }

    async fn get_public_key(&self, npub: &str) -> Result<String, AppError> {
        let inner = self.inner.read().await;
        inner
            .stored_keys
            .get(npub)
            .map(|kp| kp.public_key.clone())
            .ok_or_else(|| AppError::NotFound(format!("Key not found: {}", npub)))
    }

    async fn store_keypair(&self, keypair: &KeyPair) -> Result<(), AppError> {
        let mut inner = self.inner.write().await;
        inner
            .stored_keys
            .insert(keypair.npub.clone(), keypair.clone());
        Ok(())
    }

    async fn delete_keypair(&self, npub: &str) -> Result<(), AppError> {
        let mut inner = self.inner.write().await;
        inner.stored_keys.remove(npub);
        // If this was the current key, clear it
        if let Some(keys) = &inner.keys {
            if keys
                .public_key()
                .to_bech32()
                .map_err(|e| AppError::Crypto(format!("Failed to convert to bech32: {:?}", e)))?
                == npub
            {
                inner.keys = None;
            }
        }
        Ok(())
    }

    async fn list_npubs(&self) -> Result<Vec<String>, AppError> {
        let inner = self.inner.read().await;
        Ok(inner.stored_keys.keys().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_key_manager_new() {
        let key_manager = DefaultKeyManager::new();
        let result = key_manager.get_keys().await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "No keys loaded");
    }

    #[tokio::test]
    async fn test_generate_keypair() {
        let key_manager = DefaultKeyManager::new();
        let result = key_manager.generate_keypair().await;
        assert!(result.is_ok());

        let keypair = result.unwrap();
        assert_eq!(keypair.public_key.len(), 64); // Hex public key is 64 characters
        assert!(keypair.nsec.starts_with("nsec1")); // Bech32 secret key starts with nsec1
        assert!(keypair.npub.starts_with("npub1")); // Bech32 public key starts with npub1

        // Verify keys are stored
        let stored_keys = key_manager.get_keys().await;
        assert!(stored_keys.is_ok());
        assert_eq!(
            stored_keys.unwrap().public_key().to_hex(),
            keypair.public_key
        );
    }

    #[tokio::test]
    async fn test_import_private_key() {
        let key_manager = DefaultKeyManager::new();

        // Generate a test key first
        let test_keys = Keys::generate();
        let test_nsec = test_keys.secret_key().to_bech32().unwrap();
        let expected_public_key = test_keys.public_key().to_hex();

        // Import the test key
        let result = key_manager.import_private_key(&test_nsec).await;
        assert!(result.is_ok());

        let keypair = result.unwrap();
        assert_eq!(keypair.public_key, expected_public_key);
        assert!(keypair.npub.starts_with("npub1"));

        // Verify keys are stored
        let stored_keys = key_manager.get_keys().await;
        assert!(stored_keys.is_ok());
        assert_eq!(
            stored_keys.unwrap().public_key().to_hex(),
            keypair.public_key
        );
    }

    #[tokio::test]
    async fn test_login_with_invalid_nsec() {
        let key_manager = DefaultKeyManager::new();
        let result = key_manager.login("invalid_nsec").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_logout() {
        let key_manager = DefaultKeyManager::new();

        // Generate and store keys first
        let _ = key_manager.generate().await.unwrap();
        assert!(key_manager.get_keys().await.is_ok());

        // Logout
        let result = key_manager.logout().await;
        assert!(result.is_ok());

        // Verify keys are cleared
        let stored_keys = key_manager.get_keys().await;
        assert!(stored_keys.is_err());
        assert_eq!(stored_keys.unwrap_err().to_string(), "No keys loaded");
    }

    #[tokio::test]
    async fn test_list_npubs() {
        let key_manager = DefaultKeyManager::new();

        // Initially empty
        let npubs = key_manager.list_npubs().await.unwrap();
        assert_eq!(npubs.len(), 0);

        // Generate a keypair
        let keypair1 = key_manager.generate_keypair().await.unwrap();
        let npubs = key_manager.list_npubs().await.unwrap();
        assert_eq!(npubs.len(), 1);
        assert!(npubs.contains(&keypair1.npub));

        // Generate another keypair
        let keypair2 = key_manager.generate_keypair().await.unwrap();
        let npubs = key_manager.list_npubs().await.unwrap();
        assert_eq!(npubs.len(), 2);
        assert!(npubs.contains(&keypair1.npub));
        assert!(npubs.contains(&keypair2.npub));
    }

    #[tokio::test]
    async fn test_export_private_key() {
        let key_manager = DefaultKeyManager::new();

        // Generate a keypair
        let keypair = key_manager.generate_keypair().await.unwrap();

        // Export the private key
        let nsec = key_manager.export_private_key(&keypair.npub).await.unwrap();
        assert_eq!(nsec, keypair.nsec);

        // Try to export non-existent key
        let result = key_manager.export_private_key("npub1nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_keypair() {
        let key_manager = DefaultKeyManager::new();

        // Generate a keypair
        let keypair = key_manager.generate_keypair().await.unwrap();
        assert_eq!(key_manager.list_npubs().await.unwrap().len(), 1);

        // Delete the keypair
        key_manager.delete_keypair(&keypair.npub).await.unwrap();
        assert_eq!(key_manager.list_npubs().await.unwrap().len(), 0);

        // Verify current keys are also cleared
        let result = key_manager.get_keys().await;
        assert!(result.is_err());
    }
}
