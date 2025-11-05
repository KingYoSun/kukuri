use crate::application::ports::key_manager::KeyManager;
use crate::application::ports::messaging_gateway::{MessagingGateway, MessagingSendResult};
use crate::infrastructure::event::manager_handle::EventManagerHandle;
use crate::shared::{AppError, ValidationFailureKind};
use async_trait::async_trait;
use nostr_sdk::prelude::{EventBuilder, FromBech32, Keys, Kind, PublicKey, SecretKey, Tag, nip04};
use std::sync::Arc;
use tracing::warn;

pub struct NostrMessagingGateway {
    key_manager: Arc<dyn KeyManager>,
    event_manager: Arc<dyn EventManagerHandle>,
}

impl NostrMessagingGateway {
    pub fn new(
        key_manager: Arc<dyn KeyManager>,
        event_manager: Arc<dyn EventManagerHandle>,
    ) -> Self {
        Self {
            key_manager,
            event_manager,
        }
    }

    async fn load_keys(&self, owner_npub: &str) -> Result<Keys, AppError> {
        let nsec = self.key_manager.export_private_key(owner_npub).await?;
        let secret_key = SecretKey::from_bech32(&nsec)
            .map_err(|err| AppError::Crypto(format!("Invalid nsec for {owner_npub}: {err}")))?;
        Ok(Keys::new(secret_key))
    }

    fn parse_npub(npub: &str) -> Result<PublicKey, AppError> {
        PublicKey::from_bech32(npub).map_err(|err| AppError::ValidationError {
            kind: ValidationFailureKind::Generic,
            message: format!("Invalid npub {npub}: {err}"),
        })
    }

    fn encrypt_payload(
        sender_secret: &SecretKey,
        recipient: &PublicKey,
        plaintext: &str,
    ) -> Result<String, AppError> {
        nip04::encrypt(sender_secret, recipient, plaintext)
            .map_err(|err| AppError::Crypto(format!("Failed to encrypt direct message: {err}")))
    }

    fn decrypt_payload(
        owner_secret: &SecretKey,
        counterparty: &PublicKey,
        ciphertext: &str,
    ) -> Result<String, AppError> {
        nip04::decrypt(owner_secret, counterparty, ciphertext)
            .map_err(|err| AppError::Crypto(format!("Failed to decrypt direct message: {err}")))
    }
}

#[async_trait]
impl MessagingGateway for NostrMessagingGateway {
    async fn encrypt_and_send(
        &self,
        owner_npub: &str,
        recipient_npub: &str,
        plaintext: &str,
    ) -> Result<MessagingSendResult, AppError> {
        let keys = self.load_keys(owner_npub).await?;
        let recipient_pk = Self::parse_npub(recipient_npub)?;

        let ciphertext = Self::encrypt_payload(keys.secret_key(), &recipient_pk, plaintext)?;

        let event = EventBuilder::new(Kind::EncryptedDirectMessage, ciphertext.clone())
            .tags([Tag::public_key(recipient_pk)])
            .sign_with_keys(&keys)?;

        if let Err(err) = self.event_manager.publish_event(event.clone()).await {
            warn!("Failed to publish direct message event: {err}");
            return Err(AppError::NostrError(format!(
                "Failed to publish direct message: {err}"
            )));
        }

        let created_at_millis = (event.created_at.as_u64() as i64) * 1000;
        let event_id = event.id.to_hex();

        Ok(MessagingSendResult {
            event_id: Some(event_id),
            ciphertext,
            created_at_millis,
            delivered: true,
        })
    }

    async fn encrypt_only(
        &self,
        owner_npub: &str,
        recipient_npub: &str,
        plaintext: &str,
    ) -> Result<String, AppError> {
        let keys = self.load_keys(owner_npub).await?;
        let recipient_pk = Self::parse_npub(recipient_npub)?;
        Self::encrypt_payload(keys.secret_key(), &recipient_pk, plaintext)
    }

    async fn decrypt_with_counterparty(
        &self,
        owner_npub: &str,
        counterparty_npub: &str,
        ciphertext: &str,
    ) -> Result<String, AppError> {
        let keys = self.load_keys(owner_npub).await?;
        let counterparty_pk = Self::parse_npub(counterparty_npub)?;
        Self::decrypt_payload(keys.secret_key(), &counterparty_pk, ciphertext)
    }
}
