mod commands;
mod fallback;

pub use commands::*;

use anyhow::{Context, Result};
use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use fallback::FallbackStorage;

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

pub struct SecureStorage;

impl SecureStorage {
    /// WSL環境かどうかを検出
    fn is_wsl() -> bool {
        if cfg!(target_os = "linux") {
            if let Ok(osrelease) = std::fs::read_to_string("/proc/sys/kernel/osrelease") {
                return osrelease.to_lowercase().contains("microsoft");
            }
        }
        false
    }
    /// 秘密鍵を保存（npubごとに個別保存）
    pub fn save_private_key(npub: &str, nsec: &str) -> Result<()> {
        println!("SecureStorage: Saving private key for npub={}", npub);
        
        if Self::is_wsl() {
            println!("SecureStorage: WSL detected, using fallback storage");
            return FallbackStorage::save_data(&format!("key_{}", npub), nsec);
        }
        
        let entry = Entry::new(SERVICE_NAME, npub).context("Failed to create keyring entry")?;
        
        match entry.set_password(nsec) {
            Ok(_) => {
                println!("SecureStorage: Private key saved successfully for npub={}", npub);
                Ok(())
            }
            Err(e) => {
                eprintln!("SecureStorage: Failed to save private key: {:?}", e);
                Err(anyhow::anyhow!("Failed to save private key to keyring: {}", e))
            }
        }
    }

    /// 秘密鍵を取得
    pub fn get_private_key(npub: &str) -> Result<Option<String>> {
        if Self::is_wsl() {
            return FallbackStorage::get_data(&format!("key_{}", npub));
        }
        
        let entry = Entry::new(SERVICE_NAME, npub).context("Failed to create keyring entry")?;
        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("Failed to get private key: {}", e)),
        }
    }

    /// 秘密鍵を削除
    pub fn delete_private_key(npub: &str) -> Result<()> {
        if Self::is_wsl() {
            return FallbackStorage::delete_data(&format!("key_{}", npub));
        }
        
        let entry = Entry::new(SERVICE_NAME, npub).context("Failed to create keyring entry")?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()), // 既に削除されている場合もOK
            Err(e) => Err(anyhow::anyhow!("Failed to delete private key: {}", e)),
        }
    }

    /// アカウントメタデータを保存（公開情報のみ）
    pub fn save_accounts_metadata(metadata: &AccountsMetadata) -> Result<()> {
        let json =
            serde_json::to_string(metadata).context("Failed to serialize accounts metadata")?;
        println!("SecureStorage: Saving metadata JSON: {}", json);
        
        if Self::is_wsl() {
            println!("SecureStorage: WSL detected, using fallback storage for metadata");
            return FallbackStorage::save_data(ACCOUNTS_KEY, &json);
        }
        
        let entry =
            Entry::new(SERVICE_NAME, ACCOUNTS_KEY).context("Failed to create keyring entry")?;
        
        match entry.set_password(&json) {
            Ok(_) => {
                println!("SecureStorage: Metadata saved to keyring successfully");
                Ok(())
            }
            Err(e) => {
                eprintln!("SecureStorage: Failed to save metadata to keyring: {:?}", e);
                Err(anyhow::anyhow!("Failed to save accounts metadata: {}", e))
            }
        }
    }

    /// アカウントメタデータを取得
    pub fn get_accounts_metadata() -> Result<AccountsMetadata> {
        println!("SecureStorage: Getting accounts metadata from keyring...");
        
        if Self::is_wsl() {
            println!("SecureStorage: WSL detected, using fallback storage for metadata");
            if let Some(json) = FallbackStorage::get_data(ACCOUNTS_KEY)? {
                println!("SecureStorage: Retrieved metadata JSON: {}", json);
                let metadata: AccountsMetadata = serde_json::from_str(&json)
                    .context("Failed to deserialize accounts metadata")?;
                println!("SecureStorage: Deserialized metadata - current_npub: {:?}, accounts: {}", 
                    metadata.current_npub, metadata.accounts.len());
                return Ok(metadata);
            } else {
                println!("SecureStorage: No metadata found in fallback storage, returning default");
                return Ok(AccountsMetadata::default());
            }
        }
        
        let entry =
            Entry::new(SERVICE_NAME, ACCOUNTS_KEY).context("Failed to create keyring entry")?;
        
        match entry.get_password() {
            Ok(json) => {
                println!("SecureStorage: Retrieved metadata JSON: {}", json);
                let metadata: AccountsMetadata = serde_json::from_str(&json)
                    .context("Failed to deserialize accounts metadata")?;
                println!("SecureStorage: Deserialized metadata - current_npub: {:?}, accounts: {}", 
                    metadata.current_npub, metadata.accounts.len());
                Ok(metadata)
            }
            Err(keyring::Error::NoEntry) => {
                println!("SecureStorage: No metadata entry found in keyring, returning default");
                Ok(AccountsMetadata::default())
            }
            Err(e) => {
                eprintln!("SecureStorage: Failed to get metadata from keyring: {:?}", e);
                Err(anyhow::anyhow!("Failed to get accounts metadata: {}", e))
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
        println!("SecureStorage: Adding account npub={}", npub);
        
        // 秘密鍵を保存
        Self::save_private_key(npub, nsec)?;
        println!("SecureStorage: Private key saved");

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
        println!("SecureStorage: Metadata saved with current_npub={}", npub);

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
            Err(anyhow::anyhow!("Account not found: {}", npub))
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
        println!("SecureStorage: current_npub = {:?}", metadata.current_npub);
        println!("SecureStorage: accounts = {:?}", metadata.accounts.keys().collect::<Vec<_>>());
        
        if let Some(npub) = metadata.current_npub {
            if let Some(nsec) = Self::get_private_key(&npub)? {
                println!("SecureStorage: Found private key for npub={}", npub);
                Ok(Some((npub, nsec)))
            } else {
                println!("SecureStorage: No private key found for npub={}", npub);
                Ok(None)
            }
        } else {
            println!("SecureStorage: No current_npub set");
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests;
