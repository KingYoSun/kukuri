use crate::domain::entities::{AccountMetadata, AccountRegistration, CurrentAccountSecret};
use crate::shared::error::AppError;
use async_trait::async_trait;

#[async_trait]
pub trait SecureAccountStore: Send + Sync {
    async fn add_account(
        &self,
        registration: AccountRegistration,
    ) -> Result<AccountMetadata, AppError>;
    async fn list_accounts(&self) -> Result<Vec<AccountMetadata>, AppError>;
    async fn remove_account(&self, npub: &str) -> Result<(), AppError>;
    async fn switch_account(&self, npub: &str) -> Result<AccountMetadata, AppError>;
    async fn get_private_key(&self, npub: &str) -> Result<Option<String>, AppError>;
    async fn current_account(&self) -> Result<Option<CurrentAccountSecret>, AppError>;
}
