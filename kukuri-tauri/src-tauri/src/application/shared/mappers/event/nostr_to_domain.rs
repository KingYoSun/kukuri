use crate::application::shared::nostr::to_nostr_event;
use crate::domain::entities::Event;
use crate::domain::entities::event_gateway::DomainEvent;
use crate::domain::value_objects::EventId;
use crate::shared::error::AppError;
use chrono::{DateTime, Utc};
use nostr_sdk::prelude::Event as NostrEvent;

pub(crate) fn domain_event_from_event(event: &Event) -> Result<DomainEvent, AppError> {
    DomainEvent::try_from(event)
        .map_err(|err| AppError::ValidationError(format!("Invalid domain event: {err}")))
}

pub(crate) fn domain_event_to_nostr_event(
    domain_event: &DomainEvent,
) -> Result<nostr_sdk::Event, AppError> {
    to_nostr_event(&domain_event.to_event())
}

pub(crate) fn nostr_event_to_domain_event(event: &NostrEvent) -> Result<Event, AppError> {
    let id = EventId::from_hex(&event.id.to_string()).map_err(|err| {
        AppError::ValidationError(format!("Invalid event ID received from gateway: {err}"))
    })?;

    let created_at = DateTime::<Utc>::from_timestamp(event.created_at.as_u64() as i64, 0)
        .ok_or_else(|| AppError::ValidationError("Invalid event timestamp".to_string()))?;

    let tags = event.tags.iter().map(|tag| tag.clone().to_vec()).collect();

    Ok(Event::new_with_id(
        id,
        event.pubkey.to_string(),
        event.content.clone(),
        event.kind.as_u16() as u32,
        tags,
        created_at,
        event.sig.to_string(),
    ))
}
