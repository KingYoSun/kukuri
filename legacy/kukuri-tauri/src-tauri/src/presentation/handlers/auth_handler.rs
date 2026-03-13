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
        let (user, keypair) = self.auth_service.create_account_with_keys().await?;

        Ok(CreateAccountResponse {
            npub: user.npub.clone(),
            nsec: keypair.nsec.clone(),
            pubkey: user.pubkey.clone(),
        })
    }

    pub async fn login_with_nsec(
        &self,
        request: LoginWithNsecRequest,
    ) -> Result<LoginResponse, AppError> {
        request.validate().map_err(AppError::InvalidInput)?;

        let user = self.auth_service.login_with_nsec(&request.nsec).await?;

        Ok(LoginResponse {
            success: true,
            npub: user.npub,
            pubkey: user.pubkey,
        })
    }

    pub async fn logout(&self, npub: String) -> Result<(), AppError> {
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

    pub async fn export_private_key(&self, npub: &str) -> Result<String, AppError> {
        self.auth_service.export_private_key(npub).await
    }
}
