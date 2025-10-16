use crate::{
    application::services::AuthService,
    infrastructure::storage::secure_storage::{AccountMetadata, DefaultSecureStorage},
    presentation::dto::{Validate, auth_dto::LoginResponse},
    shared::error::AppError,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct AddAccountRequest {
    pub nsec: String,
    pub name: String,
    pub display_name: String,
    pub picture: Option<String>,
}

impl Validate for AddAccountRequest {
    fn validate(&self) -> Result<(), String> {
        if self.nsec.is_empty() {
            return Err("nsec is required".into());
        }
        if self.name.is_empty() {
            return Err("name is required".into());
        }
        if self.display_name.is_empty() {
            return Err("display_name is required".into());
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddAccountResponse {
    pub npub: String,
    pub pubkey: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwitchAccountResponse {
    pub npub: String,
    pub pubkey: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetCurrentAccountResponse {
    pub npub: String,
    pub nsec: String,
    pub pubkey: String,
    pub metadata: AccountMetadata,
}

pub struct SecureStorageHandler {
    auth_service: Arc<AuthService>,
}

impl SecureStorageHandler {
    pub fn new(auth_service: Arc<AuthService>) -> Self {
        Self { auth_service }
    }

    pub async fn add_account(
        &self,
        request: AddAccountRequest,
    ) -> Result<AddAccountResponse, AppError> {
        request.validate().map_err(|e| AppError::InvalidInput(e))?;

        // nsecから公開鍵とnpubを生成
        let user = self.auth_service.login_with_nsec(&request.nsec).await?;

        // セキュアストレージに保存（静的メソッドを使用）
        DefaultSecureStorage::add_account(
            &user.npub,
            &request.nsec,
            &user.pubkey,
            &request.name,
            &request.display_name,
            request.picture,
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(AddAccountResponse {
            npub: user.npub,
            pubkey: user.pubkey,
        })
    }

    pub async fn list_accounts(&self) -> Result<Vec<AccountMetadata>, AppError> {
        DefaultSecureStorage::list_accounts().map_err(|e| AppError::Database(e.to_string()))
    }

    pub async fn switch_account(&self, npub: String) -> Result<SwitchAccountResponse, AppError> {
        // アカウントを切り替え
        DefaultSecureStorage::switch_account(&npub)
            .map_err(|e| AppError::Database(e.to_string()))?;

        // 秘密鍵を取得してログイン
        let nsec = DefaultSecureStorage::get_private_key(&npub)
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Private key not found".into()))?;

        let user = self.auth_service.login_with_nsec(&nsec).await?;

        Ok(SwitchAccountResponse {
            npub: user.npub,
            pubkey: user.pubkey,
        })
    }

    pub async fn remove_account(&self, npub: String) -> Result<(), AppError> {
        DefaultSecureStorage::remove_account(&npub).map_err(|e| AppError::Database(e.to_string()))
    }

    pub async fn get_current_account(&self) -> Result<Option<GetCurrentAccountResponse>, AppError> {
        // 現在のアカウント情報を取得
        if let Some((npub, nsec)) = DefaultSecureStorage::get_current_private_key()
            .map_err(|e| AppError::Database(e.to_string()))?
        {
            // メタデータを取得
            let metadata = DefaultSecureStorage::get_accounts_metadata()
                .map_err(|e| AppError::Database(e.to_string()))?;

            if let Some(account_metadata) = metadata.accounts.get(&npub) {
                // ログイン処理
                let user = self.auth_service.login_with_nsec(&nsec).await?;

                Ok(Some(GetCurrentAccountResponse {
                    npub: user.npub,
                    nsec,
                    pubkey: user.pubkey,
                    metadata: account_metadata.clone(),
                }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub async fn secure_login(&self, npub: String) -> Result<LoginResponse, AppError> {
        // セキュアストレージから秘密鍵を取得
        let nsec = DefaultSecureStorage::get_private_key(&npub)
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Private key not found".into()))?;

        // アカウントを切り替え
        DefaultSecureStorage::switch_account(&npub)
            .map_err(|e| AppError::Database(e.to_string()))?;

        // ログイン処理
        let user = self.auth_service.login_with_nsec(&nsec).await?;

        Ok(LoginResponse {
            success: true,
            npub: user.npub,
            pubkey: user.pubkey,
        })
    }
}
