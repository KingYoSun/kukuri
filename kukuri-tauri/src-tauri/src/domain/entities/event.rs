use crate::domain::value_objects::EventId;
use crate::shared::validation::ValidationFailureKind;
use bech32::FromBase32 as _;
use chrono::{DateTime, Duration, Utc};
use nostr_sdk::prelude::{EventId as NostrEventId, FromBech32 as _, PublicKey as NostrPublicKey};
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt;
// NIP-19 厳密なTLVデコードはSDKの公開API差異があるため、
// ここではEventId/PublicKeyのbech32復号と形式検証で代替する。

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[repr(u32)]
pub enum EventKind {
    Metadata = 0,
    TextNote = 1,
    RecommendRelay = 2,
    Contacts = 3,
    EncryptedDirectMessage = 4,
    EventDeletion = 5,
    Repost = 6,
    Reaction = 7,
    BadgeAward = 8,
    ChannelCreation = 40,
    ChannelMetadata = 41,
    ChannelMessage = 42,
    ChannelHideMessage = 43,
    ChannelMuteUser = 44,
    Custom(u32),
}

const MAX_EVENT_TAGS: usize = 512;
const MAX_EVENT_CONTENT_BYTES: usize = 1_048_576;
const KIND30078_MAX_ATTACHMENTS: usize = 16;
const KIND30078_KIND: u32 = 30_078;
const TIMESTAMP_DRIFT_SECS: i64 = 600;

#[derive(Debug, Clone)]
pub struct EventValidationError {
    pub kind: ValidationFailureKind,
    pub message: String,
}

type ValidationResult<T> = Result<T, EventValidationError>;

