use super::errors::{EventValidationError, MAX_EVENT_CONTENT_BYTES, ValidationResult};
use crate::domain::entities::event::Event;
use crate::shared::validation::ValidationFailureKind;
use semver::Version;
use serde::Deserialize;

pub const KIND30078_KIND: u32 = 30_078;
pub const KIND30078_MAX_ATTACHMENTS: usize = 16;

#[derive(Debug, Deserialize)]
struct Kind30078Content {
    body: String,
    #[serde(default)]
    attachments: Vec<String>,
    metadata: Kind30078Metadata,
}

#[derive(Debug, Deserialize)]
struct Kind30078Metadata {
    app_version: String,
    edited: bool,
}

impl Event {
    /// kind:30078 (kukuri topic post PRE) のタグ・content検証
    pub fn validate_kind30078(&self) -> ValidationResult<()> {
        if self.kind != KIND30078_KIND {
            return Err(EventValidationError::new(
                ValidationFailureKind::UnsupportedKind,
                format!(
                    "kind mismatch for kind30078 (expected {}, got {})",
                    KIND30078_KIND, self.kind
                ),
            ));
        }

        let mut identifier: Option<(String, String)> = None;
        let mut has_k = false;
        let mut topic_tag: Option<String> = None;
        let mut address_tag: Option<String> = None;

        for tag in &self.tags {
            if tag.is_empty() {
                continue;
            }
            match tag[0].as_str() {
                "d" => {
                    if identifier.is_some() {
                        return Err(EventValidationError::new(
                            ValidationFailureKind::Kind30078TagMismatch,
                            "multiple d tags detected for kind30078",
                        ));
                    }
                    if tag.len() != 2 {
                        return Err(EventValidationError::new(
                            ValidationFailureKind::Kind30078TagMismatch,
                            format!("d tag must be [\"d\", <identifier>] (len={})", tag.len()),
                        ));
                    }
                    let (slug, revision) = parse_kind30078_identifier(&tag[1])?;
                    ensure_slug_valid(&slug)?;
                    ensure_revision_valid(&revision)?;
                    identifier = Some((slug, revision));
                }
                "k" => {
                    if has_k {
                        return Err(EventValidationError::new(
                            ValidationFailureKind::Kind30078TagMismatch,
                            "multiple k tags detected for kind30078",
                        ));
                    }
                    if tag.len() != 2 {
                        return Err(EventValidationError::new(
                            ValidationFailureKind::Kind30078TagMismatch,
                            format!("k tag must be [\"k\", \"topic-post\"] (len={})", tag.len()),
                        ));
                    }
                    if tag[1] != "topic-post" {
                        return Err(EventValidationError::new(
                            ValidationFailureKind::Kind30078TagMismatch,
                            format!("k tag must equal \"topic-post\" (value={})", tag[1]),
                        ));
                    }
                    has_k = true;
                }
                "t" => {
                    if tag.len() != 2 {
                        return Err(EventValidationError::new(
                            ValidationFailureKind::Kind30078TagMismatch,
                            format!(
                                "t tag must be [\"t\", \"topic:<slug>\"] (len={})",
                                tag.len()
                            ),
                        ));
                    }
                    topic_tag = Some(tag[1].clone());
                }
                "a" => {
                    if tag.len() != 2 {
                        return Err(EventValidationError::new(
                            ValidationFailureKind::Kind30078TagMismatch,
                            format!("a tag must be [\"a\", <address>] (len={})", tag.len()),
                        ));
                    }
                    address_tag = Some(tag[1].clone());
                }
                _ => {}
            }
        }

        let (slug, revision) = identifier.ok_or_else(|| {
            EventValidationError::new(
                ValidationFailureKind::Kind30078TagMissing,
                "missing d tag for kind30078 event",
            )
        })?;

        if !has_k {
            return Err(EventValidationError::new(
                ValidationFailureKind::Kind30078TagMissing,
                "missing k tag for kind30078 event",
            ));
        }

        let topic_value = topic_tag.ok_or_else(|| {
            EventValidationError::new(
                ValidationFailureKind::Kind30078TagMissing,
                "missing t tag for kind30078 event",
            )
        })?;

        let expected_topic = format!("topic:{slug}");
        if topic_value != expected_topic {
            return Err(EventValidationError::new(
                ValidationFailureKind::Kind30078TagMismatch,
                format!("t tag must equal \"{expected_topic}\" (value={topic_value})"),
            ));
        }

        let address_value = address_tag.ok_or_else(|| {
            EventValidationError::new(
                ValidationFailureKind::Kind30078TagMissing,
                "missing a tag for kind30078 event",
            )
        })?;

        let expected_address = format!(
            "{}:{}:kukuri:topic:{}:post:{}",
            KIND30078_KIND, self.pubkey, slug, revision
        );
        if address_value != expected_address {
            return Err(EventValidationError::new(
                ValidationFailureKind::Kind30078TagMismatch,
                format!("a tag must equal \"{expected_address}\" (value={address_value})"),
            ));
        }

        self.validate_kind30078_content()?;

        Ok(())
    }

