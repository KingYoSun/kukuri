use crate::shared::error::AppError;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct MessagingSendResult {
    pub event_id: Option<String>,
    pub ciphertext: String,
    pub created_at_millis: i64,
    pub delivered: bool,
}

#[async_trait]
pub trait MessagingGateway: Send + Sync {
    async fn encrypt_and_send(
        &self,
        owner_npub: &str,
        recipient_npub: &str,
        plaintext: &str,
    ) -> Result<MessagingSendResult, AppError>;

    async fn encrypt_only(
        &self,
        owner_npub: &str,
        recipient_npub: &str,
        plaintext: &str,
    ) -> Result<String, AppError>;

    async fn decrypt_with_counterparty(
        &self,
        owner_npub: &str,
        counterparty_npub: &str,
        ciphertext: &str,
    ) -> Result<String, AppError>;
}
