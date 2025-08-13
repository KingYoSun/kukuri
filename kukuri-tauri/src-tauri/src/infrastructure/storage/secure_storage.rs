use anyhow::{Context, Result};
use async_trait::async_trait;
use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error};

const SERVICE_NAME: &str = "kukuri";
const ACCOUNTS_KEY: &str = "accounts_metadata";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountMetadata {
    pub npub: String,
    pub pubkey: String,
    pub name: String,
    pub display_name: String,
    pub picture: Option<String>,
    pub last_used: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AccountsMetadata {
    pub accounts: HashMap<String, AccountMetadata>,
    pub current_npub: Option<String>,
}

/// セキュアストレージのトレイト
#[async_trait]
pub trait SecureStorage: Send + Sync {
    async fn store(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn retrieve(&self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn exists(&self, key: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_keys(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>>;
    async fn clear(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// デフォルトのSecureStorage実装
pub struct DefaultSecureStorage;

impl DefaultSecureStorage {
    pub fn new() -> Self {
        Self
    }

    /// 秘密鍵を保存（npubごとに個別保存）
    pub fn save_private_key(npub: &str, nsec: &str) -> Result<()> {
        debug!("SecureStorage: Saving private key for npub={npub}");
        
        let entry = Entry::new(SERVICE_NAME, npub).context("Failed to create keyring entry")?;
        
        match entry.set_password(nsec) {
            Ok(_) => {
                debug!("SecureStorage: Private key saved successfully for npub={npub}");
                Ok(())
            }
            Err(e) => {
                error!("SecureStorage: Failed to save private key: {e:?}");
                Err(anyhow::anyhow!("Failed to save private key to keyring: {e}"))
            }
        }
    }

    /// 秘密鍵を取得
    pub fn get_private_key(npub: &str) -> Result<Option<String>> {
        let entry = Entry::new(SERVICE_NAME, npub).context("Failed to create keyring entry")?;
        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("Failed to get private key: {e}")),
        }
    }

    /// 秘密鍵を削除
    pub fn delete_private_key(npub: &str) -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, npub).context("Failed to create keyring entry")?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()), // 既に削除されている場合もOK
            Err(e) => Err(anyhow::anyhow!("Failed to delete private key: {e}")),
        }
    }

    /// アカウントメタデータを保存（公開情報のみ）
    pub fn save_accounts_metadata(metadata: &AccountsMetadata) -> Result<()> {
        let json =
            serde_json::to_string(metadata).context("Failed to serialize accounts metadata")?;
        debug!("SecureStorage: Saving metadata JSON: {json}");
        
        let entry = Entry::new(SERVICE_NAME, ACCOUNTS_KEY).context("Failed to create keyring entry")?;
        
        match entry.set_password(&json) {
            Ok(_) => {
                debug!("SecureStorage: Metadata saved to keyring successfully");
                
                // デバッグ: 保存直後に読み取りテスト
                debug!("SecureStorage: Testing immediate read after save...");
                let test_entry = Entry::new(SERVICE_NAME, ACCOUNTS_KEY)
                    .context("Failed to create test entry")?;
                
                match test_entry.get_password() {
                    Ok(test_json) => {
                        debug!("SecureStorage: Immediate read test succeeded, data length: {}", test_json.len());
                    }
                    Err(e) => {
                        error!("SecureStorage: Immediate read test failed: {e:?}");
                    }
                }
                
                Ok(())
            }
            Err(e) => {
                error!("SecureStorage: Failed to save metadata to keyring: {e:?}");
                Err(anyhow::anyhow!("Failed to save accounts metadata: {e}"))
            }
        }
    }

    /// アカウントメタデータを取得
    pub fn get_accounts_metadata() -> Result<AccountsMetadata> {
        debug!("SecureStorage: Getting accounts metadata from keyring...");
        
        let entry = Entry::new(SERVICE_NAME, ACCOUNTS_KEY).context("Failed to create keyring entry")?;
        
        match entry.get_password() {
            Ok(json) => {
                debug!("SecureStorage: Retrieved metadata JSON: {json}");
                let metadata: AccountsMetadata = serde_json::from_str(&json)
                    .context("Failed to deserialize accounts metadata")?;
                debug!("SecureStorage: Deserialized metadata - current_npub: {:?}, accounts: {}", 
                    metadata.current_npub, metadata.accounts.len());
                Ok(metadata)
            }
            Err(keyring::Error::NoEntry) => {
                debug!("SecureStorage: No metadata entry found in keyring, returning default");
                Ok(AccountsMetadata::default())
            }
            Err(e) => {
                error!("SecureStorage: Failed to get metadata from keyring: {e:?}");
                Err(anyhow::anyhow!("Failed to get accounts metadata: {e}"))
            }
        }
    }

    /// アカウントを追加
    pub fn add_account(
        npub: &str,
        nsec: &str,
        pubkey: &str,
        name: &str,
        display_name: &str,
        picture: Option<String>,
    ) -> Result<()> {
        debug!("SecureStorage: Adding account npub={npub}");
        
        // 秘密鍵を保存
        Self::save_private_key(npub, nsec)?;
        debug!("SecureStorage: Private key saved");

        // メタデータを更新
        let mut metadata = Self::get_accounts_metadata()?;
        metadata.accounts.insert(
            npub.to_string(),
            AccountMetadata {
                npub: npub.to_string(),
                pubkey: pubkey.to_string(),
                name: name.to_string(),
                display_name: display_name.to_string(),
                picture,
                last_used: chrono::Utc::now(),
            },
        );
        metadata.current_npub = Some(npub.to_string());
        Self::save_accounts_metadata(&metadata)?;
        debug!("SecureStorage: Metadata saved with current_npub={npub}");

        Ok(())
    }

    /// アカウントを削除
    pub fn remove_account(npub: &str) -> Result<()> {
        // 秘密鍵を削除
        Self::delete_private_key(npub)?;

        // メタデータから削除
        let mut metadata = Self::get_accounts_metadata()?;
        metadata.accounts.remove(npub);
        if metadata.current_npub.as_ref() == Some(&npub.to_string()) {
            metadata.current_npub = metadata.accounts.keys().next().cloned();
        }
        Self::save_accounts_metadata(&metadata)?;

        Ok(())
    }

    /// 現在のアカウントを切り替え
    pub fn switch_account(npub: &str) -> Result<()> {
        let mut metadata = Self::get_accounts_metadata()?;
        if metadata.accounts.contains_key(npub) {
            metadata.current_npub = Some(npub.to_string());
            // 最終使用日時を更新
            if let Some(account) = metadata.accounts.get_mut(npub) {
                account.last_used = chrono::Utc::now();
            }
            Self::save_accounts_metadata(&metadata)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Account not found: {npub}"))
        }
    }

    /// 全アカウントを取得（秘密鍵なし）
    pub fn list_accounts() -> Result<Vec<AccountMetadata>> {
        let metadata = Self::get_accounts_metadata()?;
        let mut accounts: Vec<AccountMetadata> = metadata.accounts.values().cloned().collect();
        // 最終使用日時で降順ソート
        accounts.sort_by(|a, b| b.last_used.cmp(&a.last_used));
        Ok(accounts)
    }

    /// 現在のアカウントの秘密鍵を取得
    pub fn get_current_private_key() -> Result<Option<(String, String)>> {
        let metadata = Self::get_accounts_metadata()?;
        debug!("SecureStorage: current_npub = {:?}", metadata.current_npub);
        debug!("SecureStorage: accounts = {:?}", metadata.accounts.keys().collect::<Vec<_>>());
        
        if let Some(npub) = metadata.current_npub {
            if let Some(nsec) = Self::get_private_key(&npub)? {
                debug!("SecureStorage: Found private key for npub={npub}");
                Ok(Some((npub, nsec)))
            } else {
                debug!("SecureStorage: No private key found for npub={npub}");
                Ok(None)
            }
        } else {
            debug!("SecureStorage: No current_npub set");
            Ok(None)
        }
    }
}

impl Default for DefaultSecureStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SecureStorage for DefaultSecureStorage {
    async fn store(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let entry = Entry::new(SERVICE_NAME, key)
            .map_err(|e| e.to_string())?;
        entry.set_password(value)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn retrieve(&self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        let entry = Entry::new(SERVICE_NAME, key)
            .map_err(|e| e.to_string())?;
        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
        }
    }

    async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let entry = Entry::new(SERVICE_NAME, key)
            .map_err(|e| e.to_string())?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()), // Already deleted is OK
            Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
        }
    }

    async fn exists(&self, key: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let entry = Entry::new(SERVICE_NAME, key)
            .map_err(|e| e.to_string())?;
        match entry.get_password() {
            Ok(_) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
        }
    }

    async fn list_keys(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        // keyringライブラリは直接的なキーのリストをサポートしていないため、
        // アカウントメタデータから取得
        let metadata = Self::get_accounts_metadata()
            .map_err(|e| e.to_string())?;
        Ok(metadata.accounts.keys().cloned().collect())
    }

    async fn clear(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 全アカウントの秘密鍵を削除
        let metadata = Self::get_accounts_metadata()
            .map_err(|e| e.to_string())?;
        for npub in metadata.accounts.keys() {
            Self::delete_private_key(npub)
                .map_err(|e| e.to_string())?;
        }
        // メタデータをクリア
        Self::save_accounts_metadata(&AccountsMetadata::default())
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_secure_storage_store_retrieve() {
        let storage = DefaultSecureStorage::new();
        
        // Store a value
        let result = storage.store("test_key", "test_value").await;
        assert!(result.is_ok());
        
        // Retrieve the value
        let result = storage.retrieve("test_key").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("test_value".to_string()));
        
        // Clean up
        let _ = storage.delete("test_key").await;
    }

    #[tokio::test]
    async fn test_secure_storage_delete() {
        let storage = DefaultSecureStorage::new();
        
        // Store a value
        let _ = storage.store("test_delete_key", "test_value").await;
        
        // Delete it
        let result = storage.delete("test_delete_key").await;
        assert!(result.is_ok());
        
        // Verify it's deleted
        let result = storage.retrieve("test_delete_key").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_secure_storage_exists() {
        let storage = DefaultSecureStorage::new();
        
        // Check non-existent key
        let result = storage.exists("non_existent_key").await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
        
        // Store a value
        let _ = storage.store("test_exists_key", "test_value").await;
        
        // Check it exists
        let result = storage.exists("test_exists_key").await;
        assert!(result.is_ok());
        assert!(result.unwrap());
        
        // Clean up
        let _ = storage.delete("test_exists_key").await;
    }

    #[test]
    fn test_add_account() {
        let result = DefaultSecureStorage::add_account(
            "npub1test",
            "nsec1test",
            "pubkey_test",
            "test_user",
            "Test User",
            None,
        );
        
        // Clean up
        let _ = DefaultSecureStorage::remove_account("npub1test");
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_accounts() {
        // Add an account
        let _ = DefaultSecureStorage::add_account(
            "npub1list",
            "nsec1list",
            "pubkey_list",
            "list_user",
            "List User",
            None,
        );
        
        let result = DefaultSecureStorage::list_accounts();
        
        // Clean up
        let _ = DefaultSecureStorage::remove_account("npub1list");
        
        assert!(result.is_ok());
        let accounts = result.unwrap();
        assert!(accounts.iter().any(|a| a.npub == "npub1list"));
    }
}