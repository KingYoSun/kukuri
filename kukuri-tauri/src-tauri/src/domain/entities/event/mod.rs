mod kind;
mod model;
pub mod validation;

pub use kind::EventKind;
pub use model::Event;
pub use validation::{
    EventValidationError, KIND30078_KIND, KIND30078_MAX_ATTACHMENTS, ValidationResult,
};

#[cfg(test)]
mod tests;
