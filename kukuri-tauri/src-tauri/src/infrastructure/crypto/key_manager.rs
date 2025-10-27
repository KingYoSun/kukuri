use crate::application::ports::key_manager::{KeyManager, KeyMaterialStore, KeyPair};
use crate::shared::error::AppError;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use nostr_sdk::{FromBech32, prelude::*};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// デフォルトのKeyManager実装
#[derive(Clone)]
pub struct DefaultKeyManager {
    inner: Arc<RwLock<KeyManagerInner>>,
    key_store: Arc<dyn KeyMaterialStore>,
}

struct KeyManagerInner {
    keys: Option<Keys>,
}

impl DefaultKeyManager {
    pub fn new() -> Self {
        Self::with_store(Arc::new(InMemoryKeyMaterialStore::default()))
    }

    pub fn with_store(key_store: Arc<dyn KeyMaterialStore>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(KeyManagerInner { keys: None })),
            key_store,
        }
    }

    /// 旧インターフェース用: 新しいキーペアを生成（タプル形式）
    pub async fn generate(&self) -> Result<(String, String, String)> {
        let keys = Keys::generate();
        let public_key = keys.public_key().to_hex();
        let secret_key = keys.secret_key().to_bech32()?;
        let npub = keys.public_key().to_bech32()?;

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

    async fn save_generated_keys(&self, keys: &Keys, keypair: &KeyPair) -> Result<(), AppError> {
        {
            let mut inner = self.inner.write().await;
            inner.keys = Some(keys.clone());
        }
        self.key_store.save_keypair(keypair).await?;
        self.key_store.set_current(&keypair.npub).await
    }

    async fn install_current_from_pair(&self, keypair: &KeyPair) -> Result<(), AppError> {
        let keys = keys_from_keypair(keypair)?;
        let mut inner = self.inner.write().await;
        inner.keys = Some(keys);
        Ok(())
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
        let keypair = keypair_from_keys(&keys)?;
        self.save_generated_keys(&keys, &keypair).await?;
        Ok(keypair)
    }

    async fn import_private_key(&self, nsec: &str) -> Result<KeyPair, AppError> {
        let secret_key = SecretKey::from_bech32(nsec)
            .map_err(|e| AppError::Crypto(format!("Invalid nsec: {e:?}")))?;
        let keys = Keys::new(secret_key);
        let keypair = keypair_from_keys(&keys)?;
        self.save_generated_keys(&keys, &keypair).await?;
        Ok(keypair)
    }

    async fn export_private_key(&self, npub: &str) -> Result<String, AppError> {
        self.key_store
            .get_keypair(npub)
            .await?
            .map(|kp| kp.nsec)
            .ok_or_else(|| AppError::NotFound(format!("Key not found: {npub}")))
    }

    async fn get_public_key(&self, npub: &str) -> Result<String, AppError> {
        self.key_store
            .get_keypair(npub)
            .await?
            .map(|kp| kp.public_key)
            .ok_or_else(|| AppError::NotFound(format!("Key not found: {npub}")))
    }

    async fn store_keypair(&self, keypair: &KeyPair) -> Result<(), AppError> {
        self.key_store.save_keypair(keypair).await?;
        self.key_store.set_current(&keypair.npub).await?;
        self.install_current_from_pair(keypair).await
    }

    async fn delete_keypair(&self, npub: &str) -> Result<(), AppError> {
        self.key_store.delete_keypair(npub).await?;
        let mut inner = self.inner.write().await;
        if let Some(keys) = &inner.keys {
            let current_npub = keys
                .public_key()
                .to_bech32()
                .map_err(|e| AppError::Crypto(format!("Failed to convert npub: {e:?}")))?;
            if current_npub == npub {
                inner.keys = None;
            }
        }
        Ok(())
    }

    async fn list_npubs(&self) -> Result<Vec<String>, AppError> {
        let pairs = self.key_store.list_keypairs().await?;
        Ok(pairs.into_iter().map(|kp| kp.npub).collect())
    }

    async fn current_keypair(&self) -> Result<KeyPair, AppError> {
        if let Some(keys) = self.inner.read().await.keys.clone() {
            let npub = keys
                .public_key()
                .to_bech32()
                .map_err(|e| AppError::Crypto(format!("Failed to convert npub: {e:?}")))?;
            if let Some(pair) = self.key_store.get_keypair(&npub).await? {
                return Ok(pair);
            }
        }

        if let Some(pair) = self.key_store.current_keypair().await? {
            self.install_current_from_pair(&pair).await?;
            Ok(pair)
        } else {
            Err(AppError::NotFound("No keys loaded".into()))
        }
    }
}