impl EventValidationError {
    fn new(kind: ValidationFailureKind, message: impl Into<String>) -> Self {
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

impl From<u32> for EventKind {
    fn from(value: u32) -> Self {
        match value {
            0 => EventKind::Metadata,
            1 => EventKind::TextNote,
            2 => EventKind::RecommendRelay,
            3 => EventKind::Contacts,
            4 => EventKind::EncryptedDirectMessage,
            5 => EventKind::EventDeletion,
            6 => EventKind::Repost,
            7 => EventKind::Reaction,
            8 => EventKind::BadgeAward,
            40 => EventKind::ChannelCreation,
            41 => EventKind::ChannelMetadata,
            42 => EventKind::ChannelMessage,
            43 => EventKind::ChannelHideMessage,
            44 => EventKind::ChannelMuteUser,
            v => EventKind::Custom(v),
        }
    }
}

impl From<EventKind> for u32 {
    fn from(value: EventKind) -> Self {
        match value {
            EventKind::Metadata => 0,
            EventKind::TextNote => 1,
            EventKind::RecommendRelay => 2,
            EventKind::Contacts => 3,
            EventKind::EncryptedDirectMessage => 4,
            EventKind::EventDeletion => 5,
            EventKind::Repost => 6,
            EventKind::Reaction => 7,
            EventKind::BadgeAward => 8,
            EventKind::ChannelCreation => 40,
            EventKind::ChannelMetadata => 41,
            EventKind::ChannelMessage => 42,
            EventKind::ChannelHideMessage => 43,
            EventKind::ChannelMuteUser => 44,
            EventKind::Custom(v) => v,
        }
    }
}

impl EventKind {
    pub fn from_u32(value: u32) -> Option<Self> {
        Some(value.into())
    }

    pub fn as_u32(self) -> u32 {
        self.into()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub pubkey: String,
    pub created_at: DateTime<Utc>,
    pub kind: u32,
    pub tags: Vec<Vec<String>>,
    pub content: String,
    pub sig: String,
}

impl Event {
    pub fn new(kind: u32, content: String, pubkey: String) -> Self {
        Self {
            id: String::new(),
            pubkey,
            created_at: chrono::Utc::now(),
            kind,
            tags: Vec::new(),
            content,
            sig: String::new(),
        }
    }

    pub fn with_tags(mut self, tags: Vec<Vec<String>>) -> Self {
        self.tags = tags;
        self
    }

    pub fn add_tag(&mut self, tag: Vec<String>) {
        self.tags.push(tag);
    }

    pub fn add_p_tag(&mut self, pubkey: String) {
        self.tags.push(vec!["p".to_string(), pubkey]);
    }

    pub fn add_e_tag(&mut self, event_id: String) {
        self.tags.push(vec!["e".to_string(), event_id]);
    }

    pub fn add_t_tag(&mut self, hashtag: String) {
        self.tags.push(vec!["t".to_string(), hashtag]);
    }

    pub fn get_referenced_event_ids(&self) -> Vec<String> {
        self.tags
            .iter()
            .filter(|tag| tag.len() >= 2 && tag[0] == "e")
            .map(|tag| tag[1].clone())
            .collect()
    }

    pub fn get_referenced_pubkeys(&self) -> Vec<String> {
        self.tags
            .iter()
            .filter(|tag| tag.len() >= 2 && tag[0] == "p")
            .map(|tag| tag[1].clone())
            .collect()
    }

    pub fn get_hashtags(&self) -> Vec<String> {
        self.tags
            .iter()
            .filter(|tag| tag.len() >= 2 && tag[0] == "t")
            .map(|tag| tag[1].clone())
            .collect()
    }

    pub fn new_with_id(
        id: EventId,
        pubkey: String,
        content: String,
        kind: u32,
        tags: Vec<Vec<String>>,
        created_at: DateTime<Utc>,
        sig: String,
    ) -> Self {
        Self {
            id: id.to_hex(),
            pubkey,
            created_at,
            kind,
            tags,
            content,
            sig,
        }
    }

    /// NIP-01に基づく基本バリデーション
    /// - idは[0,pubkey,created_at,kind,tags,content]のsha256
    /// - pubkeyは32byte hex（64桁）
    /// - sigは64byte hex（128桁）
    pub fn validate_nip01(&self) -> ValidationResult<()> {
        let is_hex = |s: &str| s.chars().all(|c| c.is_ascii_hexdigit());
        if self.pubkey.len() != 64 || !is_hex(&self.pubkey) {
            return Err(EventValidationError::new(
                ValidationFailureKind::Nip01Integrity,
                "invalid pubkey (expect 64 hex)",
            ));
        }
        if self.sig.len() != 128 || !is_hex(&self.sig) {
            return Err(EventValidationError::new(
                ValidationFailureKind::Nip01Integrity,
                "invalid sig (expect 128 hex)",
            ));
        }
        if self.id.len() != 64 || !is_hex(&self.id) {
            return Err(EventValidationError::new(
                ValidationFailureKind::Nip01Integrity,
                "invalid id (expect 64 hex)",
            ));
        }

        let created_at_secs = self.created_at.timestamp();
        let arr = serde_json::json!([
            0,
            self.pubkey,
            created_at_secs,
            self.kind,
            self.tags,
            self.content,
        ]);
        let serialized = serde_json::to_vec(&arr).map_err(|e| {
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
                format!(
                    "too many tags: {} (max {})",
                    self.tags.len(),
                    MAX_EVENT_TAGS
                ),
            ));
        }

        let content_len = self.content.len();
        if content_len > MAX_EVENT_CONTENT_BYTES {
            return Err(EventValidationError::new(
                ValidationFailureKind::ContentTooLarge,
                format!("content exceeds {MAX_EVENT_CONTENT_BYTES} bytes (actual {content_len})"),
            ));
        }

        Ok(())
    }

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
                    if !Self::is_hex_n(evref, 64) {
                        Self::ensure_event_ref(evref)?;
                    }
                    if tag.len() >= 3 {
                        let relay_url = tag[2].as_str();
                        if !relay_url.is_empty() && !Self::is_ws_url(relay_url) {
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
                    if !Self::is_hex_n(pkref, 64) {
                        Self::ensure_pubkey_ref(pkref)?;
                    }
                    if tag.len() >= 3 {
                        let relay_url = tag[2].as_str();
                        if !relay_url.is_empty() && !Self::is_ws_url(relay_url) {
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

    pub fn validate_for_gateway(&self) -> ValidationResult<()> {
        self.validate_nip01()?;
        self.validate_nip10_19()?;
        if self.kind == KIND30078_KIND {
            self.validate_kind30078()?;
        }
        Ok(())
    }

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
                    if topic_tag.is_some() {
                        return Err(EventValidationError::new(
                            ValidationFailureKind::Kind30078TagMismatch,
                            "multiple t tags detected for kind30078",
                        ));
                    }
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
                    if address_tag.is_some() {
                        return Err(EventValidationError::new(
                            ValidationFailureKind::Kind30078TagMismatch,
                            "multiple a tags detected for kind30078",
                        ));
                    }
                    if tag.len() != 2 {
                        return Err(EventValidationError::new(
                            ValidationFailureKind::Kind30078TagMismatch,
                            format!(
                                "a tag must be [\"a\", \"30078:<pubkey>:...\"] (len={})",
                                tag.len()
                            ),
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

        let body_len = parsed.body.len();
        if body_len > MAX_EVENT_CONTENT_BYTES {
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

    fn is_hex_n(s: &str, n: usize) -> bool {
        s.len() == n && s.chars().all(|c| c.is_ascii_hexdigit())
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
            Self::validate_nevent_tlv(s)
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
            Self::validate_nprofile_tlv(s)
        } else {
            Err(EventValidationError::new(
                ValidationFailureKind::Nip10TagStructure,
                format!("unsupported p tag reference format: {s}"),
            ))
        }
    }

    fn is_ws_url(url: &str) -> bool {
        let lower = url.to_ascii_lowercase();
        (lower.starts_with("ws://") || lower.starts_with("wss://")) && lower.len() > 5
    }

    fn validate_nprofile_tlv(s: &str) -> ValidationResult<()> {
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
        Self::parse_tlv(&bytes, |tag, value| match tag {
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
                if relay_count > Self::MAX_TLV_RELAY_URLS {
                    return Err(EventValidationError::new(
                        ValidationFailureKind::Nip19Tlv,
                        format!(
                            "nprofile relay entries exceed max {}",
                            Self::MAX_TLV_RELAY_URLS
                        ),
                    ));
                }
                Self::validate_tlv_relay(value)
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

    fn validate_nevent_tlv(s: &str) -> ValidationResult<()> {
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
        Self::parse_tlv(&bytes, |tag, value| match tag {
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
                if relay_count > Self::MAX_TLV_RELAY_URLS {
                    return Err(EventValidationError::new(
                        ValidationFailureKind::Nip19Tlv,
                        format!(
                            "nevent relay entries exceed max {}",
                            Self::MAX_TLV_RELAY_URLS
                        ),
                    ));
                }
                Self::validate_tlv_relay(value)
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

    const MAX_TLV_RELAY_URLS: usize = 16;
    const MAX_TLV_RELAY_URL_LEN: usize = 255;

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
        if value.len() > Self::MAX_TLV_RELAY_URL_LEN {
            return Err(EventValidationError::new(
                ValidationFailureKind::Nip19Tlv,
                format!("relay url exceeds {} bytes", Self::MAX_TLV_RELAY_URL_LEN),
            ));
        }
        if value.is_empty() {
            return Ok(());
        }
        match std::str::from_utf8(value) {
            Ok(url) => {
                if url.is_ascii() && Self::is_ws_url(url) {
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
}

#[cfg(test)]
mod tests {
    use crate::shared::validation::ValidationFailureKind;
    use chrono::Duration;
    use nostr_sdk::prelude::*;
    use serde_json::json;
    use sha2::{Digest, Sha256};

    #[tokio::test]
    async fn test_validate_nip01_ok() {
        let keys = Keys::generate();
        let nostr_ev = EventBuilder::text_note("hello nip01")
            .sign_with_keys(&keys)
            .unwrap();

        let created_at =
            chrono::DateTime::<chrono::Utc>::from_timestamp(nostr_ev.created_at.as_u64() as i64, 0)
                .unwrap();

        let dom = super::Event {
            id: nostr_ev.id.to_string(),
            pubkey: nostr_ev.pubkey.to_string(),
            created_at,
            kind: nostr_ev.kind.as_u16() as u32,
            tags: nostr_ev.tags.iter().map(|t| t.clone().to_vec()).collect(),
            content: nostr_ev.content.clone(),
            sig: nostr_ev.sig.to_string(),
        };

        assert!(dom.validate_nip01().is_ok());
    }

    #[tokio::test]
    async fn test_validate_nip01_bad_id() {
        let keys = Keys::generate();
        let nostr_ev = EventBuilder::text_note("oops")
            .sign_with_keys(&keys)
            .unwrap();

        let created_at =
            chrono::DateTime::<chrono::Utc>::from_timestamp(nostr_ev.created_at.as_u64() as i64, 0)
                .unwrap();

        let mut dom = super::Event {
            id: nostr_ev.id.to_string(),
            pubkey: nostr_ev.pubkey.to_string(),
            created_at,
            kind: nostr_ev.kind.as_u16() as u32,
            tags: nostr_ev.tags.iter().map(|t| t.clone().to_vec()).collect(),
            content: nostr_ev.content.clone(),
            sig: nostr_ev.sig.to_string(),
        };
        // 改ざん
        dom.content = "tampered".into();
        let err = dom.validate_nip01().unwrap_err();
        assert_eq!(err.kind, ValidationFailureKind::Nip01Integrity);
    }

    fn build_event_with_data(
        pubkey: &str,
        kind: u32,
        tags: Vec<Vec<String>>,
        content: &str,
        created_at: chrono::DateTime<chrono::Utc>,
    ) -> super::Event {
        let id_payload = json!([0, pubkey, created_at.timestamp(), kind, tags, content]);
        let serialized = serde_json::to_vec(&id_payload).expect("serialize event");
        let id = format!("{:x}", Sha256::digest(&serialized));
        super::Event {
            id,
            pubkey: pubkey.to_string(),
            created_at,
            kind,
            tags,
            content: content.to_string(),
            sig: "f".repeat(128),
        }
    }

    #[test]
    fn test_validate_nip01_rejects_timestamp_drift() {
        let created_at = chrono::Utc::now() - Duration::hours(2);
        let event = build_event_with_data(&"f".repeat(64), 1, Vec::new(), "time drift", created_at);
        let err = event.validate_nip01().unwrap_err();
        assert_eq!(err.kind, ValidationFailureKind::TimestampOutOfRange);
    }
}

#[cfg(test)]
mod nip10_19_tests {
    use crate::shared::validation::ValidationFailureKind;
    use bech32::{ToBase32 as _, Variant};
    use chrono::Duration;
    use nostr_sdk::prelude::*;
    use serde_json::json;
    use sha2::{Digest, Sha256};

    fn dummy_event_with_tags(tags: Vec<Vec<String>>) -> super::Event {
        super::Event {
            id: "0".repeat(64),
            pubkey: "f".repeat(64),
            created_at: chrono::Utc::now(),
            kind: 1,
            tags,
            content: String::new(),
            sig: "f".repeat(128),
        }
    }

    #[test]
    fn test_validate_nip10_19_ok_with_bech32_refs() {
        let keys = Keys::generate();
        let npub = keys.public_key().to_bech32().unwrap();

        // 参照用のイベントID
        let nostr_ev = EventBuilder::text_note("x").sign_with_keys(&keys).unwrap();
        let note = nostr_ev.id.to_bech32().unwrap();

        let e_root = vec!["e".into(), note.clone(), String::new(), "root".into()];
        let e_reply = vec!["e".into(), note, String::new(), "reply".into()];
        let p_tag = vec!["p".into(), npub];
        let ev = dummy_event_with_tags(vec![e_root, e_reply, p_tag]);
        assert!(ev.validate_nip10_19().is_ok());
    }

    #[test]
    fn test_validate_nip10_19_rejects_invalid_marker_and_pk() {
        let e_tag = vec!["e".into(), "0".repeat(64), String::new(), "bad".into()];
        let p_tag = vec!["p".into(), "zzz".into()];
        let ev = dummy_event_with_tags(vec![e_tag, p_tag]);
        let err = ev.validate_nip10_19().unwrap_err();
        assert_eq!(err.kind, ValidationFailureKind::Nip10TagStructure);
    }

    #[test]
    fn test_validate_nip10_reply_without_root_ok() {
        let e_tag_reply = vec!["e".into(), "0".repeat(64), String::new(), "reply".into()];
        let ev = dummy_event_with_tags(vec![e_tag_reply]);
        assert!(ev.validate_nip10_19().is_ok());
    }

    #[test]
    fn test_nprofile_tlv_multiple_relays_ok() {
        let keys = Keys::generate();
        let mut bytes = Vec::new();
        bytes.push(0);
        bytes.push(32);
        bytes.extend_from_slice(&keys.public_key().to_bytes());
        for relay in ["wss://relay.one", "wss://relay.two"] {
            let relay_bytes = relay.as_bytes();
            bytes.push(1);
            bytes.push(relay_bytes.len() as u8);
            bytes.extend_from_slice(relay_bytes);
        }
        let encoded =
            bech32::encode("nprofile", bytes.to_base32(), Variant::Bech32).expect("encode");
        assert!(super::Event::validate_nprofile_tlv(&encoded).is_ok());
    }

    #[test]
    fn test_nprofile_tlv_rejects_invalid_relay_scheme() {
        let keys = Keys::generate();
        let mut bytes = Vec::new();
        bytes.push(0);
        bytes.push(32);
        bytes.extend_from_slice(&keys.public_key().to_bytes());
        let relay_bytes = b"https://relay.invalid";
        bytes.push(1);
        bytes.push(relay_bytes.len() as u8);
        bytes.extend_from_slice(relay_bytes);
        let encoded =
            bech32::encode("nprofile", bytes.to_base32(), Variant::Bech32).expect("encode");
        assert!(super::Event::validate_nprofile_tlv(&encoded).is_err());
    }

    #[test]
    fn test_nevent_tlv_with_optional_author_and_kind() {
        let keys = Keys::generate();
        let nostr_ev = EventBuilder::text_note("tlv")
            .sign_with_keys(&keys)
            .expect("sign");
        let mut bytes = Vec::new();
        bytes.push(0);
        bytes.push(32);
        bytes.extend_from_slice(&nostr_ev.id.to_bytes());
        let relay_bytes = b"wss://relay.example";
        bytes.push(1);
        bytes.push(relay_bytes.len() as u8);
        bytes.extend_from_slice(relay_bytes);
        bytes.push(2);
        bytes.push(32);
        bytes.extend_from_slice(&nostr_ev.pubkey.to_bytes());
        let kind_bytes = (nostr_ev.kind.as_u16() as u32).to_be_bytes();
        bytes.push(3);
        bytes.push(kind_bytes.len() as u8);
        bytes.extend_from_slice(&kind_bytes);
        let encoded = bech32::encode("nevent", bytes.to_base32(), Variant::Bech32).unwrap();
        assert!(super::Event::validate_nevent_tlv(&encoded).is_ok());
    }

    #[test]
    fn test_nevent_tlv_rejects_invalid_author_length() {
        let mut bytes = Vec::new();
        bytes.push(0);
        bytes.push(32);
        bytes.extend_from_slice(&[0u8; 32]);
        bytes.push(2);
        bytes.push(31);
        bytes.extend_from_slice(&[0u8; 31]);
        let encoded = bech32::encode("nevent", bytes.to_base32(), Variant::Bech32).unwrap();
        assert!(super::Event::validate_nevent_tlv(&encoded).is_err());
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
        matches!(up,
            '0'..='9'
            | 'A'..='H'
            | 'J'..='K'
            | 'M'..='N'
            | 'P'..='T'
            | 'V'..='Z')
    })
}

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

#[cfg(test)]
mod kind30078_tests {
    use super::*;
    use crate::shared::validation::ValidationFailureKind;
    use chrono::Utc;
    use serde_json::json;
    use sha2::{Digest, Sha256};

    fn build_kind30078_event(
        pubkey: String,
        tags: Vec<Vec<String>>,
        content: serde_json::Value,
    ) -> super::Event {
        let created_at = Utc::now();
        let kind = KIND30078_KIND;
        let content_str = content.to_string();
        let pubkey_for_id = pubkey.clone();
        let tags_for_event = tags;
        let tags_for_id = tags_for_event.clone();
        let id_payload = json!([
            0,
            pubkey_for_id,
            created_at.timestamp(),
            kind,
            tags_for_id,
            content_str
        ]);
        let serialized = serde_json::to_vec(&id_payload).expect("serialize kind30078 event");
        let id = format!("{:x}", Sha256::digest(&serialized));
        super::Event {
            id,
            pubkey,
            created_at,
            kind,
            tags: tags_for_event,
            content: content_str,
            sig: "f".repeat(128),
        }
    }

    fn base_kind30078_tags(pubkey: &str, slug: &str, revision: &str) -> Vec<Vec<String>> {
        vec![
            vec![
                "d".into(),
                format!("kukuri:topic:{}:post:{}", slug, revision),
            ],
            vec!["k".into(), "topic-post".into()],
            vec!["t".into(), format!("topic:{}", slug)],
            vec![
                "a".into(),
                format!("30078:{}:kukuri:topic:{}:post:{}", pubkey, slug, revision),
            ],
        ]
    }

    #[test]
    fn test_validate_kind30078_ok() {
        let slug = "sample-topic";
        let revision = "A".repeat(26);
        let pubkey = "f".repeat(64);
        let tags = base_kind30078_tags(&pubkey, slug, &revision);
        let content = json!({
            "body": "hello",
            "attachments": ["iroh://attachment"],
            "metadata": {"app_version": "1.0.0", "edited": false}
        });
        let event = build_kind30078_event(pubkey.clone(), tags, content);
        assert!(event.validate_kind30078().is_ok());
    }

    #[test]
    fn test_validate_kind30078_missing_d_tag() {
        let mut tags = base_kind30078_tags(&"f".repeat(64), "slug", &"A".repeat(26));
        tags.retain(|tag| tag.first().map(|s| s != "d").unwrap_or(true));
        let content = json!({
            "body": "hello",
            "attachments": [],
            "metadata": {"app_version": "1.0.0", "edited": false}
        });
        let event = build_kind30078_event("f".repeat(64), tags, content);
        let err = event.validate_kind30078().unwrap_err();
        assert_eq!(err.kind, ValidationFailureKind::Kind30078TagMissing);
    }

    #[test]
    fn test_validate_kind30078_invalid_t_value() {
        let slug = "sample-topic";
        let revision = "A".repeat(26);
        let pubkey = "f".repeat(64);
        let mut tags = base_kind30078_tags(&pubkey, slug, &revision);
        if let Some(t_tag) = tags
            .iter_mut()
            .find(|tag| tag.first().map(|s| s == "t").unwrap_or(false))
        {
            t_tag[1] = "topic:wrong".into();
        }
        let content = json!({
            "body": "hello",
            "attachments": [],
            "metadata": {"app_version": "1.0.0", "edited": false}
        });
        let event = build_kind30078_event(pubkey.clone(), tags, content);
        let err = event.validate_kind30078().unwrap_err();
        assert_eq!(err.kind, ValidationFailureKind::Kind30078TagMismatch);
    }

    #[test]
    fn test_validate_kind30078_invalid_attachment() {
        let slug = "sample-topic";
        let revision = "A".repeat(26);
        let pubkey = "f".repeat(64);
        let tags = base_kind30078_tags(&pubkey, slug, &revision);
        let content = json!({
            "body": "hello",
            "attachments": ["invalid://attachment"],
            "metadata": {"app_version": "1.0.0", "edited": false}
        });
        let event = build_kind30078_event(pubkey.clone(), tags, content);
        let err = event.validate_kind30078().unwrap_err();
        assert_eq!(err.kind, ValidationFailureKind::Kind30078ContentSchema);
    }

    #[test]
    fn test_validate_kind30078_missing_a_tag() {
        let slug = "sample-topic";
        let revision = "A".repeat(26);
        let pubkey = "f".repeat(64);
        let mut tags = base_kind30078_tags(&pubkey, slug, &revision);
        tags.retain(|tag| tag.first().map(|s| s != "a").unwrap_or(true));
        let content = json!({
            "body": "hello",
            "attachments": [],
            "metadata": {"app_version": "1.0.0", "edited": false}
        });
        let event = build_kind30078_event(pubkey.clone(), tags, content);
        let err = event.validate_kind30078().unwrap_err();
        assert_eq!(err.kind, ValidationFailureKind::Kind30078TagMissing);
    }

    #[test]
    fn test_validate_kind30078_attachment_overflow() {
        let slug = "sample-topic";
        let revision = "A".repeat(26);
        let pubkey = "f".repeat(64);
        let tags = base_kind30078_tags(&pubkey, slug, &revision);
        let attachments: Vec<String> = (0..=KIND30078_MAX_ATTACHMENTS)
            .map(|i| format!("iroh://attachment/{i}"))
            .collect();
        let content = json!({
            "body": "hello",
            "attachments": attachments,
            "metadata": {"app_version": "1.0.0", "edited": false}
        });
        let event = build_kind30078_event(pubkey.clone(), tags, content);
        let err = event.validate_kind30078().unwrap_err();
        assert_eq!(err.kind, ValidationFailureKind::Kind30078ContentSize);
    }

    #[test]
    fn test_validate_kind30078_invalid_semver() {
        let slug = "sample-topic";
        let revision = "A".repeat(26);
        let pubkey = "f".repeat(64);
        let tags = base_kind30078_tags(&pubkey, slug, &revision);
        let content = json!({
            "body": "hello",
            "attachments": [],
            "metadata": {"app_version": "not-a-version", "edited": false}
        });
        let event = build_kind30078_event(pubkey.clone(), tags, content);
        let err = event.validate_kind30078().unwrap_err();
        assert_eq!(err.kind, ValidationFailureKind::Kind30078ContentSchema);
    }
}
