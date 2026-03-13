use super::errors::{EventValidationError, ValidationResult};
use super::nip19::{validate_nevent_tlv, validate_nprofile_tlv};
use super::utils::{is_hex_n, is_ws_url};
use crate::domain::entities::event::Event;
use crate::shared::validation::ValidationFailureKind;
use nostr_sdk::prelude::{EventId as NostrEventId, FromBech32 as _, PublicKey as NostrPublicKey};

impl Event {
    /// NIP-10/NIP-19 の返信タグ・bech32/TLVの検証
    pub fn validate_nip10_19(&self) -> ValidationResult<()> {
        let mut root_seen = 0usize;
        let mut reply_seen = 0usize;

        for tag in &self.tags {
            if tag.is_empty() {
                continue;
            }
            match tag[0].as_str() {
                "e" => {
                    if tag.len() < 2 {
                        return Err(EventValidationError::new(
                            ValidationFailureKind::Nip10TagStructure,
                            "invalid e tag (len < 2)",
                        ));
                    }
                    let evref = &tag[1];
                    if !is_hex_n(evref, 64) {
                        ensure_event_ref(evref)?;
                    }
                    if tag.len() >= 3 {
                        let relay_url = tag[2].as_str();
                        if !relay_url.is_empty() && !is_ws_url(relay_url) {
                            return Err(EventValidationError::new(
                                ValidationFailureKind::Nip10TagStructure,
                                format!("invalid e tag relay_url: {relay_url}"),
                            ));
                        }
                    }
                    if tag.len() >= 4 {
                        let marker = tag[3].as_str();
                        match marker {
                            "root" => root_seen += 1,
                            "reply" => reply_seen += 1,
                            "mention" => {}
                            _ => {
                                return Err(EventValidationError::new(
                                    ValidationFailureKind::Nip10TagStructure,
                                    format!("invalid e tag marker: {marker}"),
                                ));
                            }
                        }
                    }
                }
                "p" => {
                    if tag.len() < 2 {
                        return Err(EventValidationError::new(
                            ValidationFailureKind::Nip10TagStructure,
                            "invalid p tag (len < 2)",
                        ));
                    }
                    let pkref = &tag[1];
                    if !is_hex_n(pkref, 64) {
                        ensure_pubkey_ref(pkref)?;
                    }
                    if tag.len() >= 3 {
                        let relay_url = tag[2].as_str();
                        if !relay_url.is_empty() && !is_ws_url(relay_url) {
                            return Err(EventValidationError::new(
                                ValidationFailureKind::Nip10TagStructure,
                                format!("invalid p tag relay_url: {relay_url}"),
                            ));
                        }
                    }
                }
                _ => {}
            }
        }

        if root_seen > 1 {
            return Err(EventValidationError::new(
                ValidationFailureKind::Nip10TagStructure,
                "multiple root markers in e tags",
            ));
        }
        if reply_seen > 1 {
            return Err(EventValidationError::new(
                ValidationFailureKind::Nip10TagStructure,
                "multiple reply markers in e tags",
            ));
        }
        Ok(())
    }
}

fn ensure_event_ref(s: &str) -> ValidationResult<()> {
    if s.starts_with("note1") {
        NostrEventId::from_bech32(s).map_err(|_| {
            EventValidationError::new(
                ValidationFailureKind::Nip19Encoding,
                "invalid note reference bech32",
            )
        })?;
        Ok(())
    } else if s.starts_with("nevent1") {
        validate_nevent_tlv(s)
    } else {
        Err(EventValidationError::new(
            ValidationFailureKind::Nip10TagStructure,
            format!("unsupported e tag reference format: {s}"),
        ))
    }
}

fn ensure_pubkey_ref(s: &str) -> ValidationResult<()> {
    if s.starts_with("npub1") {
        NostrPublicKey::from_bech32(s).map_err(|_| {
            EventValidationError::new(
                ValidationFailureKind::Nip19Encoding,
                "invalid npub reference bech32",
            )
        })?;
        Ok(())
    } else if s.starts_with("nprofile1") {
        validate_nprofile_tlv(s)
    } else {
        Err(EventValidationError::new(
            ValidationFailureKind::Nip10TagStructure,
            format!("unsupported p tag reference format: {s}"),
        ))
    }
}
