use crate::application::ports::{
    key_manager::{KeyMaterialStore, KeyPair},
    secure_storage::SecureAccountStore,
};
use crate::domain::entities::{
    AccountMetadata, AccountRegistration, AccountsMetadata, CurrentAccountSecret,
};
use crate::domain::value_objects::keychain::{KeyMaterialLedger, KeyMaterialRecord};
use crate::shared::error::AppError;
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use keyring::Entry;
use nostr_sdk::{FromBech32, prelude::SecretKey};
use tracing::{debug, error};

const SERVICE_NAME: &str = "kukuri";
const ACCOUNTS_KEY: &str = "accounts_metadata";
const KEY_MANAGER_LEDGER_KEY: &str = "key_manager_ledger";

/// セキュアストレージのトレイト
#[async_trait]
pub trait SecureStorage: Send + Sync {
    async fn store(
        &self,
        key: &str,
        value: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn retrieve(
        &self,
        key: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>>;
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

    #[cfg(debug_assertions)]
    pub fn clear_all_accounts_for_test() -> Result<()> {
        let metadata = Self::get_accounts_metadata()?;
        for npub in metadata.accounts.keys() {
            Self::delete_private_key(npub)?;
        }

        let entry =
            Entry::new(SERVICE_NAME, ACCOUNTS_KEY).context("Failed to create keyring entry")?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("Failed to delete accounts metadata: {e}")),
        }
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
                Err(anyhow::anyhow!(
                    "Failed to save private key to keyring: {e}"
                ))
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

        let entry =
            Entry::new(SERVICE_NAME, ACCOUNTS_KEY).context("Failed to create keyring entry")?;

        match entry.set_password(&json) {
            Ok(_) => {
                debug!("SecureStorage: Metadata saved to keyring successfully");

                // デバッグ: 保存直後に読み取りテスト
                debug!("SecureStorage: Testing immediate read after save...");
                let test_entry = Entry::new(SERVICE_NAME, ACCOUNTS_KEY)
                    .context("Failed to create test entry")?;

                match test_entry.get_password() {
                    Ok(test_json) => {
                        debug!(
                            "SecureStorage: Immediate read test succeeded, data length: {}",
                            test_json.len()
                        );
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

        let entry =
            Entry::new(SERVICE_NAME, ACCOUNTS_KEY).context("Failed to create keyring entry")?;

        match entry.get_password() {
            Ok(json) => {
                debug!("SecureStorage: Retrieved metadata JSON: {json}");
                let metadata: AccountsMetadata = serde_json::from_str(&json)
                    .context("Failed to deserialize accounts metadata")?;
                debug!(
                    "SecureStorage: Deserialized metadata - current_npub: {:?}, accounts: {}",
                    metadata.current_npub,
                    metadata.accounts.len()
                );
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

    fn get_key_material_ledger() -> Result<KeyMaterialLedger> {
        let entry = Entry::new(SERVICE_NAME, KEY_MANAGER_LEDGER_KEY)
            .context("Failed to create keyring entry")?;
        match entry.get_password() {
            Ok(json) => {
                debug!("SecureStorage: Retrieved key ledger JSON");
                serde_json::from_str(&json).context("Failed to deserialize key material ledger")
            }
            Err(keyring::Error::NoEntry) => Ok(KeyMaterialLedger::default()),
            Err(e) => Err(anyhow::anyhow!("Failed to get key ledger: {e}")),
        }
    }

    fn save_key_material_ledger(ledger: &KeyMaterialLedger) -> Result<()> {
        let json = serde_json::to_string(ledger).context("Failed to serialize key ledger")?;
        let entry = Entry::new(SERVICE_NAME, KEY_MANAGER_LEDGER_KEY)
            .context("Failed to create keyring entry")?;
        entry
            .set_password(&json)
            .map_err(|e| anyhow::anyhow!("Failed to save key ledger: {e}"))
    }
}

impl Default for DefaultSecureStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SecureStorage for DefaultSecureStorage {
    async fn store(
        &self,
        key: &str,
        value: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let entry = Entry::new(SERVICE_NAME, key).map_err(|e| e.to_string())?;
        entry.set_password(value).map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn retrieve(
        &self,
        key: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        let entry = Entry::new(SERVICE_NAME, key).map_err(|e| e.to_string())?;
        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
        }
    }

    async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let entry = Entry::new(SERVICE_NAME, key).map_err(|e| e.to_string())?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()), // Already deleted is OK
            Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
        }
    }

    async fn exists(&self, key: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let entry = Entry::new(SERVICE_NAME, key).map_err(|e| e.to_string())?;
        match entry.get_password() {
            Ok(_) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
        }
    }

    async fn list_keys(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        // keyringライブラリは直接的なキーのリストをサポートしていないため、
        // アカウントメタデータから取得
        let metadata = Self::get_accounts_metadata().map_err(|e| e.to_string())?;
        Ok(metadata.accounts.keys().cloned().collect())
    }

    async fn clear(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 全アカウントの秘密鍵を削除
        let metadata = Self::get_accounts_metadata().map_err(|e| e.to_string())?;
        for npub in metadata.accounts.keys() {
            Self::delete_private_key(npub).map_err(|e| e.to_string())?;
        }
        // メタデータをクリア
        Self::save_accounts_metadata(&AccountsMetadata::default()).map_err(|e| e.to_string())?;
        let entry = Entry::new(SERVICE_NAME, KEY_MANAGER_LEDGER_KEY).map_err(|e| e.to_string())?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(e) => return Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
        }
        Ok(())
    }
}

fn to_storage_error(err: anyhow::Error) -> AppError {
    AppError::Storage(err.to_string())
}

fn build_keypair_from_record(
    record: &KeyMaterialRecord,
    nsec: String,
) -> Result<KeyPair, AppError> {
    let secret_key = SecretKey::from_bech32(&nsec)
        .map_err(|e| AppError::Crypto(format!("Invalid nsec: {e}")))?;
    Ok(KeyPair {
        public_key: record.public_key.clone(),
        private_key: secret_key.display_secret().to_string(),
        npub: record.npub.clone(),
        nsec,
    })
}

#[async_trait]
impl SecureAccountStore for DefaultSecureStorage {
    async fn add_account(
        &self,
        registration: AccountRegistration,
    ) -> Result<AccountMetadata, AppError> {
        let (mut metadata, nsec) = registration.into_metadata();
        debug!("SecureStorage: Adding account npub={}", metadata.npub);

        Self::save_private_key(&metadata.npub, &nsec).map_err(to_storage_error)?;

        let mut accounts = Self::get_accounts_metadata().map_err(to_storage_error)?;
        metadata.mark_used(Utc::now());
        accounts
            .accounts
            .insert(metadata.npub.clone(), metadata.clone());
        accounts.current_npub = Some(metadata.npub.clone());
        Self::save_accounts_metadata(&accounts).map_err(to_storage_error)?;
        {
            let mut ledger = Self::get_key_material_ledger().map_err(to_storage_error)?;
            let entry = ledger
                .records
                .entry(metadata.npub.clone())
                .or_insert_with(|| {
                    KeyMaterialRecord::new(metadata.npub.clone(), metadata.pubkey.clone())
                });
            entry.public_key = metadata.pubkey.clone();
            entry.touch();
            ledger.touch_current(&metadata.npub);
            Self::save_key_material_ledger(&ledger).map_err(to_storage_error)?;
        }

        Ok(metadata)
    }

    async fn list_accounts(&self) -> Result<Vec<AccountMetadata>, AppError> {
        let metadata = Self::get_accounts_metadata().map_err(to_storage_error)?;
        let mut accounts: Vec<AccountMetadata> = metadata.accounts.values().cloned().collect();
        accounts.sort_by(|a, b| b.last_used.cmp(&a.last_used));
        Ok(accounts)
    }

    async fn remove_account(&self, npub: &str) -> Result<(), AppError> {
        Self::delete_private_key(npub).map_err(to_storage_error)?;

        let mut metadata = Self::get_accounts_metadata().map_err(to_storage_error)?;
        metadata.accounts.remove(npub);
        if metadata.current_npub.as_deref() == Some(npub) {
            metadata.current_npub = metadata.accounts.keys().next().cloned();
        }
        Self::save_accounts_metadata(&metadata).map_err(to_storage_error)?;
        let mut ledger = Self::get_key_material_ledger().map_err(to_storage_error)?;
        ledger.remove(npub);
        Self::save_key_material_ledger(&ledger).map_err(to_storage_error)?;

        Ok(())
    }

    async fn switch_account(&self, npub: &str) -> Result<AccountMetadata, AppError> {
        let mut metadata = Self::get_accounts_metadata().map_err(to_storage_error)?;
        let account = metadata
            .accounts
            .get_mut(npub)
            .ok_or_else(|| AppError::NotFound(format!("Account not found: {npub}")))?;
        account.mark_used(Utc::now());
        let updated = account.clone();
        metadata.current_npub = Some(npub.to_string());
        Self::save_accounts_metadata(&metadata).map_err(to_storage_error)?;
        let mut ledger = Self::get_key_material_ledger().map_err(to_storage_error)?;
        ledger.touch_current(npub);
        Self::save_key_material_ledger(&ledger).map_err(to_storage_error)?;

        Ok(updated)
    }

    async fn get_private_key(&self, npub: &str) -> Result<Option<String>, AppError> {
        Self::get_private_key(npub).map_err(to_storage_error)
    }

    async fn current_account(&self) -> Result<Option<CurrentAccountSecret>, AppError> {
        let metadata = Self::get_accounts_metadata().map_err(to_storage_error)?;
        if let Some(current) = metadata.current_npub.as_ref() {
            if let Some(account) = metadata.accounts.get(current) {
                if let Some(nsec) = Self::get_private_key(current).map_err(to_storage_error)? {
                    return Ok(Some(CurrentAccountSecret {
                        metadata: account.clone(),
                        nsec,
                    }));
                }
            }
        }
        Ok(None)
    }
}

#[async_trait]
impl KeyMaterialStore for DefaultSecureStorage {
    async fn save_keypair(&self, keypair: &KeyPair) -> Result<(), AppError> {
        Self::save_private_key(&keypair.npub, &keypair.nsec).map_err(to_storage_error)?;
        let mut ledger = Self::get_key_material_ledger().map_err(to_storage_error)?;
        let entry = ledger
            .records
            .entry(keypair.npub.clone())
            .or_insert_with(|| {
                KeyMaterialRecord::new(keypair.npub.clone(), keypair.public_key.clone())
            });
        entry.public_key = keypair.public_key.clone();
        entry.touch();
        ledger.touch_current(&keypair.npub);
        Self::save_key_material_ledger(&ledger).map_err(to_storage_error)
    }

    async fn delete_keypair(&self, npub: &str) -> Result<(), AppError> {
        let mut ledger = Self::get_key_material_ledger().map_err(to_storage_error)?;
        ledger.remove(npub);
        Self::delete_private_key(npub).map_err(to_storage_error)?;
        Self::save_key_material_ledger(&ledger).map_err(to_storage_error)
    }

    async fn get_keypair(&self, npub: &str) -> Result<Option<KeyPair>, AppError> {
        let ledger = Self::get_key_material_ledger().map_err(to_storage_error)?;
        match ledger.records.get(npub) {
            Some(record) => {
                let nsec = Self::get_private_key(&record.npub).map_err(to_storage_error)?;
                if let Some(nsec) = nsec {
                    build_keypair_from_record(record, nsec)
                } else {
                    Err(AppError::NotFound(format!(
                        "Private key not found for {}",
                        record.npub
                    )))
                }
                .map(Some)
            }
            None => Ok(None),
        }
    }

    async fn list_keypairs(&self) -> Result<Vec<KeyPair>, AppError> {
        let ledger = Self::get_key_material_ledger().map_err(to_storage_error)?;
        let mut pairs = Vec::with_capacity(ledger.records.len());
        for record in ledger.records.values() {
            let nsec = Self::get_private_key(&record.npub).map_err(to_storage_error)?;
            if let Some(nsec) = nsec {
                pairs.push(build_keypair_from_record(record, nsec)?);
            }
        }
        Ok(pairs)
    }

    async fn set_current(&self, npub: &str) -> Result<(), AppError> {
        let mut ledger = Self::get_key_material_ledger().map_err(to_storage_error)?;
        if ledger.records.contains_key(npub) {
            ledger.touch_current(npub);
            Self::save_key_material_ledger(&ledger).map_err(to_storage_error)
        } else {
            Err(AppError::NotFound(format!(
                "Keypair not found for npub {npub}"
            )))
        }
    }

    async fn current_keypair(&self) -> Result<Option<KeyPair>, AppError> {
        let ledger = Self::get_key_material_ledger().map_err(to_storage_error)?;
        if let Some(npub) = ledger.current_npub.as_deref() {
            if let Some(record) = ledger.records.get(npub) {
                let nsec = Self::get_private_key(&record.npub).map_err(to_storage_error)?;
                if let Some(nsec) = nsec {
                    return build_keypair_from_record(record, nsec).map(Some);
                }
            }
        }
        Ok(None)
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    use crate::application::ports::secure_storage::SecureAccountStore;
    use crate::domain::entities::AccountRegistration;

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

    #[tokio::test]
    async fn test_add_account() {
        let storage = DefaultSecureStorage::new();
        let registration = AccountRegistration {
            npub: "npub1test".to_string(),
            nsec: "nsec1test".to_string(),
            pubkey: "pubkey_test".to_string(),
            name: "test_user".to_string(),
            display_name: "Test User".to_string(),
            picture: None,
        };
        let npub = registration.npub.clone();
        let result = SecureAccountStore::add_account(&storage, registration).await;

        // Clean up
        let _ = SecureAccountStore::remove_account(&storage, &npub).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_accounts() {
        let storage = DefaultSecureStorage::new();
        // Add an account
        let registration = AccountRegistration {
            npub: "npub1list".to_string(),
            nsec: "nsec1list".to_string(),
            pubkey: "pubkey_list".to_string(),
            name: "list_user".to_string(),
            display_name: "List User".to_string(),
            picture: None,
        };
        let npub = registration.npub.clone();
        let _ = SecureAccountStore::add_account(&storage, registration).await;

        let result = SecureAccountStore::list_accounts(&storage).await;

        // Clean up
        let _ = SecureAccountStore::remove_account(&storage, &npub).await;

        assert!(result.is_ok());
        let accounts = result.unwrap();
        assert!(accounts.iter().any(|a| a.npub == "npub1list"));
    }
}
