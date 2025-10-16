use anyhow::{Result, anyhow};
use nostr_sdk::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;

#[allow(dead_code)]
#[derive(Clone)]
pub struct KeyManager {
    inner: Arc<RwLock<KeyManagerInner>>,
}

#[allow(dead_code)]
struct KeyManagerInner {
    keys: Option<Keys>,
}

#[allow(dead_code)]
impl KeyManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(KeyManagerInner { keys: None })),
        }
    }

    pub async fn generate_keypair(&self) -> Result<(String, String, String)> {
        let keys = Keys::generate();
        let public_key = keys.public_key().to_hex();
        let secret_key = keys.secret_key().to_bech32()?;
        let npub = keys.public_key().to_bech32()?;

        // Save generated keys
        let mut inner = self.inner.write().await;
        inner.keys = Some(keys);

        Ok((public_key, secret_key, npub))
    }

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

    pub async fn logout(&self) -> Result<()> {
        let mut inner = self.inner.write().await;
        inner.keys = None;
        Ok(())
    }

    pub async fn get_keys(&self) -> Result<Keys> {
        let inner = self.inner.read().await;
        inner.keys.clone().ok_or_else(|| anyhow!("No keys loaded"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_key_manager_new() {
        let key_manager = KeyManager::new();
        let result = key_manager.get_keys().await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "No keys loaded");
    }

    #[tokio::test]
    async fn test_generate_keypair() {
        let key_manager = KeyManager::new();
        let result = key_manager.generate_keypair().await;
        assert!(result.is_ok());

        let (public_key, secret_key, npub) = result.unwrap();
        assert_eq!(public_key.len(), 64); // Hex public key is 64 characters
        assert!(secret_key.starts_with("nsec1")); // Bech32 secret key starts with nsec1
        assert!(npub.starts_with("npub1")); // Bech32 public key starts with npub1

        // Verify keys are stored
        let stored_keys = key_manager.get_keys().await;
        assert!(stored_keys.is_ok());
        assert_eq!(stored_keys.unwrap().public_key().to_hex(), public_key);
    }

    #[tokio::test]
    async fn test_login_with_valid_nsec() {
        let key_manager = KeyManager::new();

        // Generate a test key first
        let test_keys = Keys::generate();
        let test_nsec = test_keys.secret_key().to_bech32().unwrap();
        let expected_public_key = test_keys.public_key().to_hex();

        // Login with the test key
        let result = key_manager.login(&test_nsec).await;
        assert!(result.is_ok());

        let (public_key, npub) = result.unwrap();
        assert_eq!(public_key, expected_public_key);
        assert!(npub.starts_with("npub1"));

        // Verify keys are stored
        let stored_keys = key_manager.get_keys().await;
        assert!(stored_keys.is_ok());
        assert_eq!(stored_keys.unwrap().public_key().to_hex(), public_key);
    }

    #[tokio::test]
    async fn test_login_with_invalid_nsec() {
        let key_manager = KeyManager::new();
        let result = key_manager.login("invalid_nsec").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_logout() {
        let key_manager = KeyManager::new();

        // Generate and store keys first
        let _ = key_manager.generate_keypair().await.unwrap();
        assert!(key_manager.get_keys().await.is_ok());

        // Logout
        let result = key_manager.logout().await;
        assert!(result.is_ok());

        // Verify keys are cleared
        let stored_keys = key_manager.get_keys().await;
        assert!(stored_keys.is_err());
        assert_eq!(stored_keys.unwrap_err().to_string(), "No keys loaded");
    }
}
