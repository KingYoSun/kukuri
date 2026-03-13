use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageDirection {
    Outbound,
    Inbound,
}

impl MessageDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageDirection::Outbound => "outbound",
            MessageDirection::Inbound => "inbound",
        }
    }
}

impl FromStr for MessageDirection {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "outbound" => Ok(MessageDirection::Outbound),
            "inbound" => Ok(MessageDirection::Inbound),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NewDirectMessage {
    pub owner_npub: String,
    pub conversation_npub: String,
    pub sender_npub: String,
    pub recipient_npub: String,
    pub event_id: Option<String>,
    pub client_message_id: Option<String>,
    pub payload_cipher_base64: String,
    pub created_at: DateTime<Utc>,
    pub delivered: bool,
    pub direction: MessageDirection,
}

#[derive(Debug, Clone)]
pub struct DirectMessage {
    pub id: i64,
    pub owner_npub: String,
    pub conversation_npub: String,
    pub sender_npub: String,
    pub recipient_npub: String,
    pub event_id: Option<String>,
    pub client_message_id: Option<String>,
    pub payload_cipher_base64: String,
    pub created_at: DateTime<Utc>,
    pub delivered: bool,
    pub direction: MessageDirection,
    pub decrypted_content: Option<String>,
}

impl DirectMessage {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: i64,
        owner_npub: String,
        conversation_npub: String,
        sender_npub: String,
        recipient_npub: String,
        event_id: Option<String>,
        client_message_id: Option<String>,
        payload_cipher_base64: String,
        created_at_millis: i64,
        delivered: bool,
        direction: MessageDirection,
    ) -> Self {
        let created_at = match Utc.timestamp_millis_opt(created_at_millis) {
            chrono::LocalResult::Single(ts) => ts,
            _ => Utc
                .timestamp_millis_opt(0)
                .single()
                .unwrap_or_else(Utc::now),
        };

        Self {
            id,
            owner_npub,
            conversation_npub,
            sender_npub,
            recipient_npub,
            event_id,
            client_message_id,
            payload_cipher_base64,
            created_at,
            delivered,
            direction,
            decrypted_content: None,
        }
    }

    pub fn with_decrypted_content(mut self, content: String) -> Self {
        self.decrypted_content = Some(content);
        self
    }

    pub fn mark_delivered(mut self, delivered: bool) -> Self {
        self.delivered = delivered;
        self
    }

    pub fn cursor(&self) -> String {
        let event_part = self.event_id.clone().unwrap_or_default();
        format!("{}:{}", self.created_at.timestamp_millis(), event_part)
    }

    pub fn created_at_millis(&self) -> i64 {
        self.created_at.timestamp_millis()
    }

    pub fn counterparty_npub(&self) -> &str {
        &self.conversation_npub
    }
}
