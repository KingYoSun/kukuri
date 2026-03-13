mod errors;
mod kind30078;
mod nip01;
mod nip10_19;
mod nip19;
mod utils;

pub use errors::{EventValidationError, ValidationResult};
pub use kind30078::{KIND30078_KIND, KIND30078_MAX_ATTACHMENTS};

use super::Event;

impl Event {
    pub fn validate_for_gateway(&self) -> ValidationResult<()> {
        self.validate_nip01()?;
        self.validate_nip10_19()?;
        if self.kind == KIND30078_KIND {
            self.validate_kind30078()?;
        }
        Ok(())
    }

    pub fn validate_nprofile_tlv(s: &str) -> ValidationResult<()> {
        nip19::validate_nprofile_tlv(s)
    }

    pub fn validate_nevent_tlv(s: &str) -> ValidationResult<()> {
        nip19::validate_nevent_tlv(s)
    }
}
