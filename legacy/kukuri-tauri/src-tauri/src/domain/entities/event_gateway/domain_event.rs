use crate::domain::entities::{Event, EventKind};
use crate::domain::value_objects::{EventId, PublicKey};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Nostr イベントのタグを表現するドメイン型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventTag {
    name: String,
    values: Vec<String>,
}

impl EventTag {
    /// 新しいタグを生成する。タグ名は1文字以上である必要がある。
    pub fn new<N: Into<String>>(name: N, values: Vec<String>) -> Result<Self, String> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err("Tag name cannot be empty".to_string());
        }
        Ok(Self { name, values })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn values(&self) -> &[String] {
        &self.values
    }

    pub fn to_raw(&self) -> Vec<String> {
        std::iter::once(self.name.clone())
            .chain(self.values.iter().cloned())
            .collect()
    }

    pub fn into_raw(self) -> Vec<String> {
        std::iter::once(self.name).chain(self.values).collect()
    }
}

impl TryFrom<Vec<String>> for EventTag {
    type Error = String;

    fn try_from(value: Vec<String>) -> Result<Self, Self::Error> {
        let mut iter = value.into_iter();
        let name = iter
            .next()
            .ok_or_else(|| "Tag vector must contain at least one element".to_string())?;
        let values: Vec<String> = iter.collect();
        Self::new(name, values)
    }
}

impl From<EventTag> for Vec<String> {
    fn from(tag: EventTag) -> Self {
        tag.into_raw()
    }
}

/// Application 層が扱う Nostr ドメインイベント。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainEvent {
    pub id: EventId,
    pub author: PublicKey,
    pub kind: EventKind,
    pub created_at: DateTime<Utc>,
    pub content: String,
    pub tags: Vec<EventTag>,
    pub signature: String,
}

impl DomainEvent {
    const SIGNATURE_LENGTH: usize = 128;

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: EventId,
        author: PublicKey,
        kind: EventKind,
        created_at: DateTime<Utc>,
        content: String,
        tags: Vec<EventTag>,
        signature: String,
    ) -> Result<Self, String> {
        Self::validate_signature(&signature)?;
        Ok(Self {
            id,
            author,
            kind,
            created_at,
            content,
            tags,
            signature,
        })
    }

    pub fn with_tags(mut self, tags: Vec<EventTag>) -> Self {
        self.tags = tags;
        self
    }

    pub fn signature(&self) -> &str {
        &self.signature
    }

    pub fn kind(&self) -> EventKind {
        self.kind
    }

    pub fn to_event(&self) -> Event {
        Event {
            id: self.id.to_hex(),
            pubkey: self.author.as_hex().to_string(),
            created_at: self.created_at,
            kind: u32::from(self.kind),
            tags: self.tags.iter().map(EventTag::to_raw).collect(),
            content: self.content.clone(),
            sig: self.signature.clone(),
        }
    }

    fn validate_signature(signature: &str) -> Result<(), String> {
        if signature.len() != Self::SIGNATURE_LENGTH {
            return Err("Signature must be 128 hex characters".to_string());
        }
        if !signature.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("Signature must contain only hex characters".to_string());
        }
        Ok(())
    }
}

impl TryFrom<Event> for DomainEvent {
    type Error = String;

    fn try_from(value: Event) -> Result<Self, Self::Error> {
        let id = EventId::from_hex(&value.id)?;
        let author = PublicKey::from_hex_str(&value.pubkey)?;
        let kind = EventKind::from(value.kind);
        let tags = value
            .tags
            .into_iter()
            .map(EventTag::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        Self::new(
            id,
            author,
            kind,
            value.created_at,
            value.content,
            tags,
            value.sig,
        )
    }
}

impl TryFrom<&Event> for DomainEvent {
    type Error = String;

    fn try_from(value: &Event) -> Result<Self, Self::Error> {
        let id = EventId::from_hex(&value.id)?;
        let author = PublicKey::from_hex_str(&value.pubkey)?;
        let kind = EventKind::from(value.kind);
        let tags = value
            .tags
            .iter()
            .cloned()
            .map(EventTag::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        Self::new(
            id,
            author,
            kind,
            value.created_at,
            value.content.clone(),
            tags,
            value.sig.clone(),
        )
    }
}

impl From<&DomainEvent> for Event {
    fn from(value: &DomainEvent) -> Self {
        value.to_event()
    }
}
