use async_trait::async_trait;

#[async_trait]
pub trait EncryptionService: Send + Sync {
    async fn encrypt(
        &self,
        data: &[u8],
        recipient_pubkey: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
    async fn decrypt(
        &self,
        encrypted_data: &[u8],
        sender_pubkey: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
    async fn encrypt_symmetric(
        &self,
        data: &[u8],
        password: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
    async fn decrypt_symmetric(
        &self,
        encrypted_data: &[u8],
        password: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
}
