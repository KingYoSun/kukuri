use crate::domain::value_objects::EventId;
use crate::shared::{AppError, ValidationFailureKind};

pub(crate) fn parse_event_id(hex: &str) -> Result<EventId, AppError> {
    EventId::from_hex(hex).map_err(|err| {
        AppError::validation(
            ValidationFailureKind::Generic,
            format!("Invalid event ID: {err}"),
        )
    })
}

pub(crate) fn parse_optional_event_id(hex: Option<&str>) -> Result<Option<EventId>, AppError> {
    match hex {
        Some(value) => parse_event_id(value).map(Some),
        None => Ok(None),
    }
}

pub(crate) fn parse_event_ids(hexes: &[String]) -> Result<Vec<EventId>, AppError> {
    hexes.iter().map(|value| parse_event_id(value)).collect()
}
