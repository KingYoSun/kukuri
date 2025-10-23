use crate::application::shared::nostr::to_nostr_event;
use crate::domain::entities::Event;
use crate::domain::entities::event_gateway::DomainEvent;
use crate::shared::error::AppError;

pub(crate) fn domain_event_from_event(event: &Event) -> Result<DomainEvent, AppError> {
    DomainEvent::try_from(event)
        .map_err(|err| AppError::ValidationError(format!("Invalid domain event: {err}")))
}

pub(crate) fn domain_event_to_nostr_event(
    domain_event: &DomainEvent,
) -> Result<nostr_sdk::Event, AppError> {
    to_nostr_event(&domain_event.to_event())
}
