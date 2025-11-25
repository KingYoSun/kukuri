use crate::application::ports::{
    key_manager::{KeyMaterialStore, KeyPair},
    secure_storage::SecureAccountStore,
};
use crate::domain::entities::{
    AccountMetadata, AccountRegistration, AccountsMetadata, CurrentAccountSecret,
};
use crate::domain::value_objects::keychain::{KeyMaterialLedger, KeyMaterialRecord};
use crate::shared::error::AppError;
use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use chrono::Utc;
use keyring::Entry;
use nostr_sdk::{
    FromBech32,
    prelude::{Keys, PublicKey, SecretKey},
};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fmt;
use std::sync::Mutex;
use tracing::{debug, error, warn};

const SERVICE_NAME: &str = "kukuri";
const ACCOUNTS_KEY: &str = "accounts_metadata";
const KEY_MANAGER_LEDGER_KEY: &str = "key_manager_ledger";

static FALLBACK_STORE: Lazy<Mutex<HashMap<String, String>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn fallback_store(key: &str, value: &str) -> Result<()> {
    let mut guard = FALLBACK_STORE
        .lock()
        .map_err(|_| anyhow!("Failed to lock fallback store"))?;
    guard.insert(key.to_string(), value.to_string());
    Ok(())
}

fn fallback_get(key: &str) -> Option<String> {
    FALLBACK_STORE
        .lock()
        .ok()
        .and_then(|guard| guard.get(key).cloned())
}

fn fallback_delete(key: &str) -> Result<()> {
    let mut guard = FALLBACK_STORE
        .lock()
        .map_err(|_| anyhow!("Failed to lock fallback store"))?;
    guard.remove(key);
    Ok(())
}

fn fallback_has(key: &str) -> bool {
    FALLBACK_STORE
        .lock()
        .ok()
        .map(|guard| guard.contains_key(key))
        .unwrap_or(false)
}

fn fallback_clear() -> Result<()> {
    let mut guard = FALLBACK_STORE
        .lock()
        .map_err(|_| anyhow!("Failed to lock fallback store"))?;
    guard.clear();
    Ok(())
}

fn store_with_fallback(key: &str, value: &str) -> Result<()> {
    match Entry::new(SERVICE_NAME, key) {
        Ok(entry) => {
            if let Err(err) = entry.set_password(value) {
                warn!(
                    "SecureStorage: keyring set_password failed for key {}: {err:?}",
                    key
                );
            }
        }
        Err(err) => {
            warn!(
                "SecureStorage: keyring entry creation failed for key {}: {err:?}",
                key
            );
        }
    }
    fallback_store(key, value)
}

fn retrieve_with_fallback(key: &str) -> Result<Option<String>> {
    match Entry::new(SERVICE_NAME, key) {
        Ok(entry) => match entry.get_password() {
            Ok(password) => {
                let _ = fallback_store(key, &password);
                return Ok(Some(password));
            }
            Err(keyring::Error::NoEntry) => {}
            Err(err) => {
                warn!(
                    "SecureStorage: keyring get_password failed for key {}: {err:?}",
                    key
                );
            }
        },
        Err(err) => {
            warn!(
                "SecureStorage: keyring entry creation failed for key {}: {err:?}",
                key
            );
        }
    }
    Ok(fallback_get(key))
}

fn delete_with_fallback(key: &str) -> Result<()> {
    match Entry::new(SERVICE_NAME, key) {
        Ok(entry) => match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(err) => {
                warn!(
                    "SecureStorage: keyring delete failed for key {}: {err:?}",
                    key
                );
            }
        },
        Err(err) => {
            warn!(
                "SecureStorage: keyring entry creation failed for key {}: {err:?}",
                key
            );
        }
    }
    fallback_delete(key)
}

fn exists_with_fallback(key: &str) -> Result<bool> {
    match Entry::new(SERVICE_NAME, key) {
        Ok(entry) => match entry.get_password() {
            Ok(_) => return Ok(true),
            Err(keyring::Error::NoEntry) => {}
            Err(err) => {
                warn!(
                    "SecureStorage: keyring exists check failed for key {}: {err:?}",
                    key
                );
            }
        },
        Err(err) => {
            warn!(
                "SecureStorage: keyring entry creation failed for key {}: {err:?}",
                key
            );
        }
    }
    Ok(fallback_has(key))
}

#[derive(Debug)]
struct SecureStorageError(String);

impl fmt::Display for SecureStorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for SecureStorageError {}

fn boxed_error(e: anyhow::Error) -> Box<dyn std::error::Error + Send + Sync + 'static> {
    Box::new(SecureStorageError(e.to_string()))
}

/// Secure storage trait used by the app and Tauri bridge.
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

/// Default SecureStorage implementation backed by keyring with in-memory fallback.
pub struct DefaultSecureStorage;

