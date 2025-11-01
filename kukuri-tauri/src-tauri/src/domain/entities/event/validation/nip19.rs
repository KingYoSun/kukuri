use super::errors::{EventValidationError, ValidationResult};
use super::utils::is_ws_url;
use crate::shared::validation::ValidationFailureKind;
use bech32::{self, FromBase32 as _};

pub(super) const MAX_TLV_RELAY_URLS: usize = 16;
pub(super) const MAX_TLV_RELAY_URL_LEN: usize = 255;

pub(super) fn validate_nprofile_tlv(s: &str) -> ValidationResult<()> {
    let (hrp, data, _) = bech32::decode(s).map_err(|_| {
        EventValidationError::new(
            ValidationFailureKind::Nip19Encoding,
            "invalid nprofile bech32 encoding",
        )
    })?;
    if hrp != "nprofile" {
        return Err(EventValidationError::new(
            ValidationFailureKind::Nip19Encoding,
            format!("unexpected nprofile hrp: {hrp}"),
        ));
    }
    let bytes = Vec::<u8>::from_base32(&data).map_err(|_| {
        EventValidationError::new(
            ValidationFailureKind::Nip19Encoding,
            "invalid nprofile bech32 payload",
        )
    })?;
    let mut has_pubkey = false;
    let mut relay_count = 0usize;
    parse_tlv(&bytes, |tag, value| match tag {
        0 => {
            if has_pubkey || value.len() != 32 {
                Err(EventValidationError::new(
                    ValidationFailureKind::Nip19Tlv,
                    "nprofile tag 0 must appear exactly once with 32 bytes",
                ))
            } else {
                has_pubkey = true;
                Ok(())
            }
        }
        1 => {
            relay_count += 1;
            if relay_count > MAX_TLV_RELAY_URLS {
                return Err(EventValidationError::new(
                    ValidationFailureKind::Nip19Tlv,
                    format!("nprofile relay entries exceed max {MAX_TLV_RELAY_URLS}"),
                ));
            }
            validate_tlv_relay(value)
        }
        _ => Ok(()),
    })?;
    if !has_pubkey {
        return Err(EventValidationError::new(
            ValidationFailureKind::Nip19Tlv,
            "nprofile missing tag 0 (pubkey)",
        ));
    }
    Ok(())
}

pub(super) fn validate_nevent_tlv(s: &str) -> ValidationResult<()> {
    let (hrp, data, _) = bech32::decode(s).map_err(|_| {
        EventValidationError::new(
            ValidationFailureKind::Nip19Encoding,
            "invalid nevent bech32 encoding",
        )
    })?;
    if hrp != "nevent" {
        return Err(EventValidationError::new(
            ValidationFailureKind::Nip19Encoding,
            format!("unexpected nevent hrp: {hrp}"),
        ));
    }
    let bytes = Vec::<u8>::from_base32(&data).map_err(|_| {
        EventValidationError::new(
            ValidationFailureKind::Nip19Encoding,
            "invalid nevent bech32 payload",
        )
    })?;
    let mut has_event_id = false;
    let mut has_author = false;
    let mut has_kind = false;
    let mut relay_count = 0usize;
    parse_tlv(&bytes, |tag, value| match tag {
        0 => {
            if has_event_id || value.len() != 32 {
                Err(EventValidationError::new(
                    ValidationFailureKind::Nip19Tlv,
                    "nevent tag 0 must appear exactly once with 32 bytes",
                ))
            } else {
                has_event_id = true;
                Ok(())
            }
        }
        1 => {
            relay_count += 1;
            if relay_count > MAX_TLV_RELAY_URLS {
                return Err(EventValidationError::new(
                    ValidationFailureKind::Nip19Tlv,
                    format!("nevent relay entries exceed max {MAX_TLV_RELAY_URLS}"),
                ));
            }
            validate_tlv_relay(value)
        }
        2 => {
            if has_author || value.len() != 32 {
                Err(EventValidationError::new(
                    ValidationFailureKind::Nip19Tlv,
                    "nevent tag 2 (author) must be 32 bytes and appear at most once",
                ))
            } else {
                has_author = true;
                Ok(())
            }
        }
        3 => {
            if has_kind || value.len() != 4 {
                Err(EventValidationError::new(
                    ValidationFailureKind::Nip19Tlv,
                    "nevent tag 3 (kind) must be 4 bytes and appear at most once",
                ))
            } else {
                has_kind = true;
                Ok(())
            }
        }
        _ => Ok(()),
    })?;
    if !has_event_id {
        return Err(EventValidationError::new(
            ValidationFailureKind::Nip19Tlv,
            "nevent missing tag 0 (event id)",
        ));
    }
    Ok(())
}

fn parse_tlv(
    bytes: &[u8],
    mut handler: impl FnMut(u8, &[u8]) -> ValidationResult<()>,
) -> ValidationResult<()> {
    let mut i = 0usize;
    while i + 2 <= bytes.len() {
        let tag = bytes[i];
        let len = bytes[i + 1] as usize;
        i += 2;
        if i + len > bytes.len() {
            return Err(EventValidationError::new(
                ValidationFailureKind::Nip19Tlv,
                "TLV value length exceeds buffer",
            ));
        }
        let value = &bytes[i..i + len];
        handler(tag, value)?;
        i += len;
    }
    if i == bytes.len() {
        Ok(())
    } else {
        Err(EventValidationError::new(
            ValidationFailureKind::Nip19Tlv,
            "TLV parse ended with trailing bytes",
        ))
    }
}

fn validate_tlv_relay(value: &[u8]) -> ValidationResult<()> {
    if value.len() > MAX_TLV_RELAY_URL_LEN {
        return Err(EventValidationError::new(
            ValidationFailureKind::Nip19Tlv,
            format!("relay url exceeds {MAX_TLV_RELAY_URL_LEN} bytes"),
        ));
    }
    if value.is_empty() {
        return Ok(());
    }
    match std::str::from_utf8(value) {
        Ok(url) => {
            if url.is_ascii() && is_ws_url(url) {
                Ok(())
            } else {
                Err(EventValidationError::new(
                    ValidationFailureKind::Nip19Tlv,
                    format!("relay url must be ws[s]:// and ASCII: {url}"),
                ))
            }
        }
        Err(_) => Err(EventValidationError::new(
            ValidationFailureKind::Nip19Tlv,
            "relay url must be valid UTF-8",
        )),
    }
}