fn keypair_from_keys(keys: &Keys) -> Result<KeyPair, AppError> {
    let public_key = keys.public_key().to_hex();
    let private_key = keys.secret_key().display_secret().to_string();
    let npub = keys
        .public_key()
        .to_bech32()
        .map_err(|e| AppError::Crypto(format!("Failed to convert to bech32: {e:?}")))?;
    let nsec = keys
        .secret_key()
        .to_bech32()
        .map_err(|e| AppError::Crypto(format!("Failed to convert to bech32: {e:?}")))?;

    Ok(KeyPair {
        public_key,
        private_key,
        npub,
        nsec,
    })
}

fn keys_from_keypair(keypair: &KeyPair) -> Result<Keys, AppError> {
    let secret_key = SecretKey::from_bech32(&keypair.nsec)
        .map_err(|e| AppError::Crypto(format!("Invalid nsec: {e:?}")))?;
    Ok(Keys::new(secret_key))
}

#[derive(Default)]
struct InMemoryKeyMaterialStore {
    keys: RwLock<HashMap<String, KeyPair>>,
    current: RwLock<Option<String>>,
}

#[async_trait]
impl KeyMaterialStore for InMemoryKeyMaterialStore {
    async fn save_keypair(&self, keypair: &KeyPair) -> Result<(), AppError> {
        let mut guard = self.keys.write().await;
        guard.insert(keypair.npub.clone(), keypair.clone());
        Ok(())
    }

    async fn delete_keypair(&self, npub: &str) -> Result<(), AppError> {
        let mut guard = self.keys.write().await;
        guard.remove(npub);
        let mut current = self.current.write().await;
        if current.as_deref() == Some(npub) {
            *current = None;
        }
        Ok(())
    }

    async fn get_keypair(&self, npub: &str) -> Result<Option<KeyPair>, AppError> {
        let guard = self.keys.read().await;
        Ok(guard.get(npub).cloned())
    }

    async fn list_keypairs(&self) -> Result<Vec<KeyPair>, AppError> {
        let guard = self.keys.read().await;
        Ok(guard.values().cloned().collect())
    }

    async fn set_current(&self, npub: &str) -> Result<(), AppError> {
        let guard = self.keys.read().await;
        if guard.contains_key(npub) {
            let mut current = self.current.write().await;
            *current = Some(npub.to_string());
            Ok(())
        } else {
            Err(AppError::NotFound(format!(
                "Keypair not found for npub {npub}"
            )))
        }
    }

    async fn current_keypair(&self) -> Result<Option<KeyPair>, AppError> {
        let guard = self.keys.read().await;
        let current = self.current.read().await;
        Ok(match current.as_deref() {
            Some(npub) => guard.get(npub).cloned(),
            None => None,
        })
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
        let pair = result.unwrap();
        assert!(!pair.public_key.is_empty());
        assert!(!pair.npub.is_empty());
        assert!(!pair.nsec.is_empty());
    }

    #[tokio::test]
    async fn test_import_private_key_roundtrip() {
        let key_manager = DefaultKeyManager::new();
        let (_, nsec, _) = key_manager.generate().await.expect("generate");
        let pair = key_manager
            .import_private_key(&nsec)
            .await
            .expect("import should work");
        assert_eq!(pair.nsec, nsec);
    }

    #[tokio::test]
    async fn test_store_and_export_keypair() {
        let key_manager = DefaultKeyManager::new();
        let generated = key_manager.generate_keypair().await.expect("generate");
        key_manager.store_keypair(&generated).await.expect("store");
        let exported = key_manager
            .export_private_key(&generated.npub)
            .await
            .expect("export");
        assert_eq!(exported, generated.nsec);
    }

    #[tokio::test]
    async fn test_delete_keypair_clears_current() {
        let key_manager = DefaultKeyManager::new();
        let pair = key_manager.generate_keypair().await.expect("generate");
        key_manager
            .store_keypair(&pair)
            .await
            .expect("store default");
        key_manager
            .delete_keypair(&pair.npub)
            .await
            .expect("delete");
        let result = key_manager.current_keypair().await;
        assert!(result.is_err());
    }
}
