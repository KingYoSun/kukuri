#[cfg(test)]
use crate::application::ports::messaging_gateway::MessagingSendResult;
use crate::application::ports::repositories::{
    DirectMessageConversationCursor, DirectMessageConversationPageRaw,
    DirectMessageConversationRecord, DirectMessageCursor, DirectMessageListDirection,
    DirectMessagePageRaw, DirectMessageRepository,
};
use crate::application::ports::{
    direct_message_notifier::DirectMessageNotifier, messaging_gateway::MessagingGateway,
};
use crate::domain::entities::{DirectMessage, MessageDirection, NewDirectMessage};
use crate::shared::{AppError, ValidationFailureKind};
use chrono::{DateTime, TimeZone, Utc};
use nostr_sdk::prelude::nip04;
use nostr_sdk::prelude::{FromBech32, Keys, PublicKey, SecretKey, ToBech32};
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

#[derive(Debug)]
pub struct DirectMessageConversationSummary {
    pub conversation_npub: String,
    pub unread_count: u64,
    pub last_read_at: i64,
    pub last_message: Option<DirectMessage>,
}

#[derive(Debug)]
pub struct DirectMessageConversationPageResult {
    pub items: Vec<DirectMessageConversationSummary>,
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

        self.persist_conversation_snapshot(owner_npub, &stored)
            .await?;
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

    pub async fn list_direct_message_conversations(
        &self,
        owner_npub: &str,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> Result<DirectMessageConversationPageResult, AppError> {
        let limit = limit.unwrap_or(50).clamp(1, 200);
        let parsed_cursor = parse_conversation_cursor(cursor)?;
        let page: DirectMessageConversationPageRaw = self
            .repository
            .list_direct_message_conversations(owner_npub, parsed_cursor, limit)
            .await?;

        let mut summaries = Vec::with_capacity(page.items.len());
        for record in page.items {
            let DirectMessageConversationRecord {
                conversation_npub,
                last_message,
                last_read_at,
                unread_count,
                ..
            } = record;

            let decrypted = if let Some(message) = last_message {
                let plaintext = self
                    .messaging_gateway
                    .decrypt_with_counterparty(
                        owner_npub,
                        message.counterparty_npub(),
                        &message.payload_cipher_base64,
                    )
                    .await?;
                Some(message.with_decrypted_content(plaintext))
            } else {
                None
            };

            summaries.push(DirectMessageConversationSummary {
                conversation_npub,
                unread_count: unread_count.max(0) as u64,
                last_read_at,
                last_message: decrypted,
            });
        }

        Ok(DirectMessageConversationPageResult {
            items: summaries,
            next_cursor: page.next_cursor,
            has_more: page.has_more,
        })
    }

    pub async fn mark_conversation_as_read(
        &self,
        owner_npub: &str,
        conversation_npub: &str,
        read_at_millis: i64,
    ) -> Result<(), AppError> {
        let normalized = read_at_millis.max(0);
        self.repository
            .mark_conversation_as_read(owner_npub, conversation_npub, normalized)
            .await
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
                self.persist_conversation_snapshot(owner_npub, &stored)
                    .await?;
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

    pub async fn seed_incoming_message_for_e2e(
        &self,
        owner_npub: &str,
        content: &str,
        created_at_millis: Option<i64>,
        owner_nsec: Option<&str>,
    ) -> Result<DirectMessage, AppError> {
        let owner_keys = if let Some(nsec) = owner_nsec {
            let secret_key =
                SecretKey::from_bech32(nsec).map_err(|err| AppError::ValidationError {
                    kind: ValidationFailureKind::Generic,
                    message: format!("Invalid owner nsec for seeded direct message: {err}"),
                })?;
            Some(Keys::new(secret_key))
        } else {
            None
        };

        let owner_pk = match &owner_keys {
            Some(keys) => keys.public_key(),
            None => {
                PublicKey::from_bech32(owner_npub).map_err(|err| AppError::ValidationError {
                    kind: ValidationFailureKind::Generic,
                    message: format!("Invalid owner npub {owner_npub}: {err}"),
                })?
            }
        };

        let sender_keys = Keys::generate();
        let sender_pk = sender_keys.public_key();
        let ciphertext =
            nip04::encrypt(sender_keys.secret_key(), &owner_pk, content).map_err(|err| {
                AppError::Crypto(format!("Failed to encrypt seeded direct message: {err}"))
            })?;
        let sender_npub = sender_pk
            .to_bech32()
            .map_err(|err| AppError::Crypto(format!("Failed to encode seeded npub: {err}")))?;
        let created_at = created_at_millis.unwrap_or_else(|| Utc::now().timestamp_millis());

        if let Some(keys) = owner_keys {
            let plaintext =
                nip04::decrypt(keys.secret_key(), &sender_pk, &ciphertext).map_err(|err| {
                    AppError::Crypto(format!(
                        "Failed to decrypt seeded direct message with provided nsec: {err}"
                    ))
                })?;

            let new_message = NewDirectMessage {
                owner_npub: owner_npub.to_string(),
                conversation_npub: sender_npub.clone(),
                sender_npub: sender_npub.clone(),
                recipient_npub: owner_npub.to_string(),
                event_id: None,
                client_message_id: None,
                payload_cipher_base64: ciphertext.clone(),
                created_at: millis_to_datetime(created_at).unwrap_or_else(chrono::Utc::now),
                delivered: true,
                direction: MessageDirection::Inbound,
            };

            match self.repository.insert_direct_message(&new_message).await {
                Ok(record) => {
                    let stored = record.with_decrypted_content(plaintext);
                    self.persist_conversation_snapshot(owner_npub, &stored)
                        .await?;
                    self.dispatch_notification(owner_npub, &stored).await;
                    Ok(stored)
                }
                Err(err) => {
                    if is_unique_violation(&err) {
                        debug!(
                            owner_npub,
                            conversation = sender_npub,
                            "Duplicate seeded direct message detected; skipping insertion"
                        );
                        Err(AppError::Internal(
                            "Failed to persist seeded direct message conversation".into(),
                        ))
                    } else {
                        Err(err)
                    }
                }
            }
        } else {
            self.ingest_incoming_message(owner_npub, &sender_npub, &ciphertext, None, created_at)
                .await?
                .ok_or_else(|| {
                    AppError::Internal(
                        "Failed to persist seeded direct message conversation".into(),
                    )
                })
        }
    }

    async fn dispatch_notification(&self, owner_npub: &str, message: &DirectMessage) {
        if let Some(notifier) = &self.notifier
            && let Err(err) = notifier.notify(owner_npub, message).await
        {
            error!(
                error = %err,
                owner_npub,
                conversation = message.conversation_npub,
                "Failed to emit direct message notification"
            );
        }
    }

    async fn persist_conversation_snapshot(
        &self,
        owner_npub: &str,
        message: &DirectMessage,
    ) -> Result<(), AppError> {
        self.repository
            .upsert_conversation_metadata(
                owner_npub,
                &message.conversation_npub,
                message.id,
                message.created_at_millis(),
            )
            .await
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

fn parse_conversation_cursor(
    cursor: Option<&str>,
) -> Result<Option<DirectMessageConversationCursor>, AppError> {
    match cursor {
        None => Ok(None),
        Some(raw) => DirectMessageConversationCursor::parse(raw)
            .ok_or_else(|| {
                AppError::validation(
                    ValidationFailureKind::Generic,
                    format!("Invalid conversation cursor format: {raw}"),
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
