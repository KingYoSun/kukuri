use crate::application::ports::direct_message_notifier::DirectMessageNotifier;
use crate::domain::entities::DirectMessage;
use crate::shared::AppError;
use async_trait::async_trait;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Clone)]
pub struct IpcDirectMessageNotifier {
    app_handle: AppHandle,
}

impl IpcDirectMessageNotifier {
    pub fn new(app_handle: &AppHandle) -> Self {
        Self {
            app_handle: app_handle.clone(),
        }
    }
}

#[derive(Serialize, Clone)]
struct DirectMessagePayload {
    event_id: Option<String>,
    client_message_id: Option<String>,
    sender_npub: String,
    recipient_npub: String,
    content: String,
    created_at: i64,
    delivered: bool,
    direction: &'static str,
}

#[derive(Serialize, Clone)]
struct DirectMessageEventPayload {
    owner_npub: String,
    conversation_npub: String,
    message: DirectMessagePayload,
}

#[async_trait]
impl DirectMessageNotifier for IpcDirectMessageNotifier {
    async fn notify(&self, owner_npub: &str, message: &DirectMessage) -> Result<(), AppError> {
        let payload = DirectMessageEventPayload {
            owner_npub: owner_npub.to_string(),
            conversation_npub: message.conversation_npub.clone(),
            message: DirectMessagePayload {
                event_id: message.event_id.clone(),
                client_message_id: message.client_message_id.clone(),
                sender_npub: message.sender_npub.clone(),
                recipient_npub: message.recipient_npub.clone(),
                content: message.decrypted_content.clone().unwrap_or_default(),
                created_at: message.created_at_millis(),
                delivered: message.delivered,
                direction: message.direction.as_str(),
            },
        };

        self.app_handle
            .emit("direct-message:received", payload)
            .map_err(|err| {
                AppError::Internal(format!("Failed to emit direct message event: {err}"))
            })
    }
}