    fn validate_kind30078_content(&self) -> ValidationResult<()> {
        let parsed: Kind30078Content = serde_json::from_str(&self.content).map_err(|err| {
            EventValidationError::new(
                ValidationFailureKind::Kind30078ContentSchema,
                format!("content must be valid JSON object: {err}"),
            )
        })?;

        if parsed.body.len() > MAX_EVENT_CONTENT_BYTES {
            return Err(EventValidationError::new(
                ValidationFailureKind::Kind30078ContentSize,
                format!("body exceeds {MAX_EVENT_CONTENT_BYTES} bytes"),
            ));
        }

        if parsed.attachments.len() > KIND30078_MAX_ATTACHMENTS {
            return Err(EventValidationError::new(
                ValidationFailureKind::Kind30078ContentSize,
                format!("attachments exceed max {KIND30078_MAX_ATTACHMENTS}"),
            ));
        }

        for (idx, attachment) in parsed.attachments.iter().enumerate() {
            if attachment.is_empty() {
                return Err(EventValidationError::new(
                    ValidationFailureKind::Kind30078ContentSchema,
                    format!("attachment[{idx}] must not be empty"),
                ));
            }
            if !attachment.is_ascii() {
                return Err(EventValidationError::new(
                    ValidationFailureKind::Kind30078ContentSchema,
                    format!("attachment[{idx}] must be ASCII"),
                ));
            }
            let lower = attachment.to_ascii_lowercase();
            if !(lower.starts_with("iroh://") || lower.starts_with("https://")) {
                return Err(EventValidationError::new(
                    ValidationFailureKind::Kind30078ContentSchema,
                    format!(
                        "attachment[{idx}] must start with iroh:// or https:// (value={attachment})"
                    ),
                ));
            }
        }

        Version::parse(&parsed.metadata.app_version).map_err(|err| {
            EventValidationError::new(
                ValidationFailureKind::Kind30078ContentSchema,
                format!("metadata.app_version must be semantic version: {err}"),
            )
        })?;

        Ok(())
    }
}

fn parse_kind30078_identifier(value: &str) -> ValidationResult<(String, String)> {
    let rest = value.strip_prefix("kukuri:topic:").ok_or_else(|| {
        EventValidationError::new(
            ValidationFailureKind::Kind30078TagMismatch,
            format!("d tag must start with kukuri:topic: (value={value})"),
        )
    })?;
    let (slug, revision) = rest.split_once(":post:").ok_or_else(|| {
        EventValidationError::new(
            ValidationFailureKind::Kind30078TagMismatch,
            format!("d tag must contain :post: separator (value={value})"),
        )
    })?;
    Ok((slug.to_string(), revision.to_string()))
}

fn ensure_slug_valid(slug: &str) -> ValidationResult<()> {
    if slug.is_empty() || slug.len() > 48 {
        return Err(EventValidationError::new(
            ValidationFailureKind::Kind30078TagMismatch,
            format!("slug must be 1..48 characters (slug={slug})"),
        ));
    }
    if !slug
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(EventValidationError::new(
            ValidationFailureKind::Kind30078TagMismatch,
            format!("slug contains invalid characters: {slug}"),
        ));
    }
    Ok(())
}

fn ensure_revision_valid(revision: &str) -> ValidationResult<()> {
    if revision.len() == 26 && is_crockford_base32(revision) {
        return Ok(());
    }
    if revision.len() == 32 && revision.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(());
    }
    Err(EventValidationError::new(
        ValidationFailureKind::Kind30078TagMismatch,
        format!("invalid revision identifier: {revision}"),
    ))
}

fn is_crockford_base32(value: &str) -> bool {
    value.chars().all(|c| {
        let up = c.to_ascii_uppercase();
        matches!(
            up,
            '0'..='9'
                | 'A'..='H'
                | 'J'..='K'
                | 'M'..='N'
                | 'P'..='T'
                | 'V'..='Z'
        )
    })
}
