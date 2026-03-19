use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use bech32::{Bech32, Hrp};
use secp256k1::rand::thread_rng;
use secp256k1::schnorr::Signature;
use secp256k1::{Keypair, Message, SECP256K1, SecretKey, XOnlyPublicKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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

pub const LEGACY_SECRET_HRP: &str = "nsec";

#[derive(Clone)]
pub struct KukuriKeys {
    secret_key: SecretKey,
}

impl std::fmt::Debug for KukuriKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KukuriKeys").finish_non_exhaustive()
    }
}

impl KukuriKeys {
    pub fn generate() -> Self {
        Self {
            secret_key: SecretKey::new(&mut thread_rng()),
        }
    }

    pub fn parse(secret: &str) -> Result<Self> {
        let secret_key = parse_secret_key(secret)?;
        Ok(Self { secret_key })
    }

    pub fn public_key_hex(&self) -> String {
        let keypair = Keypair::from_secret_key(SECP256K1, &self.secret_key);
        let (pubkey, _) = keypair.x_only_public_key();
        pubkey.to_string()
    }

    pub fn public_key(&self) -> Pubkey {
        Pubkey(self.public_key_hex())
    }

    pub fn export_secret_hex(&self) -> String {
        hex::encode(self.secret_key.secret_bytes())
    }

    pub fn sign_schnorr(&self, message: &Message) -> Signature {
        let keypair = Keypair::from_secret_key(SECP256K1, &self.secret_key);
        SECP256K1.sign_schnorr(message, &keypair)
    }
}

fn parse_secret_key(secret: &str) -> Result<SecretKey> {
    let trimmed = secret.trim();
    if let Ok(bytes) = hex::decode(trimmed)
        && bytes.len() == 32
    {
        return SecretKey::from_slice(bytes.as_slice()).context("invalid hex secret key");
    }

    let (hrp, bytes) = bech32::decode(trimmed).context("failed to decode secret key")?;
    if hrp.as_str() != LEGACY_SECRET_HRP {
        bail!("unsupported secret key hrp `{}`", hrp.as_str());
    }
    SecretKey::from_slice(bytes.as_slice()).context("invalid bech32 secret key")
}

pub fn encode_secret_key_bech32(secret_key_hex: &str, hrp: &str) -> Result<String> {
    let bytes = hex::decode(secret_key_hex).context("invalid hex secret key")?;
    if bytes.len() != 32 {
        bail!("invalid secret key length");
    }
    bech32::encode::<Bech32>(
        Hrp::parse(hrp).context("invalid secret key hrp")?,
        bytes.as_slice(),
    )
    .context("failed to encode secret key")
}

fn sha256_digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
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
pub struct ChannelId(pub String);

impl ChannelId {
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
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChannelRef {
    Public,
    PrivateChannel { channel_id: ChannelId },
}

impl Default for ChannelRef {
    fn default() -> Self {
        Self::Public
    }
}

impl ChannelRef {
    pub fn channel_id(&self) -> Option<&ChannelId> {
        match self {
            Self::Public => None,
            Self::PrivateChannel { channel_id } => Some(channel_id),
        }
    }