impl DefaultSecureStorage {
    pub fn new() -> Self {
        Self
    }

    /// Save private key by npub.
    pub fn save_private_key(npub: &str, nsec: &str) -> Result<()> {
        debug!("SecureStorage: Saving private key for npub={npub}");
        store_with_fallback(npub, nsec)
    }

    /// Get private key by npub.
    pub fn get_private_key(npub: &str) -> Result<Option<String>> {
        retrieve_with_fallback(npub)
    }

    /// Delete private key by npub.
    pub fn delete_private_key(npub: &str) -> Result<()> {
        delete_with_fallback(npub)
    }

    /// Save accounts metadata (includes current npub).
    pub fn save_accounts_metadata(metadata: &AccountsMetadata) -> Result<()> {
        let json =
            serde_json::to_string(metadata).context("Failed to serialize accounts metadata")?;
        debug!("SecureStorage: Saving metadata JSON: {json}");
        store_with_fallback(ACCOUNTS_KEY, &json)?;

        // Read-back check to ensure fallback has the latest value.
        match retrieve_with_fallback(ACCOUNTS_KEY) {
            Ok(Some(test_json)) => {
                debug!(
                    "SecureStorage: Immediate read test succeeded, data length: {}",
                    test_json.len()
                );
            }
            Ok(None) => {
                warn!("SecureStorage: Metadata read-after-write returned empty result");
            }
            Err(err) => {
                error!("SecureStorage: Immediate metadata read failed: {err:?}");
            }
        }

        Ok(())
    }

    /// Fetch accounts metadata or default.
    pub fn get_accounts_metadata() -> Result<AccountsMetadata> {
        debug!("SecureStorage: Getting accounts metadata...");
        match retrieve_with_fallback(ACCOUNTS_KEY) {
            Ok(Some(json)) => {
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
            Ok(None) => {
                debug!("SecureStorage: No metadata entry found, returning default");
                Ok(AccountsMetadata::default())
            }
            Err(err) => {
                error!("SecureStorage: Failed to get metadata: {err:?}");
                Err(err)
            }
        }
    }

    fn get_key_material_ledger() -> Result<KeyMaterialLedger> {
        match retrieve_with_fallback(KEY_MANAGER_LEDGER_KEY) {
            Ok(Some(json)) => {
                debug!("SecureStorage: Retrieved key ledger JSON");
                serde_json::from_str(&json).context("Failed to deserialize key material ledger")
            }
            Ok(None) => Ok(KeyMaterialLedger::default()),
            Err(err) => Err(err),
        }
    }

    fn save_key_material_ledger(ledger: &KeyMaterialLedger) -> Result<()> {
        let json = serde_json::to_string(ledger).context("Failed to serialize key ledger")?;
        store_with_fallback(KEY_MANAGER_LEDGER_KEY, &json)
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
        store_with_fallback(key, value).map_err(boxed_error)
    }

    async fn retrieve(
        &self,
        key: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        retrieve_with_fallback(key).map_err(boxed_error)
    }

    async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        delete_with_fallback(key).map_err(boxed_error)
    }

    async fn exists(&self, key: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        exists_with_fallback(key).map_err(boxed_error)
    }

    async fn list_keys(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        // keyring does not support listing; derive from stored metadata.
        let metadata = Self::get_accounts_metadata().map_err(boxed_error)?;
        Ok(metadata.accounts.keys().cloned().collect())
    }

    async fn clear(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Remove all stored keys and metadata.
        let metadata = Self::get_accounts_metadata().map_err(boxed_error)?;
        for npub in metadata.accounts.keys() {
            Self::delete_private_key(npub).map_err(boxed_error)?;
        }
        Self::save_accounts_metadata(&AccountsMetadata::default()).map_err(boxed_error)?;
        delete_with_fallback(KEY_MANAGER_LEDGER_KEY).map_err(boxed_error)?;
        fallback_clear().map_err(boxed_error)?;
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
        if !ledger.records.contains_key(npub) {
            let public_key = match Self::get_private_key(npub).map_err(to_storage_error)? {
                Some(nsec) => {
                    let secret_key = SecretKey::from_bech32(&nsec).map_err(|e| {
                        AppError::Crypto(format!("Invalid nsec stored for npub {npub}: {e:?}"))
                    })?;
                    Keys::new(secret_key).public_key()
                }
                None => PublicKey::from_bech32(npub).map_err(|e| {
                    AppError::Crypto(format!("Failed to decode npub {npub}: {e:?}"))
                })?,
            };
            let public_key_hex = public_key.to_hex();
            ledger.upsert(KeyMaterialRecord::new(npub.to_string(), public_key_hex));
        }
        ledger.touch_current(npub);
        Self::save_key_material_ledger(&ledger).map_err(to_storage_error)
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

        // Clean up
        let _ = storage.delete("test_delete_key").await;
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
