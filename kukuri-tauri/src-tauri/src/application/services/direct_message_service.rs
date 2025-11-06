#[cfg(test)]
use crate::application::ports::messaging_gateway::MessagingSendResult;
use crate::application::ports::repositories::{
    DirectMessageCursor, DirectMessageListDirection, DirectMessagePageRaw, DirectMessageRepository,
};
use crate::application::ports::{
    direct_message_notifier::DirectMessageNotifier, messaging_gateway::MessagingGateway,
};
use crate::domain::entities::{DirectMessage, MessageDirection, NewDirectMessage};
use crate::shared::{AppError, ValidationFailureKind};
use chrono::{DateTime, TimeZone, Utc};
use std::sync::Arc;
use tracing::{debug, error};

pub struct DirectMessageService {
    repository: Arc<dyn DirectMessageRepository>,
    messaging_gateway: Arc<dyn MessagingGateway>,
    notifier: Option<Arc<dyn DirectMessageNotifier>>,
}

#[derive(Debug)]
pub struct SendDirectMessageResult {
    pub event_id: Option<String>,
    pub queued: bool,
    pub message: DirectMessage,
}

#[derive(Debug)]
pub struct DirectMessagePageResult {
    pub items: Vec<DirectMessage>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MessagePageDirection {
    #[default]
    Backward,
    Forward,
}

impl From<MessagePageDirection> for DirectMessageListDirection {
    fn from(value: MessagePageDirection) -> Self {
        match value {
            MessagePageDirection::Backward => DirectMessageListDirection::Backward,
            MessagePageDirection::Forward => DirectMessageListDirection::Forward,
        }
    }
}

impl DirectMessageService {
    pub fn new(
        repository: Arc<dyn DirectMessageRepository>,
        messaging_gateway: Arc<dyn MessagingGateway>,
        notifier: Option<Arc<dyn DirectMessageNotifier>>,
    ) -> Self {
        Self {
            repository,
            messaging_gateway,
            notifier,
        }
    }

    pub async fn send_direct_message(
        &self,
        owner_npub: &str,
        recipient_npub: &str,
        content: &str,
        client_message_id: Option<String>,
    ) -> Result<SendDirectMessageResult, AppError> {
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return Err(AppError::validation(
                ValidationFailureKind::Generic,
                "Direct message content must not be empty",
            ));
        }

        let messaging_result = self
            .messaging_gateway
            .encrypt_and_send(owner_npub, recipient_npub, trimmed)
            .await?;

        let created_at =
            millis_to_datetime(messaging_result.created_at_millis).unwrap_or_else(Utc::now);

        let generated_client_id = client_message_id
            .filter(|id| !id.trim().is_empty())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let new_message = NewDirectMessage {
            owner_npub: owner_npub.to_string(),
            conversation_npub: recipient_npub.to_string(),
            sender_npub: owner_npub.to_string(),
            recipient_npub: recipient_npub.to_string(),
            event_id: messaging_result.event_id.clone(),
            client_message_id: Some(generated_client_id.clone()),
            payload_cipher_base64: messaging_result.ciphertext.clone(),
            created_at,
            delivered: messaging_result.delivered,
            direction: MessageDirection::Outbound,
        };

        let stored = self
            .repository
            .insert_direct_message(&new_message)
            .await?
            .with_decrypted_content(trimmed.to_string());

        self.dispatch_notification(owner_npub, &stored).await;

        Ok(SendDirectMessageResult {
            event_id: messaging_result.event_id,
            queued: !messaging_result.delivered,
            message: stored,
        })
    }

    pub async fn list_direct_messages(
        &self,
        owner_npub: &str,
        conversation_npub: &str,
        cursor: Option<&str>,
        limit: Option<usize>,
        direction: MessagePageDirection,
    ) -> Result<DirectMessagePageResult, AppError> {
        let limit = limit.unwrap_or(50).clamp(1, 200);
        let parsed_cursor = parse_cursor(cursor)?;

        let raw_page: DirectMessagePageRaw = self
            .repository
            .list_direct_messages(
                owner_npub,
                conversation_npub,
                parsed_cursor,
                limit,
                direction.into(),
            )
            .await?;

        let mut items = Vec::with_capacity(raw_page.items.len());
        for message in raw_page.items {
            let plaintext = self
                .messaging_gateway
                .decrypt_with_counterparty(
                    owner_npub,
                    message.counterparty_npub(),
                    &message.payload_cipher_base64,
                )
                .await?;
            items.push(message.with_decrypted_content(plaintext));
        }

        Ok(DirectMessagePageResult {
            items,
            next_cursor: raw_page.next_cursor,
            has_more: raw_page.has_more,
        })
    }

    pub async fn ingest_incoming_message(
        &self,
        owner_npub: &str,
        sender_npub: &str,
        ciphertext: &str,
        event_id: Option<String>,
        created_at_millis: i64,
    ) -> Result<Option<DirectMessage>, AppError> {
        let plaintext = self
            .messaging_gateway
            .decrypt_with_counterparty(owner_npub, sender_npub, ciphertext)
            .await?;

        let created_at = millis_to_datetime(created_at_millis).unwrap_or_else(chrono::Utc::now);

        let new_message = NewDirectMessage {
            owner_npub: owner_npub.to_string(),
            conversation_npub: sender_npub.to_string(),
            sender_npub: sender_npub.to_string(),
            recipient_npub: owner_npub.to_string(),
            event_id: event_id.clone(),
            client_message_id: None,
            payload_cipher_base64: ciphertext.to_string(),
            created_at,
            delivered: true,
            direction: MessageDirection::Inbound,
        };

        match self.repository.insert_direct_message(&new_message).await {
            Ok(record) => {
                let stored = record.with_decrypted_content(plaintext);
                self.dispatch_notification(owner_npub, &stored).await;
                Ok(Some(stored))
            }
            Err(err) => {
                if is_unique_violation(&err) {
                    debug!(
                        event_id = event_id.as_deref().unwrap_or(""),
                        owner_npub, "Duplicate direct message detected; skipping insertion"
                    );
                    Ok(None)
                } else {
                    Err(err)
                }
            }
        }
    }

    async fn dispatch_notification(&self, owner_npub: &str, message: &DirectMessage) {
        if let Some(notifier) = &self.notifier {
            if let Err(err) = notifier.notify(owner_npub, message).await {
                error!(
                    error = %err,
                    owner_npub,
                    conversation = message.conversation_npub,
                    "Failed to emit direct message notification"
                );
            }
        }
    }
}

fn parse_cursor(cursor: Option<&str>) -> Result<Option<DirectMessageCursor>, AppError> {
    match cursor {
        None => Ok(None),
        Some(raw) => DirectMessageCursor::parse(raw)
            .ok_or_else(|| {
                AppError::validation(
                    ValidationFailureKind::Generic,
                    format!("Invalid cursor format: {raw}"),
                )
            })
            .map(Some),
    }
}

fn millis_to_datetime(millis: i64) -> Option<DateTime<Utc>> {
    Utc.timestamp_millis_opt(millis).single()
}

fn is_unique_violation(error: &AppError) -> bool {
    match error {
        AppError::Database(message) => {
            message.contains("UNIQUE constraint failed: direct_messages.owner_npub, event_id")
                || message.contains(
                    "UNIQUE constraint failed: direct_messages.owner_npub, client_message_id",
                )
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests;
