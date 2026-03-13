use crate::shared::validation::ValidationFailureKind;
use std::fmt;

pub type ValidationResult<T> = Result<T, EventValidationError>;

pub(super) const MAX_EVENT_TAGS: usize = 512;
pub(super) const MAX_EVENT_CONTENT_BYTES: usize = 1_048_576;
pub(super) const TIMESTAMP_DRIFT_SECS: i64 = 600;

#[derive(Debug, Clone)]
pub struct EventValidationError {
    pub kind: ValidationFailureKind,
    pub message: String,
}

impl EventValidationError {
    pub fn new(kind: ValidationFailureKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for EventValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl std::error::Error for EventValidationError {}
