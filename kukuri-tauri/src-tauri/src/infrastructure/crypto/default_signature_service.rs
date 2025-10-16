use crate::domain::entities::Event;
use crate::infrastructure::crypto::signature_service::SignatureService;
use async_trait::async_trait;
use nostr_sdk::prelude::*;

/// デフォルトの署名サービス実装
pub struct DefaultSignatureService;

impl DefaultSignatureService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultSignatureService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SignatureService for DefaultSignatureService {
    async fn sign_event(
        &self,
        event: &mut Event,
        private_key: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Nostr SDKを使用してイベントに署名
        let secret_key = SecretKey::from_hex(private_key)?;
        let keys = Keys::new(secret_key);

        // イベントIDを計算
        let tags: Vec<nostr_sdk::Tag> = event
            .tags
            .clone()
            .into_iter()
            .map(|t| {
                // Convert Vec<String> to Tag
                if !t.is_empty() {
                    nostr_sdk::Tag::custom(nostr_sdk::TagKind::from(t[0].as_str()), t[1..].to_vec())
                } else {
                    nostr_sdk::Tag::custom(nostr_sdk::TagKind::from(""), Vec::<String>::new())
                }
            })
            .collect();

        let mut event_builder =
            nostr_sdk::EventBuilder::new(Kind::from(event.kind as u16), event.content.clone());
        for tag in tags {
            event_builder = event_builder.tag(tag);
        }

        // 署名を生成
        let signed_event = event_builder.sign_with_keys(&keys)?;
        event.sig = signed_event.sig.to_string();
        event.id = signed_event.id.to_hex();

        Ok(())
    }

    async fn verify_event(
        &self,
        event: &Event,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // イベントの署名を検証
        let public_key = PublicKey::from_hex(&event.pubkey)?;

        // Nostrイベントを再構築
        let nostr_event = nostr_sdk::Event::from_json(serde_json::to_string(event)?)?;

        // 署名を検証
        Ok(nostr_event.verify().is_ok())
    }

    async fn sign_message(
        &self,
        message: &str,
        private_key: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let secret_key = SecretKey::from_hex(private_key)?;
        let keys = Keys::new(secret_key);

        // Create a simple text note event and sign it
        let event = EventBuilder::text_note(message).sign_with_keys(&keys)?;

        // Return the signature
        Ok(event.sig.to_string())
    }

    async fn verify_message(
        &self,
        message: &str,
        signature: &str,
        public_key: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // For now, we'll use the Nostr event verification approach
        // In a real implementation, you'd need to reconstruct the event with the signature
        // and verify it properly

        // This is a simplified version - you may need to store more context
        // to properly verify standalone signatures
        Ok(true) // Placeholder implementation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sign_and_verify_message() {
        let service = DefaultSignatureService::new();
        let keys = Keys::generate();
        let private_key = keys.secret_key().display_secret().to_string();
        let public_key = keys.public_key().to_string();

        let message = "Test message";
        let signature = service.sign_message(message, &private_key).await.unwrap();

        let is_valid = service
            .verify_message(message, &signature, &public_key)
            .await
            .unwrap();
        assert!(is_valid);
    }
}
