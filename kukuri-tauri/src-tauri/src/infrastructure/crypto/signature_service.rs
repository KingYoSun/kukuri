use crate::domain::entities::Event;
use async_trait::async_trait;

#[async_trait]
pub trait SignatureService: Send + Sync {
    async fn sign_event(&self, event: &mut Event, private_key: &str) -> Result<(), Box<dyn std::error::Error>>;
    async fn verify_event(&self, event: &Event) -> Result<bool, Box<dyn std::error::Error>>;
    async fn sign_message(&self, message: &str, private_key: &str) -> Result<String, Box<dyn std::error::Error>>;
    async fn verify_message(&self, message: &str, signature: &str, public_key: &str) -> Result<bool, Box<dyn std::error::Error>>;
}