    pub fn visibility(&self) -> ObjectVisibility {
        match self {
            Self::Public => ObjectVisibility::Public,
            Self::PrivateChannel { .. } => ObjectVisibility::Private,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TimelineScope {
    Public,
    AllJoined,
    Channel { channel_id: ChannelId },
}

impl Default for TimelineScope {
    fn default() -> Self {
        Self::Public
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
    #[serde(default)]
    pub channel_id: Option<ChannelId>,
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
    #[serde(default)]
    pub channel_id: Option<ChannelId>,
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
    #[serde(default)]
    pub channel_id: Option<ChannelId>,
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
    #[serde(default)]
    pub channel_id: Option<ChannelId>,
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
    #[serde(default)]
    pub channel_id: Option<ChannelId>,
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
    #[serde(default)]
    pub channel_id: Option<ChannelId>,
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

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KukuriProfileEnvelopeContentV1 {
    pub author_pubkey: Pubkey,
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub about: Option<String>,
    pub picture: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorProfileDocV1 {
    pub author_pubkey: Pubkey,
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub about: Option<String>,
    pub picture: Option<String>,
    pub updated_at: i64,
    pub envelope_id: EnvelopeId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FollowEdgeStatus {
    Active,
    Revoked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KukuriFollowEdgeEnvelopeContentV1 {
    pub subject_pubkey: Pubkey,
    pub target_pubkey: Pubkey,
    pub status: FollowEdgeStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FollowEdge {
    pub subject_pubkey: Pubkey,
    pub target_pubkey: Pubkey,
    pub status: FollowEdgeStatus,
    pub updated_at: i64,
    pub envelope_id: EnvelopeId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FollowEdgeDocV1 {
    pub subject_pubkey: Pubkey,
    pub target_pubkey: Pubkey,
    pub status: FollowEdgeStatus,
    pub updated_at: i64,
    pub envelope_id: EnvelopeId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadRef {
    pub root: EnvelopeId,
    pub reply_to: Option<EnvelopeId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePrivateChannelInput {
    pub topic_id: TopicId,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivateChannelMetadataDocV1 {
    pub channel_id: ChannelId,
    pub topic_id: TopicId,
    pub label: String,
    pub creator_pubkey: Pubkey,
    pub created_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KukuriPrivateChannelInviteEnvelopeContentV1 {
    pub channel_id: ChannelId,
    pub topic_id: TopicId,
    pub channel_label: String,
    pub namespace_secret_hex: String,
    pub expires_at: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivateChannelInviteTokenV1 {
    pub envelope: KukuriEnvelope,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivateChannelInvitePreview {
    pub channel_id: ChannelId,
    pub topic_id: TopicId,
    pub channel_label: String,
    pub inviter_pubkey: Pubkey,
    pub expires_at: Option<i64>,
    pub namespace_secret_hex: String,
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
        let digest = sha256_digest(canonical.as_bytes());
        let computed_id = hex::encode(digest);
        if computed_id != self.id.0 {
            bail!("envelope id mismatch");
        }
        let message = Message::from_digest_slice(&digest).context("invalid envelope digest")?;
        let signature = Signature::from_str(self.sig.as_str()).context("invalid envelope sig")?;
        let public_key =
            XOnlyPublicKey::from_str(self.pubkey.as_str()).context("invalid envelope pubkey")?;
        SECP256K1
            .verify_schnorr(&signature, &message, &public_key)
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
            channel_id: content.channel_id,
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

pub fn generate_keys() -> KukuriKeys {
    KukuriKeys::generate()
}

pub fn build_post_envelope(
    keys: &KukuriKeys,
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
    keys: &KukuriKeys,
    topic: &TopicId,
    payload_ref: PayloadRef,
    attachments: Vec<AssetRef>,
    media_manifest_refs: Vec<String>,
    reply_to: Option<&KukuriEnvelope>,
    visibility: ObjectVisibility,
) -> Result<KukuriEnvelope> {
    build_post_envelope_with_payload_in_channel(
        keys,
        topic,
        payload_ref,
        attachments,
        media_manifest_refs,
        reply_to,
        visibility,
        None,
    )
}

pub fn build_post_envelope_with_payload_in_channel(
    keys: &KukuriKeys,
    topic: &TopicId,
    payload_ref: PayloadRef,
    attachments: Vec<AssetRef>,
    media_manifest_refs: Vec<String>,
    reply_to: Option<&KukuriEnvelope>,
    visibility: ObjectVisibility,
    channel_id: Option<&ChannelId>,
) -> Result<KukuriEnvelope> {
    let thread = reply_to
        .and_then(KukuriEnvelope::thread_ref)
        .unwrap_or_else(|| {
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
    let kind = if reply_to.is_some() {
        "comment"
    } else {
        "post"
    };
    let root_id = reply_to.map(|_| thread.root.clone());
    let reply_id = reply_to.map(|parent| parent.id.clone());
    let content = KukuriPostEnvelopeContentV1 {
        object_kind: kind.to_string(),
        topic_id: topic.clone(),
        channel_id: channel_id.cloned(),
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
    if let Some(channel_id) = channel_id {
        tags.push(vec!["channel".into(), channel_id.as_str().to_string()]);
    }
    sign_envelope_json(keys, kind, tags, &content)
}

pub fn build_media_manifest_envelope(
    keys: &KukuriKeys,
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

pub fn build_profile_envelope(
    keys: &KukuriKeys,
    content: &KukuriProfileEnvelopeContentV1,
) -> Result<KukuriEnvelope> {
    let author_pubkey = keys.public_key();
    if content.author_pubkey != author_pubkey {
        bail!("profile author pubkey must match signer");
    }
    let created_at = now_timestamp_millis()?;
    let encoded = serde_json::to_string(content).context("failed to encode envelope content")?;
    sign_envelope_at(
        keys,
        "identity-profile",
        vec![
            vec!["author".into(), content.author_pubkey.as_str().to_string()],
            vec!["object".into(), "identity-profile".into()],
        ],
        encoded,
        created_at,
    )
}

pub fn build_follow_edge_envelope(
    keys: &KukuriKeys,
    target_pubkey: &Pubkey,
    status: FollowEdgeStatus,
) -> Result<KukuriEnvelope> {
    let subject_pubkey = keys.public_key();
    if subject_pubkey == *target_pubkey {
        bail!("self follow is not allowed");
    }
    let content = KukuriFollowEdgeEnvelopeContentV1 {
        subject_pubkey: subject_pubkey.clone(),
        target_pubkey: target_pubkey.clone(),
        status,
    };
    let created_at = now_timestamp_millis()?;
    let encoded = serde_json::to_string(&content).context("failed to encode envelope content")?;
    sign_envelope_at(
        keys,
        "follow-edge",
        vec![
            vec!["subject".into(), subject_pubkey.as_str().to_string()],
            vec!["target".into(), target_pubkey.as_str().to_string()],
            vec!["object".into(), "follow-edge".into()],
        ],
        encoded,
        created_at,
    )
}

pub fn build_live_session_envelope<T: Serialize>(
    keys: &KukuriKeys,
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
    keys: &KukuriKeys,
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

pub fn build_private_channel_invite_token(
    keys: &KukuriKeys,
    topic: &TopicId,
    channel_id: &ChannelId,
    channel_label: &str,
    namespace_secret_hex: &str,
    expires_at: Option<i64>,
) -> Result<String> {
    let token = PrivateChannelInviteTokenV1 {
        envelope: sign_envelope_json(
            keys,
            "channel-invite",
            vec![
                vec!["topic".into(), topic.as_str().to_string()],
                vec!["object".into(), "channel-invite".into()],
                vec!["channel".into(), channel_id.as_str().to_string()],
            ],
            &KukuriPrivateChannelInviteEnvelopeContentV1 {
                channel_id: channel_id.clone(),
                topic_id: topic.clone(),
                channel_label: channel_label.trim().to_string(),
                namespace_secret_hex: namespace_secret_hex.trim().to_string(),
                expires_at,
            },
        )?,
    };
    serde_json::to_string(&token).context("failed to encode private channel invite token")
}

pub fn parse_private_channel_invite_token(token: &str) -> Result<PrivateChannelInvitePreview> {
    let token: PrivateChannelInviteTokenV1 =
        serde_json::from_str(token).context("failed to parse private channel invite token")?;
    token.envelope.verify()?;
    if token.envelope.kind != "channel-invite" {
        bail!("invite envelope kind must be channel-invite");
    }
    let content: KukuriPrivateChannelInviteEnvelopeContentV1 =
        serde_json::from_str(token.envelope.content.as_str())
            .context("failed to decode private channel invite content")?;
    if content.channel_label.trim().is_empty() {
        bail!("channel invite label is required");
    }
    let secret_bytes =
        hex::decode(content.namespace_secret_hex.trim()).context("invalid invite secret hex")?;
    if secret_bytes.len() != 32 {
        bail!("invite secret must be 32 bytes");
    }
    if let Some(expires_at) = content.expires_at
        && expires_at < now_timestamp_millis()?
    {
        bail!("invite has expired");
    }
    Ok(PrivateChannelInvitePreview {
        channel_id: content.channel_id,
        topic_id: content.topic_id,
        channel_label: content.channel_label,
        inviter_pubkey: token.envelope.pubkey,
        expires_at: content.expires_at,
        namespace_secret_hex: content.namespace_secret_hex,
    })
}

pub fn parse_profile(envelope: &KukuriEnvelope) -> Result<Option<Profile>> {
    if envelope.kind != "identity-profile" {
        return Ok(None);
    }

    let metadata: KukuriProfileEnvelopeContentV1 =
        serde_json::from_str(&envelope.content).context("failed to parse profile envelope")?;
    validate_pubkey(metadata.author_pubkey.as_str()).context("invalid profile author pubkey")?;
    if metadata.author_pubkey != envelope.pubkey {
        bail!("profile author pubkey must match envelope signer");
    }

    Ok(Some(Profile {
        pubkey: envelope.pubkey.clone(),
        name: metadata.name,
        display_name: metadata.display_name,
        about: metadata.about,
        picture: metadata.picture,
        updated_at: envelope.created_at,
    }))
}

pub fn parse_follow_edge(envelope: &KukuriEnvelope) -> Result<Option<FollowEdge>> {
    if envelope.kind != "follow-edge" {
        return Ok(None);
    }

    let content: KukuriFollowEdgeEnvelopeContentV1 =
        serde_json::from_str(&envelope.content).context("failed to parse follow edge envelope")?;
    validate_pubkey(content.subject_pubkey.as_str()).context("invalid follow subject pubkey")?;
    validate_pubkey(content.target_pubkey.as_str()).context("invalid follow target pubkey")?;
    if content.subject_pubkey != envelope.pubkey {
        bail!("follow subject pubkey must match envelope signer");
    }
    if content.subject_pubkey == content.target_pubkey {
        bail!("self follow is not allowed");
    }

    Ok(Some(FollowEdge {
        subject_pubkey: content.subject_pubkey,
        target_pubkey: content.target_pubkey,
        status: content.status,
        updated_at: envelope.created_at,
        envelope_id: envelope.id.clone(),
    }))
}

pub fn sign_envelope_json<T: Serialize>(
    keys: &KukuriKeys,
    kind: impl Into<String>,
    tags: Vec<Vec<String>>,
    content: &T,
) -> Result<KukuriEnvelope> {
    let content = serde_json::to_string(content).context("failed to encode envelope content")?;
    sign_envelope(keys, kind, tags, content)
}

pub fn sign_envelope(
    keys: &KukuriKeys,
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
    keys: &KukuriKeys,
    kind: impl Into<String>,
    tags: Vec<Vec<String>>,
    content: String,
    created_at: i64,
) -> Result<KukuriEnvelope> {
    let kind = kind.into();
    let pubkey = keys.public_key_hex();
    let canonical = canonical_envelope_payload(
        pubkey.as_str(),
        created_at,
        kind.as_str(),
        &tags,
        content.as_str(),
    )?;
    let digest = sha256_digest(canonical.as_bytes());
    let id = hex::encode(digest);
    let message = Message::from_digest_slice(&digest).context("invalid envelope digest")?;
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
    serde_json::to_string(&serde_json::json!([
        0, pubkey, created_at, kind, tags, content
    ]))
    .context("failed to encode canonical envelope payload")
}

fn validate_pubkey(value: &str) -> Result<()> {
    XOnlyPublicKey::from_str(value).context("invalid x-only public key")?;
    Ok(())
}

fn now_timestamp_millis() -> Result<i64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before unix epoch")?
        .as_millis() as i64)
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
        let root = build_post_envelope(&keys, &TopicId::new("kukuri:topic:thread"), "root", None)
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
                owner_pubkey: keys.public_key(),
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

    #[test]
    fn profile_envelope_roundtrip() {
        let keys = generate_keys();
        let envelope = build_profile_envelope(
            &keys,
            &KukuriProfileEnvelopeContentV1 {
                author_pubkey: keys.public_key(),
                name: Some("alice".into()),
                display_name: Some("Alice".into()),
                about: Some("hello".into()),
                picture: Some("https://example.com/alice.png".into()),
            },
        )
        .expect("profile envelope");

        envelope.verify().expect("signature verification");
        let profile = parse_profile(&envelope)
            .expect("parse profile")
            .expect("profile");
        assert_eq!(profile.pubkey, keys.public_key());
        assert_eq!(profile.display_name.as_deref(), Some("Alice"));
        assert_eq!(profile.about.as_deref(), Some("hello"));
    }

    #[test]
    fn follow_edge_roundtrip_and_self_follow_rejected() {
        let keys = generate_keys();
        let target = generate_keys().public_key();
        let envelope = build_follow_edge_envelope(&keys, &target, FollowEdgeStatus::Active)
            .expect("follow edge envelope");

        envelope.verify().expect("signature verification");
        let edge = parse_follow_edge(&envelope)
            .expect("parse follow edge")
            .expect("follow edge");
        assert_eq!(edge.subject_pubkey, keys.public_key());
        assert_eq!(edge.target_pubkey, target);
        assert_eq!(edge.status, FollowEdgeStatus::Active);

        let self_follow_error =
            build_follow_edge_envelope(&keys, &keys.public_key(), FollowEdgeStatus::Active)
                .expect_err("self follow should be rejected");
        assert!(self_follow_error.to_string().contains("self follow"));
    }

    #[test]
    fn follow_edge_parser_rejects_subject_mismatch() {
        let signer = generate_keys();
        let subject = generate_keys().public_key();
        let target = generate_keys().public_key();
        let envelope = sign_envelope_json(
            &signer,
            "follow-edge",
            vec![vec!["object".into(), "follow-edge".into()]],
            &KukuriFollowEdgeEnvelopeContentV1 {
                subject_pubkey: subject,
                target_pubkey: target,
                status: FollowEdgeStatus::Active,
            },
        )
        .expect("envelope");

        let error = parse_follow_edge(&envelope).expect_err("subject mismatch must fail");
        assert!(error.to_string().contains("subject pubkey must match"));
    }
}
