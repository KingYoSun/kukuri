use crate::{
    application::ports::secure_storage::SecureAccountStore,
    application::services::AuthService,
    domain::entities::{AccountMetadata, AccountRegistration},
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
    secure_store: Arc<dyn SecureAccountStore>,
}

impl SecureStorageHandler {
    pub fn new(auth_service: Arc<AuthService>, secure_store: Arc<dyn SecureAccountStore>) -> Self {
        Self {
            auth_service,
            secure_store,
        }
    }

    pub async fn add_account(
        &self,
        request: AddAccountRequest,
    ) -> Result<AddAccountResponse, AppError> {
        request.validate().map_err(AppError::InvalidInput)?;
        let AddAccountRequest {
            nsec,
            name,
            display_name,
            picture,
        } = request;

        // nsecから公開鍵とnpubを生成
        let user = self.auth_service.login_with_nsec(&nsec).await?;

        // セキュアストレージに保存
        let registration = AccountRegistration {
            npub: user.npub.clone(),
            nsec,
            pubkey: user.pubkey.clone(),
            name,
            display_name,
            picture,
        };
        self.secure_store.add_account(registration).await?;

        Ok(AddAccountResponse {
            npub: user.npub,
            pubkey: user.pubkey,
        })
    }

    pub async fn list_accounts(&self) -> Result<Vec<AccountMetadata>, AppError> {
        self.secure_store.list_accounts().await
    }

    pub async fn switch_account(&self, npub: String) -> Result<SwitchAccountResponse, AppError> {
        // アカウントを切り替え
        self.secure_store.switch_account(&npub).await?;

        // 秘密鍵を取得してログイン
        let nsec = self
            .secure_store
            .get_private_key(&npub)
            .await?
            .ok_or_else(|| AppError::NotFound("Private key not found".into()))?;

        let user = self.auth_service.login_with_nsec(&nsec).await?;

        Ok(SwitchAccountResponse {
            npub: user.npub,
            pubkey: user.pubkey,
        })
    }

    pub async fn remove_account(&self, npub: String) -> Result<(), AppError> {
        self.secure_store.remove_account(&npub).await
    }

    pub async fn get_current_account(&self) -> Result<Option<GetCurrentAccountResponse>, AppError> {
        if let Some(current) = self.secure_store.current_account().await? {
            let user = self.auth_service.login_with_nsec(&current.nsec).await?;

            Ok(Some(GetCurrentAccountResponse {
                npub: user.npub,
                nsec: current.nsec,
                pubkey: user.pubkey,
                metadata: current.metadata,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn secure_login(&self, npub: String) -> Result<LoginResponse, AppError> {
        // セキュアストレージから秘密鍵を取得
        let nsec = self
            .secure_store
            .get_private_key(&npub)
            .await?
            .ok_or_else(|| AppError::NotFound("Private key not found".into()))?;

        // アカウントを切り替え
        self.secure_store.switch_account(&npub).await?;

        // ログイン処理
        let user = self.auth_service.login_with_nsec(&nsec).await?;

        Ok(LoginResponse {
            success: true,
            npub: user.npub,
            pubkey: user.pubkey,
        })
    }
}
