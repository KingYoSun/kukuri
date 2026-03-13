use crate::domain::entities::{Event, EventKind};
use crate::shared::error::AppError;
use nostr_sdk::JsonUtil;
use serde_json::json;

pub fn build_deletion_event(id: &str, pubkey: String) -> Event {
    let mut deletion_event = Event::new(EventKind::EventDeletion.as_u32(), String::new(), pubkey);
    deletion_event.add_e_tag(id.to_string());
    deletion_event
}

pub fn to_nostr_event(event: &Event) -> Result<nostr_sdk::Event, AppError> {
    let event_json = json!({
        "id": event.id,
        "pubkey": event.pubkey,
        "created_at": event.created_at.timestamp(),
        "kind": event.kind,
        "tags": event.tags,
        "content": event.content,
        "sig": event.sig,
    });

    nostr_sdk::Event::from_json(event_json.to_string())
        .map_err(|e| AppError::NostrError(format!("Failed to convert event: {e}")))
}
