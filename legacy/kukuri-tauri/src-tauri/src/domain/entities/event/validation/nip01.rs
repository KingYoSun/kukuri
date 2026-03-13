use super::errors::{
    EventValidationError, MAX_EVENT_CONTENT_BYTES, MAX_EVENT_TAGS, TIMESTAMP_DRIFT_SECS,
    ValidationResult,
};
use super::utils::is_hex_n;
use crate::domain::entities::event::Event;
use crate::shared::validation::ValidationFailureKind;
use chrono::Utc;
use sha2::{Digest, Sha256};

impl Event {
    /// NIP-01に基づく基本バリデーション
    /// - idは[0,pubkey,created_at,kind,tags,content]のsha256
    /// - pubkeyは32byte hex（64桁）
    /// - sigは64byte hex（128桁）
    pub fn validate_nip01(&self) -> ValidationResult<()> {
        if !is_hex_n(&self.pubkey, 64) {
            return Err(EventValidationError::new(
                ValidationFailureKind::Nip01Integrity,
                "invalid pubkey (expect 64 hex)",
            ));
        }
        if !is_hex_n(&self.sig, 128) {
            return Err(EventValidationError::new(
                ValidationFailureKind::Nip01Integrity,
                "invalid sig (expect 128 hex)",
            ));
        }
        if !is_hex_n(&self.id, 64) {
            return Err(EventValidationError::new(
                ValidationFailureKind::Nip01Integrity,
                "invalid id (expect 64 hex)",
            ));
        }

        let created_at_secs = self.created_at.timestamp();
        let payload = serde_json::json!([
            0,
            self.pubkey,
            created_at_secs,
            self.kind,
            self.tags,
            self.content,
        ]);
        let serialized = serde_json::to_vec(&payload).map_err(|e| {
            EventValidationError::new(
                ValidationFailureKind::Nip01Integrity,
                format!("serialization error: {e}"),
            )
        })?;
        let hash = Sha256::digest(&serialized);
        let calc_id = format!("{hash:x}");
        if calc_id != self.id {
            return Err(EventValidationError::new(
                ValidationFailureKind::Nip01Integrity,
                "id mismatch (not NIP-01 compliant)",
            ));
        }

        let drift_secs = self
            .created_at
            .signed_duration_since(Utc::now())
            .num_seconds()
            .abs();
        if drift_secs > TIMESTAMP_DRIFT_SECS {
            return Err(EventValidationError::new(
                ValidationFailureKind::TimestampOutOfRange,
                format!("created_at outside ±{TIMESTAMP_DRIFT_SECS}s window (drift={drift_secs}s)"),
            ));
        }

        if self.tags.len() > MAX_EVENT_TAGS {
            return Err(EventValidationError::new(
                ValidationFailureKind::TagLimitExceeded,
                format!("too many tags: {} (max {MAX_EVENT_TAGS})", self.tags.len()),
            ));
        }

        if self.content.len() > MAX_EVENT_CONTENT_BYTES {
            return Err(EventValidationError::new(
                ValidationFailureKind::ContentTooLarge,
                format!(
                    "content exceeds {MAX_EVENT_CONTENT_BYTES} bytes (actual {})",
                    self.content.len()
                ),
            ));
        }

        Ok(())
    }
}
