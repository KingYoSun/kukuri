use crate::{
    application::services::AuthService,
    presentation::dto::{
        Validate,
        auth_dto::{CreateAccountResponse, LoginResponse, LoginWithNsecRequest},
    },
    shared::error::AppError,
};
use std::sync::Arc;

pub struct AuthHandler {
    auth_service: Arc<AuthService>,
}

impl AuthHandler {
    pub fn new(auth_service: Arc<AuthService>) -> Self {
        Self { auth_service }
    }

    pub async fn create_account(&self) -> Result<CreateAccountResponse, AppError> {
        let user = self.auth_service.create_account().await?;

        // nsecの生成（実際の実装では秘密鍵から生成）
        let nsec = format!("nsec1{}", &user.pubkey[..32]); // 仮実装

        Ok(CreateAccountResponse {
            npub: user.npub,
            nsec,
            pubkey: user.pubkey,
        })
    }

    pub async fn login_with_nsec(
        &self,
        request: LoginWithNsecRequest,
    ) -> Result<LoginResponse, AppError> {
        request.validate().map_err(|e| AppError::InvalidInput(e))?;

        let user = self.auth_service.login_with_nsec(&request.nsec).await?;

        Ok(LoginResponse {
            success: true,
            npub: user.npub,
            pubkey: user.pubkey,
        })
    }

    pub async fn login_with_npub(&self, npub: String) -> Result<LoginResponse, AppError> {
        let user = self.auth_service.login_with_npub(&npub).await?;

        Ok(LoginResponse {
            success: true,
            npub: user.npub,
            pubkey: user.pubkey,
        })
    }

    pub async fn logout(&self, npub: String) -> Result<(), AppError> {
        // npubは使用しない（現在のユーザーをログアウト）
        let _ = npub;
        self.auth_service.logout().await?;
        Ok(())
    }

    pub async fn get_current_user(&self) -> Result<Option<LoginResponse>, AppError> {
        match self.auth_service.get_current_user().await? {
            Some(user) => Ok(Some(LoginResponse {
                success: true,
                npub: user.npub,
                pubkey: user.pubkey,
            })),
            None => Ok(None),
        }
    }
}
