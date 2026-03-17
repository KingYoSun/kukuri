use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use nostr_sdk::prelude::{Keys, PublicKey};
use nostr_sdk::secp256k1::Message;
use nostr_sdk::secp256k1::schnorr::Signature;
use nostr_sdk::{SECP256K1, hashes::Hash, hashes::sha256};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EnvelopeId(pub String);

impl EnvelopeId {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<String> for EnvelopeId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for EnvelopeId {
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
    InlineText { text: String },
    BlobText { hash: BlobHash, mime: String, bytes: u64 },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetRef {
    pub hash: BlobHash,
    pub mime: String,
    pub bytes: u64,
    pub role: AssetRole,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestBlobRef {
    pub hash: BlobHash,
    pub mime: String,
    pub bytes: u64,
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
pub enum LiveSessionStatus {
    Scheduled,
    Live,
    Paused,
    Ended,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameRoomStatus {
    Waiting,
    Running,
    Paused,
    Ended,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectVisibility {
    Public,
    Community,
    Room,
    Private,
}

impl Default for ObjectVisibility {
    fn default() -> Self {
        Self::Public
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectStatus {
    Active,
    Edited,
    Deleted,
    Tombstoned,
}

impl Default for ObjectStatus {
    fn default() -> Self {
        Self::Active
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaManifestItem {
    pub blob_hash: BlobHash,
    pub mime: String,
    pub size: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_ms: Option<u64>,
    pub codec: Option<String>,
    pub thumbnail_blob_hash: Option<BlobHash>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KukuriMediaManifestV1 {
    pub manifest_id: String,
    pub owner_pubkey: Pubkey,
    pub created_at: i64,
    pub items: Vec<MediaManifestItem>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveSessionManifestBlobV1 {
    pub session_id: String,
    pub topic_id: TopicId,
    pub owner_pubkey: Pubkey,
    pub title: String,
    pub description: String,
    pub status: LiveSessionStatus,
    pub started_at: i64,
    pub ended_at: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveSessionStateDocV1 {
    pub session_id: String,
    pub topic_id: TopicId,
    pub owner_pubkey: Pubkey,
    pub created_at: i64,
    pub updated_at: i64,
    pub status: LiveSessionStatus,
    pub current_manifest: ManifestBlobRef,
    pub last_envelope_id: EnvelopeId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameParticipant {
    pub participant_id: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameScoreEntry {
    pub participant_id: String,
    pub label: String,
    pub score: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRoomManifestBlobV1 {
    pub room_id: String,
    pub topic_id: TopicId,
    pub owner_pubkey: Pubkey,
    pub title: String,
    pub description: String,
    pub status: GameRoomStatus,
    pub phase_label: Option<String>,
    pub participants: Vec<GameParticipant>,
    pub scores: Vec<GameScoreEntry>,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRoomStateDocV1 {
    pub room_id: String,
    pub topic_id: TopicId,
    pub owner_pubkey: Pubkey,
    pub created_at: i64,
    pub updated_at: i64,
    pub status: GameRoomStatus,
    pub current_manifest: ManifestBlobRef,
    pub last_envelope_id: EnvelopeId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HintObjectRef {
    pub object_id: String,
    pub object_kind: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GossipHint {
    TopicObjectsChanged {
        topic_id: TopicId,
        objects: Vec<HintObjectRef>,
    },
    ThreadUpdated {
        root_id: EnvelopeId,
        object_ids: Vec<EnvelopeId>,
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
        root_id: Option<EnvelopeId>,
        author: Pubkey,
        ttl_ms: u32,
    },
    SessionChanged {
        topic_id: TopicId,
        session_id: String,
        object_kind: String,
    },
    LivePresence {
        topic_id: TopicId,
        session_id: String,
        author: Pubkey,
        ttl_ms: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KukuriPostEnvelopeContentV1 {
    pub object_kind: String,
    pub topic_id: TopicId,
    pub payload_ref: PayloadRef,
    #[serde(default)]
    pub attachments: Vec<AssetRef>,
    #[serde(default)]
    pub media_manifest_refs: Vec<String>,
    #[serde(default)]
    pub visibility: ObjectVisibility,
    pub reply_to: Option<EnvelopeId>,
    pub root_id: Option<EnvelopeId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KukuriPostObjectV1 {
    pub object_id: EnvelopeId,
    pub envelope_id: EnvelopeId,
    pub object_kind: String,
    pub topic_id: TopicId,
    pub author: Pubkey,
    pub created_at: i64,
    pub updated_at: i64,
    pub payload_ref: PayloadRef,
    pub attachments: Vec<AssetRef>,
    pub media_manifest_refs: Vec<String>,
    pub visibility: ObjectVisibility,
    pub reply_to: Option<EnvelopeId>,
    pub root: Option<EnvelopeId>,
    pub status: ObjectStatus,
    pub signature: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KukuriAuthEnvelopeContentV1 {
    pub scope: String,
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
pub struct ThreadRef {
    pub root: EnvelopeId,
    pub reply_to: Option<EnvelopeId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KukuriEnvelope {
    pub id: EnvelopeId,
    pub pubkey: Pubkey,
    pub created_at: i64,
    pub kind: String,
    pub tags: Vec<Vec<String>>,
    pub content: String,
    pub sig: String,
}

pub type Event = KukuriEnvelope;
pub type EventId = EnvelopeId;
pub type CanonicalPostHeader = KukuriPostObjectV1;

impl KukuriEnvelope {
    pub fn verify(&self) -> Result<()> {
        let canonical = canonical_envelope_payload(
            self.pubkey.as_str(),
            self.created_at,
            self.kind.as_str(),
            &self.tags,
            self.content.as_str(),
        )?;
        let digest = sha256::Hash::hash(canonical.as_bytes()).to_byte_array();
        let computed_id = hex::encode(digest);
        if computed_id != self.id.0 {
            bail!("envelope id mismatch");
        }
        let message = Message::from_digest(digest);
        let signature = Signature::from_str(self.sig.as_str()).context("invalid envelope sig")?;
        let public_key =
            PublicKey::from_hex(self.pubkey.as_str()).context("invalid envelope pubkey")?;
        let xonly = public_key.xonly().context("invalid xonly pubkey")?;
        SECP256K1
            .verify_schnorr(&signature, &message, &xonly)
            .context("envelope signature verification failed")?;
        Ok(())
    }

    pub fn topic_id(&self) -> Option<TopicId> {
        self.tags
            .iter()
            .find_map(|tag| match tag.first().map(String::as_str) {
                Some("topic" | "context") if tag.len() >= 2 => Some(TopicId::new(tag[1].clone())),
                _ => None,
            })
            .or_else(|| {
                self.post_content()
                    .ok()
                    .flatten()
                    .map(|content| content.topic_id)
            })
    }

    pub fn thread_ref(&self) -> Option<ThreadRef> {
        let root = self
            .tags
            .iter()
            .find(|tag| tag.first().map(String::as_str) == Some("root"))
            .and_then(|tag| tag.get(1).cloned())
            .filter(|value| !value.trim().is_empty())
            .map(EnvelopeId::from);
        let reply = self
            .tags
            .iter()
            .find(|tag| tag.first().map(String::as_str) == Some("reply_to"))
            .and_then(|tag| tag.get(1).cloned())
            .filter(|value| !value.trim().is_empty())
            .map(EnvelopeId::from);

        root.or_else(|| {
            self.post_content()
                .ok()
                .flatten()
                .and_then(|content| content.root_id.or(content.reply_to.clone()))
        })
        .map(|root| ThreadRef {
            root,
            reply_to: reply.or_else(|| {
                self.post_content()
                    .ok()
                    .flatten()
                    .and_then(|content| content.reply_to)
            }),
        })
    }

    pub fn post_content(&self) -> Result<Option<KukuriPostEnvelopeContentV1>> {
        if !matches!(self.kind.as_str(), "post" | "comment") {
            return Ok(None);
        }
        serde_json::from_str(self.content.as_str())
            .map(Some)
            .context("failed to parse post envelope content")
    }

    pub fn to_post_object(&self) -> Result<Option<KukuriPostObjectV1>> {
        let Some(content) = self.post_content()? else {
            return Ok(None);
        };
        Ok(Some(KukuriPostObjectV1 {
            object_id: self.id.clone(),
            envelope_id: self.id.clone(),
            object_kind: content.object_kind,
            topic_id: content.topic_id,
            author: self.pubkey.clone(),
            created_at: self.created_at,
            updated_at: self.created_at,
            payload_ref: content.payload_ref,
            attachments: content.attachments,
            media_manifest_refs: content.media_manifest_refs,
            visibility: content.visibility,
            reply_to: content.reply_to,
            root: content.root_id,
            status: ObjectStatus::Active,
            signature: self.sig.clone(),
        }))
    }
}

pub fn blob_hash(data: impl AsRef<[u8]>) -> BlobHash {
    BlobHash::new(blake3::hash(data.as_ref()).to_hex().to_string())
}

pub const LIVE_MANIFEST_MIME: &str = "application/vnd.kukuri.live-manifest+json";
pub const GAME_MANIFEST_MIME: &str = "application/vnd.kukuri.game-manifest+json";

pub fn timeline_sort_key(created_at: i64, object_id: &EnvelopeId) -> String {
    format!("{created_at:020}-{}", object_id.as_str())
}

pub fn generate_keys() -> Keys {
    Keys::generate()
}

pub fn build_post_envelope(
    keys: &Keys,
    topic: &TopicId,
    body: &str,
    reply_to: Option<&KukuriEnvelope>,
) -> Result<KukuriEnvelope> {
    build_post_envelope_with_payload(
        keys,
        topic,
        PayloadRef::InlineText {
            text: body.to_string(),
        },
        Vec::new(),
        Vec::new(),
        reply_to,
        ObjectVisibility::Public,
    )
}

pub fn build_post_envelope_with_payload(
    keys: &Keys,
    topic: &TopicId,
    payload_ref: PayloadRef,
    attachments: Vec<AssetRef>,
    media_manifest_refs: Vec<String>,
    reply_to: Option<&KukuriEnvelope>,
    visibility: ObjectVisibility,
) -> Result<KukuriEnvelope> {
    let thread = reply_to.and_then(KukuriEnvelope::thread_ref).unwrap_or_else(|| {
        reply_to
            .map(|parent| ThreadRef {
                root: parent.id.clone(),
                reply_to: Some(parent.id.clone()),
            })
            .unwrap_or(ThreadRef {
                root: EnvelopeId::default(),
                reply_to: None,
            })
    });
    let kind = if reply_to.is_some() { "comment" } else { "post" };
    let root_id = reply_to.map(|_| thread.root.clone());
    let reply_id = reply_to.map(|parent| parent.id.clone());
    let content = KukuriPostEnvelopeContentV1 {
        object_kind: kind.to_string(),
        topic_id: topic.clone(),
        payload_ref,
        attachments,
        media_manifest_refs,
        visibility,
        reply_to: reply_id.clone(),
        root_id: root_id.clone(),
    };
    let mut tags = vec![
        vec!["topic".into(), topic.as_str().into()],
        vec!["object".into(), kind.into()],
    ];
    if let Some(root_id) = root_id {
        tags.push(vec!["root".into(), root_id.0]);
    }
    if let Some(reply_id) = reply_id {
        tags.push(vec!["reply_to".into(), reply_id.0]);
    }
    sign_envelope_json(keys, kind, tags, &content)
}

pub fn build_media_manifest_envelope(
    keys: &Keys,
    topic: &TopicId,
    manifest: &KukuriMediaManifestV1,
) -> Result<KukuriEnvelope> {
    sign_envelope_json(
        keys,
        "media-manifest",
        vec![
            vec!["topic".into(), topic.as_str().into()],
            vec!["object".into(), "media-manifest".into()],
            vec!["manifest_id".into(), manifest.manifest_id.clone()],
        ],
        manifest,
    )
}

pub fn build_live_session_envelope<T: Serialize>(
    keys: &Keys,
    topic: &TopicId,
    session_id: &str,
    content: &T,
) -> Result<KukuriEnvelope> {
    sign_envelope_json(
        keys,
        "live-session",
        vec![
            vec!["topic".into(), topic.as_str().into()],
            vec!["object".into(), "live-session".into()],
            vec!["session_id".into(), session_id.to_string()],
        ],
        content,
    )
}

pub fn build_game_session_envelope<T: Serialize>(
    keys: &Keys,
    topic: &TopicId,
    room_id: &str,
    content: &T,
) -> Result<KukuriEnvelope> {
    sign_envelope_json(
        keys,
        "game-session",
        vec![
            vec!["topic".into(), topic.as_str().into()],
            vec!["object".into(), "game-session".into()],
            vec!["room_id".into(), room_id.to_string()],
        ],
        content,
    )
}

pub fn parse_profile(envelope: &KukuriEnvelope) -> Result<Option<Profile>> {
    if envelope.kind != "identity-profile" {
        return Ok(None);
    }

    let metadata: serde_json::Value =
        serde_json::from_str(&envelope.content).context("failed to parse profile envelope")?;

    Ok(Some(Profile {
        pubkey: envelope.pubkey.clone(),
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
        updated_at: envelope.created_at,
    }))
}

pub fn sign_envelope_json<T: Serialize>(
    keys: &Keys,
    kind: impl Into<String>,
    tags: Vec<Vec<String>>,
    content: &T,
) -> Result<KukuriEnvelope> {
    let content = serde_json::to_string(content).context("failed to encode envelope content")?;
    sign_envelope(keys, kind, tags, content)
}

pub fn sign_envelope(
    keys: &Keys,
    kind: impl Into<String>,
    tags: Vec<Vec<String>>,
    content: String,
) -> Result<KukuriEnvelope> {
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before unix epoch")?
        .as_secs() as i64;
    sign_envelope_at(keys, kind, tags, content, created_at)
}

pub fn sign_envelope_at(
    keys: &Keys,
    kind: impl Into<String>,
    tags: Vec<Vec<String>>,
    content: String,
    created_at: i64,
) -> Result<KukuriEnvelope> {
    let kind = kind.into();
    let pubkey = keys.public_key().to_hex();
    let canonical = canonical_envelope_payload(
        pubkey.as_str(),
        created_at,
        kind.as_str(),
        &tags,
        content.as_str(),
    )?;
    let digest = sha256::Hash::hash(canonical.as_bytes()).to_byte_array();
    let id = hex::encode(digest);
    let message = Message::from_digest(digest);
    let sig = keys.sign_schnorr(&message).to_string();
    Ok(KukuriEnvelope {
        id: EnvelopeId(id),
        pubkey: Pubkey(pubkey),
        created_at,
        kind,
        tags,
        content,
        sig,
    })
}

fn canonical_envelope_payload(
    pubkey: &str,
    created_at: i64,
    kind: &str,
    tags: &[Vec<String>],
    content: &str,
) -> Result<String> {
    serde_json::to_string(&serde_json::json!([0, pubkey, created_at, kind, tags, content]))
        .context("failed to encode canonical envelope payload")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_envelope_roundtrip_json() {
        let keys = generate_keys();
        let topic = TopicId::new("kukuri:topic:contract");
        let envelope = build_post_envelope(&keys, &topic, "hello", None).expect("envelope");
        let json = serde_json::to_string(&envelope).expect("serialize");
        let restored: KukuriEnvelope = serde_json::from_str(&json).expect("deserialize");

        restored.verify().expect("signature verification");
        assert_eq!(restored.id, envelope.id);
        assert_eq!(restored.topic_id(), Some(topic));
    }

    #[test]
    fn comment_envelope_tracks_root_and_reply() {
        let keys = generate_keys();
        let root =
            build_post_envelope(&keys, &TopicId::new("kukuri:topic:thread"), "root", None)
                .expect("root");
        let reply = build_post_envelope(
            &keys,
            &TopicId::new("kukuri:topic:thread"),
            "reply",
            Some(&root),
        )
        .expect("reply");

        reply.verify().expect("signature verification");

        let thread = reply.thread_ref().expect("thread ref");
        assert_eq!(thread.root, root.id);
        assert_eq!(thread.reply_to, Some(root.id));
        assert_eq!(reply.kind, "comment");
    }

    #[test]
    fn mutation_breaks_signature_verification() {
        let keys = generate_keys();
        let mut envelope =
            build_post_envelope(&keys, &TopicId::new("kukuri:topic:wire"), "display", None)
                .expect("envelope");
        envelope.content = "mutated".to_string();

        let error = envelope.verify().expect_err("verification should fail");
        assert!(error.to_string().contains("mismatch") || error.to_string().contains("failed"));
    }

    #[test]
    fn media_manifest_envelope_uses_protocol_object_kind() {
        let keys = generate_keys();
        let envelope = build_media_manifest_envelope(
            &keys,
            &TopicId::new("kukuri:topic:media"),
            &KukuriMediaManifestV1 {
                manifest_id: "manifest-1".into(),
                owner_pubkey: Pubkey(keys.public_key().to_hex()),
                created_at: 1,
                items: vec![MediaManifestItem {
                    blob_hash: BlobHash::new("blob-1"),
                    mime: "image/png".into(),
                    size: 123,
                    width: Some(10),
                    height: Some(10),
                    duration_ms: None,
                    codec: None,
                    thumbnail_blob_hash: None,
                }],
            },
        )
        .expect("manifest envelope");

        envelope.verify().expect("signature verification");
        assert_eq!(envelope.kind, "media-manifest");
    }
}
