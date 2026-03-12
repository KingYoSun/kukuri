use anyhow::{Context, Result, anyhow, bail};
use nostr_sdk::JsonUtil;
use nostr_sdk::prelude::{
    Event as NostrEvent, EventBuilder, Keys, Marker, PublicKey, Tag, TagKind, TagStandard, ToBech32,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventId(pub String);

impl EventId {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<String> for EventId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for EventId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Pubkey(pub String);

impl Pubkey {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<String> for Pubkey {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for Pubkey {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TopicId(pub String);

impl TopicId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReplicaId(pub String);

impl ReplicaId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BlobHash(pub String);

impl BlobHash {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PayloadRef {
    InlineText {
        text: String,
    },
    BlobText {
        hash: BlobHash,
        mime: String,
        bytes: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetRef {
    pub hash: BlobHash,
    pub mime: String,
    pub bytes: u64,
    pub role: AssetRole,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetRole {
    ImageOriginal,
    ImagePreview,
    VideoPoster,
    VideoManifest,
    Attachment,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiveSignalKind {
    SessionStarted,
    SessionEnded,
    RoomActivity,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GossipHint {
    TopicIndexUpdated {
        topic_id: TopicId,
        event_ids: Vec<EventId>,
    },
    ThreadUpdated {
        root_id: EventId,
        event_ids: Vec<EventId>,
    },
    ProfileUpdated {
        author: Pubkey,
    },
    Presence {
        topic_id: TopicId,
        author: Pubkey,
        ttl_ms: u32,
    },
    Typing {
        topic_id: TopicId,
        root_id: Option<EventId>,
        author: Pubkey,
        ttl_ms: u32,
    },
    LiveSignal {
        topic_id: TopicId,
        session_id: String,
        kind: LiveSignalKind,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalPostHeader {
    pub event_id: EventId,
    pub topic_id: TopicId,
    pub author: Pubkey,
    pub root: Option<EventId>,
    pub reply_to: Option<EventId>,
    pub created_at: i64,
    pub payload_ref: PayloadRef,
    pub attachments: Vec<AssetRef>,
    pub signature: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadRef {
    pub root: EventId,
    pub reply_to: Option<EventId>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub pubkey: Pubkey,
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub about: Option<String>,
    pub picture: Option<String>,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub pubkey: Pubkey,
    pub created_at: i64,
    pub kind: u16,
    pub tags: Vec<Vec<String>>,
    pub content: String,
    pub sig: String,
}

impl Event {
    pub fn from_nostr(event: NostrEvent) -> Self {
        let tags = event.tags.into_iter().map(|tag| tag.to_vec()).collect();

        Self {
            id: EventId(event.id.to_hex()),
            pubkey: Pubkey(event.pubkey.to_hex()),
            created_at: event.created_at.as_secs() as i64,
            kind: event.kind.as_u16(),
            tags,
            content: event.content,
            sig: event.sig.to_string(),
        }
    }

    pub fn as_nostr(&self) -> Result<NostrEvent> {
        let json = serde_json::to_string(self)?;
        NostrEvent::from_json(json).map_err(Into::into)
    }

    pub fn verify_nip01(&self) -> Result<()> {
        let event = self.as_nostr()?;
        event.verify().context("failed to verify nostr event")
    }

    pub fn validate_nip10(&self) -> Result<()> {
        let e_tags = self
            .tags
            .iter()
            .filter(|tag| tag.first().map(String::as_str) == Some("e"))
            .collect::<Vec<_>>();
        if e_tags.is_empty() {
            return Ok(());
        }

        let root_tags = e_tags
            .iter()
            .filter(|tag| tag.get(3).map(String::as_str) == Some("root"))
            .collect::<Vec<_>>();
        let reply_tags = e_tags
            .iter()
            .filter(|tag| tag.get(3).map(String::as_str) == Some("reply"))
            .collect::<Vec<_>>();

        if root_tags.len() != 1 {
            bail!("reply event must include exactly one root tag");
        }
        if reply_tags.len() != 1 {
            bail!("reply event must include exactly one reply tag");
        }
        Ok(())
    }

    pub fn topic_id(&self) -> Option<TopicId> {
        self.tags
            .iter()
            .find_map(|tag| match tag.first().map(String::as_str) {
                Some("topic" | "t") if tag.len() >= 2 => Some(TopicId::new(tag[1].clone())),
                _ => None,
            })
    }

    pub fn thread_ref(&self) -> Option<ThreadRef> {
        let root = self
            .tags
            .iter()
            .find(|tag| {
                tag.first().map(String::as_str) == Some("e")
                    && tag.get(3).map(String::as_str) == Some("root")
            })
            .and_then(|tag| tag.get(1).cloned())
            .map(EventId::from);
        let reply = self
            .tags
            .iter()
            .find(|tag| {
                tag.first().map(String::as_str) == Some("e")
                    && tag.get(3).map(String::as_str) == Some("reply")
            })
            .and_then(|tag| tag.get(1).cloned())
            .map(EventId::from);

        root.or_else(|| reply.clone()).map(|root| ThreadRef {
            root,
            reply_to: reply,
        })
    }

    pub fn note_id(&self) -> Result<String> {
        let id = nostr_sdk::EventId::from_hex(self.id.as_str())?;
        id.to_bech32().map_err(Into::into)
    }

    pub fn author_npub(&self) -> Result<String> {
        let pubkey = PublicKey::from_hex(self.pubkey.as_str())?;
        pubkey.to_bech32().map_err(Into::into)
    }

    pub fn to_canonical_header(&self, payload_ref: PayloadRef) -> CanonicalPostHeader {
        let thread = self.thread_ref();
        CanonicalPostHeader {
            event_id: self.id.clone(),
            topic_id: self
                .topic_id()
                .unwrap_or_else(|| TopicId::new("kukuri:topic:unknown")),
            author: self.pubkey.clone(),
            root: thread.as_ref().map(|thread| thread.root.clone()),
            reply_to: thread.and_then(|thread| thread.reply_to),
            created_at: self.created_at,
            payload_ref,
            attachments: Vec::new(),
            signature: self.sig.clone(),
        }
    }
}

pub fn blob_hash(data: impl AsRef<[u8]>) -> BlobHash {
    BlobHash::new(blake3::hash(data.as_ref()).to_hex().to_string())
}

pub fn timeline_sort_key(created_at: i64, event_id: &EventId) -> String {
    format!("{created_at:020}-{}", event_id.as_str())
}

pub fn generate_keys() -> Keys {
    Keys::generate()
}

pub fn build_text_note(
    keys: &Keys,
    topic: &TopicId,
    content: &str,
    reply_to: Option<&Event>,
) -> Result<Event> {
    let mut tags = vec![
        Tag::hashtag(topic.as_str()),
        Tag::custom(TagKind::Custom("topic".into()), vec![topic.0.clone()]),
    ];

    if let Some(parent) = reply_to {
        let thread = parent.thread_ref().unwrap_or(ThreadRef {
            root: parent.id.clone(),
            reply_to: Some(parent.id.clone()),
        });
        let root_id = nostr_sdk::EventId::from_hex(thread.root.as_str())?;
        let reply_id = nostr_sdk::EventId::from_hex(parent.id.as_str())?;
        let parent_pubkey = PublicKey::from_hex(parent.pubkey.as_str())?;

        tags.push(Tag::from_standardized(TagStandard::Event {
            event_id: root_id,
            relay_url: None,
            marker: Some(Marker::Root),
            public_key: None,
            uppercase: false,
        }));
        tags.push(Tag::from_standardized(TagStandard::Event {
            event_id: reply_id,
            relay_url: None,
            marker: Some(Marker::Reply),
            public_key: None,
            uppercase: false,
        }));
        let parent_pubkey_hex = parent_pubkey.to_hex();
        tags.push(Tag::parse(["p", parent_pubkey_hex.as_str()])?);
    }

    let event = EventBuilder::text_note(content)
        .tags(tags)
        .sign_with_keys(keys)
        .map_err(|error| anyhow!(error))?;

    Ok(Event::from_nostr(event))
}

pub fn parse_profile(event: &Event) -> Result<Option<Profile>> {
    if event.kind != 0 {
        return Ok(None);
    }

    let metadata: serde_json::Value =
        serde_json::from_str(&event.content).context("failed to parse metadata event")?;

    Ok(Some(Profile {
        pubkey: event.pubkey.clone(),
        name: metadata
            .get("name")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned),
        display_name: metadata
            .get("display_name")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned),
        about: metadata
            .get("about")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned),
        picture: metadata
            .get("picture")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned),
        updated_at: event.created_at,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nip01_event_roundtrip_json() {
        let keys = generate_keys();
        let topic = TopicId::new("kukuri:topic:contract");
        let event = build_text_note(&keys, &topic, "hello", None).expect("event");
        let json = serde_json::to_string(&event).expect("serialize");
        let restored: Event = serde_json::from_str(&json).expect("deserialize");

        restored.verify_nip01().expect("NIP-01 verification");
        assert_eq!(restored.id, event.id);
        assert_eq!(restored.topic_id(), Some(topic));
    }

    #[test]
    fn nip10_root_reply_rules() {
        let keys = generate_keys();
        let root = build_text_note(&keys, &TopicId::new("kukuri:topic:thread"), "root", None)
            .expect("root");
        let reply = build_text_note(
            &keys,
            &TopicId::new("kukuri:topic:thread"),
            "reply",
            Some(&root),
        )
        .expect("reply");

        reply.verify_nip01().expect("NIP-01 verification");
        reply.validate_nip10().expect("NIP-10 validation");

        let thread = reply.thread_ref().expect("thread ref");
        assert_eq!(thread.root, root.id);
        assert_eq!(thread.reply_to, Some(root.id));
    }

    #[test]
    fn nip19_display_only_not_in_wire() {
        let keys = generate_keys();
        let event = build_text_note(&keys, &TopicId::new("kukuri:topic:wire"), "display", None)
            .expect("event");
        let json = serde_json::to_string(&event).expect("serialize");
        let note = event.note_id().expect("note id");
        let npub = event.author_npub().expect("npub");

        assert!(note.starts_with("note1"));
        assert!(npub.starts_with("npub1"));
        assert!(!json.contains("note1"));
        assert!(!json.contains("npub1"));
    }

    #[test]
    fn gossip_hint_contains_no_payload_body() {
        let hint = GossipHint::TopicIndexUpdated {
            topic_id: TopicId::new("kukuri:topic:docs"),
            event_ids: vec![EventId::from("event-1"), EventId::from("event-2")],
        };

        let json = serde_json::to_string(&hint).expect("serialize hint");
        assert!(json.contains("TopicIndexUpdated"));
        assert!(!json.contains("hello"));
        assert!(!json.contains("content"));
        assert!(!json.contains("payload_ref"));
    }

    #[test]
    fn blob_hash_roundtrip() {
        let payload = "hello blobs";
        let hash = blob_hash(payload);
        let restored = BlobHash::new(hash.as_str());

        assert_eq!(restored, hash);
        assert_eq!(hash.as_str().len(), 64);
    }
}
