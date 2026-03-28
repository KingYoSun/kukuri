use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};
use bech32::{Bech32, Hrp};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use hkdf::Hkdf;
use secp256k1::ecdh::SharedSecret;
use secp256k1::rand::{RngCore, rng};
use secp256k1::schnorr::Signature;
use secp256k1::{Keypair, Parity, PublicKey, SECP256K1, SecretKey, XOnlyPublicKey};
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
            secret_key: SecretKey::new(&mut rng()),
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

    pub fn sign_schnorr(&self, message: &[u8]) -> Signature {
        let keypair = Keypair::from_secret_key(SECP256K1, &self.secret_key);
        SECP256K1.sign_schnorr(message, &keypair)
    }
}

fn parse_secret_key(secret: &str) -> Result<SecretKey> {
    let trimmed = secret.trim();
    if let Ok(bytes) = hex::decode(trimmed)
        && bytes.len() == 32
    {
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| anyhow!("invalid hex secret key length"))?;
        return SecretKey::from_byte_array(bytes).context("invalid hex secret key");
    }

    let (hrp, bytes) = bech32::decode(trimmed).context("failed to decode secret key")?;
    if hrp.as_str() != LEGACY_SECRET_HRP {
        bail!("unsupported secret key hrp `{}`", hrp.as_str());
    }
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow!("invalid bech32 secret key length"))?;
    SecretKey::from_byte_array(bytes).context("invalid bech32 secret key")
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

pub fn author_profile_topic_id(author_pubkey: &str) -> TopicId {
    TopicId::new(format!("kukuri:topic:profile:{author_pubkey}"))
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
    #[serde(default)]
    pub repost_of: Option<RepostSourceSnapshotV1>,
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
    #[serde(default)]
    pub repost_of: Option<RepostSourceSnapshotV1>,
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
pub struct RepostSourceSnapshotV1 {
    pub source_object_id: EnvelopeId,
    pub source_topic_id: TopicId,
    pub source_author_pubkey: Pubkey,
    pub source_object_kind: String,
    pub content: String,
    #[serde(default)]
    pub attachments: Vec<AssetRef>,
    #[serde(default)]
    pub reply_to_object_id: Option<EnvelopeId>,
    #[serde(default)]
    pub root_id: Option<EnvelopeId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReactionKeyKind {
    Emoji,
    CustomAsset,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomReactionAssetSnapshotV1 {
    pub asset_id: String,
    pub owner_pubkey: Pubkey,
    pub blob_hash: BlobHash,
    pub mime: String,
    pub bytes: u64,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReactionKeyV1 {
    Emoji { emoji: String },
    CustomAsset {
        asset_id: String,
        snapshot: CustomReactionAssetSnapshotV1,
    },
}

impl ReactionKeyV1 {
    pub fn normalized_key(&self) -> Result<String> {
        match self {
            Self::Emoji { emoji } => {
                let emoji = normalize_reaction_emoji(emoji)
                    .ok_or_else(|| anyhow!("reaction emoji must not be empty"))?;
                Ok(format!("emoji:{emoji}"))
            }
            Self::CustomAsset { asset_id, snapshot } => {
                let asset_id = asset_id.trim();
                if asset_id.is_empty() {
                    bail!("custom reaction asset id must not be empty");
                }
                if snapshot.asset_id.trim() != asset_id {
                    bail!("custom reaction asset snapshot id must match reaction key");
                }
                Ok(format!("custom_asset:{asset_id}"))
            }
        }
    }

    pub fn key_kind(&self) -> ReactionKeyKind {
        match self {
            Self::Emoji { .. } => ReactionKeyKind::Emoji,
            Self::CustomAsset { .. } => ReactionKeyKind::CustomAsset,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KukuriReactionEnvelopeContentV1 {
    pub reaction_id: EnvelopeId,
    pub target_topic_id: TopicId,
    #[serde(default)]
    pub channel_id: Option<ChannelId>,
    pub target_object_id: EnvelopeId,
    pub reaction_key_kind: ReactionKeyKind,
    pub normalized_reaction_key: String,
    #[serde(default)]
    pub emoji: Option<String>,
    #[serde(default)]
    pub custom_asset_id: Option<String>,
    #[serde(default)]
    pub custom_asset_snapshot: Option<CustomReactionAssetSnapshotV1>,
    #[serde(default)]
    pub status: ObjectStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactionDocV1 {
    pub reaction_id: EnvelopeId,
    pub target_topic_id: TopicId,
    #[serde(default)]
    pub channel_id: Option<ChannelId>,
    pub target_object_id: EnvelopeId,
    pub author_pubkey: Pubkey,
    pub created_at: i64,
    pub updated_at: i64,
    pub reaction_key_kind: ReactionKeyKind,
    pub normalized_reaction_key: String,
    #[serde(default)]
    pub emoji: Option<String>,
    #[serde(default)]
    pub custom_asset_id: Option<String>,
    #[serde(default)]
    pub custom_asset_snapshot: Option<CustomReactionAssetSnapshotV1>,
    pub status: ObjectStatus,
    pub envelope_id: EnvelopeId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KukuriCustomReactionAssetEnvelopeContentV1 {
    pub author_pubkey: Pubkey,
    pub blob_hash: BlobHash,
    pub mime: String,
    pub bytes: u64,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomReactionAssetDocV1 {
    pub asset_id: String,
    pub author_pubkey: Pubkey,
    pub blob_hash: BlobHash,
    pub mime: String,
    pub bytes: u64,
    pub width: u32,
    pub height: u32,
    pub created_at: i64,
    pub updated_at: i64,
    pub envelope_id: EnvelopeId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KukuriProfilePostEnvelopeContentV1 {
    pub author_pubkey: Pubkey,
    pub profile_topic_id: TopicId,
    pub published_topic_id: TopicId,
    pub object_id: EnvelopeId,
    pub created_at: i64,
    pub object_kind: String,
    pub content: String,
    #[serde(default)]
    pub attachments: Vec<AssetRef>,
    #[serde(default)]
    pub reply_to_object_id: Option<EnvelopeId>,
    #[serde(default)]
    pub root_id: Option<EnvelopeId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorProfilePostDocV1 {
    pub author_pubkey: Pubkey,
    pub profile_topic_id: TopicId,
    pub published_topic_id: TopicId,
    pub object_id: EnvelopeId,
    pub created_at: i64,
    pub object_kind: String,
    pub content: String,
    #[serde(default)]
    pub attachments: Vec<AssetRef>,
    #[serde(default)]
    pub reply_to_object_id: Option<EnvelopeId>,
    #[serde(default)]
    pub root_id: Option<EnvelopeId>,
    pub envelope_id: EnvelopeId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfilePost {
    pub author_pubkey: Pubkey,
    pub profile_topic_id: TopicId,
    pub published_topic_id: TopicId,
    pub object_id: EnvelopeId,
    pub created_at: i64,
    pub object_kind: String,
    pub content: String,
    pub attachments: Vec<AssetRef>,
    pub reply_to_object_id: Option<EnvelopeId>,
    pub root_id: Option<EnvelopeId>,
    pub envelope_id: EnvelopeId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KukuriProfileRepostEnvelopeContentV1 {
    pub author_pubkey: Pubkey,
    pub profile_topic_id: TopicId,
    pub published_topic_id: TopicId,
    pub object_id: EnvelopeId,
    pub created_at: i64,
    #[serde(default)]
    pub commentary: Option<String>,
    pub repost_of: RepostSourceSnapshotV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorProfileRepostDocV1 {
    pub author_pubkey: Pubkey,
    pub profile_topic_id: TopicId,
    pub published_topic_id: TopicId,
    pub object_id: EnvelopeId,
    pub created_at: i64,
    #[serde(default)]
    pub commentary: Option<String>,
    pub repost_of: RepostSourceSnapshotV1,
    pub envelope_id: EnvelopeId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileRepost {
    pub author_pubkey: Pubkey,
    pub profile_topic_id: TopicId,
    pub published_topic_id: TopicId,
    pub object_id: EnvelopeId,
    pub created_at: i64,
    #[serde(default)]
    pub commentary: Option<String>,
    pub repost_of: RepostSourceSnapshotV1,
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
#[serde(rename_all = "snake_case")]
pub enum ChannelAudienceKind {
    InviteOnly,
    FriendOnly,
    FriendPlus,
}

impl Default for ChannelAudienceKind {
    fn default() -> Self {
        Self::InviteOnly
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelSharingState {
    Open,
    Frozen,
}

impl Default for ChannelSharingState {
    fn default() -> Self {
        Self::Open
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePrivateChannelInput {
    pub topic_id: TopicId,
    pub label: String,
    #[serde(default)]
    pub audience_kind: ChannelAudienceKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivateChannelJoinMode {
    OwnerSeed,
    FriendOnlyGrant,
    FriendPlusShare,
    RotationRedeem,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivateChannelMetadataDocV1 {
    pub channel_id: ChannelId,
    pub topic_id: TopicId,
    pub label: String,
    pub creator_pubkey: Pubkey,
    pub created_at: i64,
    #[serde(default)]
    pub audience_kind: ChannelAudienceKind,
    #[serde(default)]
    pub owner_pubkey: Pubkey,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivateChannelPolicyDocV1 {
    pub channel_id: ChannelId,
    pub topic_id: TopicId,
    pub audience_kind: ChannelAudienceKind,
    pub owner_pubkey: Pubkey,
    pub epoch_id: String,
    pub sharing_state: ChannelSharingState,
    pub rotated_at: Option<i64>,
    #[serde(default)]
    pub previous_epoch_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivateChannelParticipantDocV1 {
    pub channel_id: ChannelId,
    pub topic_id: TopicId,
    pub epoch_id: String,
    pub participant_pubkey: Pubkey,
    pub joined_at: i64,
    pub is_owner: bool,
    #[serde(default)]
    pub join_mode: Option<PrivateChannelJoinMode>,
    #[serde(default)]
    pub sponsor_pubkey: Option<Pubkey>,
    #[serde(default)]
    pub share_token_id: Option<String>,
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
pub struct KukuriFriendOnlyGrantEnvelopeContentV1 {
    pub channel_id: ChannelId,
    pub topic_id: TopicId,
    pub channel_label: String,
    pub owner_pubkey: Pubkey,
    pub epoch_id: String,
    pub namespace_secret_hex: String,
    pub expires_at: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FriendOnlyGrantTokenV1 {
    pub envelope: KukuriEnvelope,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FriendOnlyGrantPreview {
    pub channel_id: ChannelId,
    pub topic_id: TopicId,
    pub channel_label: String,
    pub owner_pubkey: Pubkey,
    pub epoch_id: String,
    pub expires_at: Option<i64>,
    pub namespace_secret_hex: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KukuriFriendPlusShareEnvelopeContentV1 {
    pub channel_id: ChannelId,
    pub topic_id: TopicId,
    pub channel_label: String,
    pub owner_pubkey: Pubkey,
    pub sponsor_pubkey: Pubkey,
    pub epoch_id: String,
    pub namespace_secret_hex: String,
    pub expires_at: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FriendPlusShareTokenV1 {
    pub envelope: KukuriEnvelope,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FriendPlusSharePreview {
    pub channel_id: ChannelId,
    pub topic_id: TopicId,
    pub channel_label: String,
    pub owner_pubkey: Pubkey,
    pub sponsor_pubkey: Pubkey,
    pub epoch_id: String,
    pub expires_at: Option<i64>,
    pub namespace_secret_hex: String,
    pub share_token_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivateChannelRotationGrantPayloadV1 {
    pub channel_id: ChannelId,
    pub topic_id: TopicId,
    pub owner_pubkey: Pubkey,
    pub recipient_pubkey: Pubkey,
    pub old_epoch_id: String,
    pub new_epoch_id: String,
    pub new_namespace_secret_hex: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivateChannelRotationGrantDocV1 {
    pub channel_id: ChannelId,
    pub topic_id: TopicId,
    pub owner_pubkey: Pubkey,
    pub recipient_pubkey: Pubkey,
    pub old_epoch_id: String,
    pub new_epoch_id: String,
    pub nonce_hex: String,
    pub ciphertext_hex: String,
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
        let signature = Signature::from_str(self.sig.as_str()).context("invalid envelope sig")?;
        let public_key =
            XOnlyPublicKey::from_str(self.pubkey.as_str()).context("invalid envelope pubkey")?;
        SECP256K1
            .verify_schnorr(&signature, &digest, &public_key)
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
        if !matches!(self.kind.as_str(), "post" | "comment" | "repost") {
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
            repost_of: content.repost_of,
            status: ObjectStatus::Active,
            signature: self.sig.clone(),
        }))
    }

    pub fn reaction_content(&self) -> Result<Option<KukuriReactionEnvelopeContentV1>> {
        if self.kind != "reaction" {
            return Ok(None);
        }
        serde_json::from_str(self.content.as_str())
            .map(Some)
            .context("failed to parse reaction envelope content")
    }

    pub fn to_reaction_doc(&self) -> Result<Option<ReactionDocV1>> {
        let Some(content) = self.reaction_content()? else {
            return Ok(None);
        };
        Ok(Some(ReactionDocV1 {
            reaction_id: content.reaction_id,
            target_topic_id: content.target_topic_id,
            channel_id: content.channel_id,
            target_object_id: content.target_object_id,
            author_pubkey: self.pubkey.clone(),
            created_at: self.created_at,
            updated_at: self.created_at,
            reaction_key_kind: content.reaction_key_kind,
            normalized_reaction_key: content.normalized_reaction_key,
            emoji: content.emoji,
            custom_asset_id: content.custom_asset_id,
            custom_asset_snapshot: content.custom_asset_snapshot,
            status: content.status,
            envelope_id: self.id.clone(),
        }))
    }

    pub fn custom_reaction_asset_content(
        &self,
    ) -> Result<Option<KukuriCustomReactionAssetEnvelopeContentV1>> {
        if self.kind != "custom-reaction-asset" {
            return Ok(None);
        }
        serde_json::from_str(self.content.as_str())
            .map(Some)
            .context("failed to parse custom reaction asset envelope content")
    }

    pub fn to_custom_reaction_asset_doc(&self) -> Result<Option<CustomReactionAssetDocV1>> {
        let Some(content) = self.custom_reaction_asset_content()? else {
            return Ok(None);
        };
        Ok(Some(CustomReactionAssetDocV1 {
            asset_id: self.id.as_str().to_string(),
            author_pubkey: self.pubkey.clone(),
            blob_hash: content.blob_hash,
            mime: content.mime,
            bytes: content.bytes,
            width: content.width,
            height: content.height,
            created_at: self.created_at,
            updated_at: self.created_at,
            envelope_id: self.id.clone(),
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

#[allow(clippy::too_many_arguments)]
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
        repost_of: None,
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

pub fn build_repost_envelope(
    keys: &KukuriKeys,
    topic: &TopicId,
    repost_of: RepostSourceSnapshotV1,
    commentary: Option<&str>,
) -> Result<KukuriEnvelope> {
    if !matches!(repost_of.source_object_kind.as_str(), "post" | "comment") {
        bail!("repost source object kind must be post or comment");
    }
    let normalized_commentary = commentary
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let content = KukuriPostEnvelopeContentV1 {
        object_kind: "repost".into(),
        topic_id: topic.clone(),
        channel_id: None,
        payload_ref: PayloadRef::InlineText {
            text: normalized_commentary.clone().unwrap_or_default(),
        },
        attachments: Vec::new(),
        media_manifest_refs: Vec::new(),
        visibility: ObjectVisibility::Public,
        reply_to: None,
        root_id: None,
        repost_of: Some(repost_of.clone()),
    };
    sign_envelope_json(
        keys,
        "repost",
        vec![
            vec!["topic".into(), topic.as_str().into()],
            vec!["object".into(), "repost".into()],
            vec![
                "source_topic".into(),
                repost_of.source_topic_id.as_str().to_string(),
            ],
            vec![
                "source_object".into(),
                repost_of.source_object_id.as_str().to_string(),
            ],
            vec![
                "source_author".into(),
                repost_of.source_author_pubkey.as_str().to_string(),
            ],
        ],
        &content,
    )
}

pub fn normalize_reaction_emoji(value: &str) -> Option<String> {
    let normalized = value.trim();
    (!normalized.is_empty()).then(|| normalized.to_string())
}

pub fn deterministic_reaction_id(
    source_replica_id: &ReplicaId,
    target_object_id: &EnvelopeId,
    author_pubkey: &Pubkey,
    normalized_reaction_key: &str,
) -> EnvelopeId {
    EnvelopeId(hex::encode(sha256_digest(
        format!(
            "kukuri:reaction:{}:{}:{}:{}",
            source_replica_id.as_str(),
            target_object_id.as_str(),
            author_pubkey.as_str(),
            normalized_reaction_key.trim()
        )
        .as_bytes(),
    )))
}

pub fn build_reaction_envelope(
    keys: &KukuriKeys,
    target_topic_id: &TopicId,
    channel_id: Option<&ChannelId>,
    target_object_id: &EnvelopeId,
    reaction_key: ReactionKeyV1,
    reaction_id: &EnvelopeId,
    status: ObjectStatus,
) -> Result<KukuriEnvelope> {
    if !matches!(status, ObjectStatus::Active | ObjectStatus::Deleted) {
        bail!("reaction status must be active or deleted");
    }
    let author_pubkey = keys.public_key();
    let normalized_reaction_key = reaction_key.normalized_key()?;
    let (reaction_key_kind, emoji, custom_asset_id, custom_asset_snapshot) = match reaction_key {
        ReactionKeyV1::Emoji { emoji } => (
            ReactionKeyKind::Emoji,
            Some(
                normalize_reaction_emoji(&emoji)
                    .ok_or_else(|| anyhow!("reaction emoji must not be empty"))?,
            ),
            None,
            None,
        ),
        ReactionKeyV1::CustomAsset { asset_id, snapshot } => {
            if snapshot.owner_pubkey.as_str().trim().is_empty() {
                bail!("custom reaction snapshot owner pubkey must not be empty");
            }
            if snapshot.mime.trim().is_empty() {
                bail!("custom reaction snapshot mime must not be empty");
            }
            (
                ReactionKeyKind::CustomAsset,
                None,
                Some(asset_id),
                Some(snapshot),
            )
        }
    };
    let created_at = now_timestamp_millis()?;
    sign_envelope_at(
        keys,
        "reaction",
        vec![
            vec!["topic".into(), target_topic_id.as_str().into()],
            vec!["object".into(), "reaction".into()],
            vec!["target_object".into(), target_object_id.as_str().to_string()],
            vec!["reaction_id".into(), reaction_id.as_str().to_string()],
            vec![
                "reaction_key".into(),
                normalized_reaction_key.clone(),
            ],
            vec!["author".into(), author_pubkey.as_str().to_string()],
        ]
        .into_iter()
        .chain(channel_id.into_iter().map(|channel_id| {
            vec!["channel".into(), channel_id.as_str().to_string()]
        }))
        .collect(),
        serde_json::to_string(&KukuriReactionEnvelopeContentV1 {
            reaction_id: reaction_id.clone(),
            target_topic_id: target_topic_id.clone(),
            channel_id: channel_id.cloned(),
            target_object_id: target_object_id.clone(),
            reaction_key_kind,
            normalized_reaction_key,
            emoji,
            custom_asset_id,
            custom_asset_snapshot,
            status,
        })?,
        created_at,
    )
}

pub fn build_custom_reaction_asset_envelope(
    keys: &KukuriKeys,
    blob_hash: BlobHash,
    mime: String,
    bytes: u64,
    width: u32,
    height: u32,
) -> Result<KukuriEnvelope> {
    let author_pubkey = keys.public_key();
    if mime.trim().is_empty() {
        bail!("custom reaction asset mime must not be empty");
    }
    if width == 0 || height == 0 {
        bail!("custom reaction asset dimensions must be non-zero");
    }
    let created_at = now_timestamp_millis()?;
    sign_envelope_at(
        keys,
        "custom-reaction-asset",
        vec![
            vec!["author".into(), author_pubkey.as_str().to_string()],
            vec!["object".into(), "custom-reaction-asset".into()],
            vec!["blob_hash".into(), blob_hash.as_str().to_string()],
        ],
        serde_json::to_string(&KukuriCustomReactionAssetEnvelopeContentV1 {
            author_pubkey,
            blob_hash,
            mime,
            bytes,
            width,
            height,
        })?,
        created_at,
    )
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

pub fn build_profile_post_envelope(
    keys: &KukuriKeys,
    content: &KukuriProfilePostEnvelopeContentV1,
) -> Result<KukuriEnvelope> {
    let author_pubkey = keys.public_key();
    if content.author_pubkey != author_pubkey {
        bail!("profile post author pubkey must match signer");
    }
    if content.profile_topic_id != author_profile_topic_id(content.author_pubkey.as_str()) {
        bail!("profile post topic id must match author profile topic");
    }
    if !matches!(content.object_kind.as_str(), "post" | "comment") {
        bail!("profile post object kind must be post or comment");
    }
    let created_at = now_timestamp_millis()?;
    let encoded = serde_json::to_string(content).context("failed to encode envelope content")?;
    sign_envelope_at(
        keys,
        "profile-post",
        vec![
            vec!["author".into(), content.author_pubkey.as_str().to_string()],
            vec!["object".into(), "profile-post".into()],
            vec![
                "published_topic".into(),
                content.published_topic_id.as_str().to_string(),
            ],
            vec!["post".into(), content.object_id.as_str().to_string()],
        ],
        encoded,
        created_at,
    )
}

pub fn build_profile_repost_envelope(
    keys: &KukuriKeys,
    content: &KukuriProfileRepostEnvelopeContentV1,
) -> Result<KukuriEnvelope> {
    let author_pubkey = keys.public_key();
    if content.author_pubkey != author_pubkey {
        bail!("profile repost author pubkey must match signer");
    }
    if content.profile_topic_id != author_profile_topic_id(content.author_pubkey.as_str()) {
        bail!("profile repost topic id must match author profile topic");
    }
    if !matches!(
        content.repost_of.source_object_kind.as_str(),
        "post" | "comment"
    ) {
        bail!("profile repost source object kind must be post or comment");
    }
    let created_at = now_timestamp_millis()?;
    let encoded =
        serde_json::to_string(content).context("failed to encode profile repost content")?;
    sign_envelope_at(
        keys,
        "profile-repost",
        vec![
            vec!["author".into(), content.author_pubkey.as_str().to_string()],
            vec!["object".into(), "profile-repost".into()],
            vec![
                "published_topic".into(),
                content.published_topic_id.as_str().to_string(),
            ],
            vec!["repost".into(), content.object_id.as_str().to_string()],
            vec![
                "source_topic".into(),
                content.repost_of.source_topic_id.as_str().to_string(),
            ],
            vec![
                "source_object".into(),
                content.repost_of.source_object_id.as_str().to_string(),
            ],
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

pub fn build_friend_only_grant_token(
    keys: &KukuriKeys,
    topic: &TopicId,
    channel_id: &ChannelId,
    channel_label: &str,
    epoch_id: &str,
    namespace_secret_hex: &str,
    expires_at: Option<i64>,
) -> Result<String> {
    let owner_pubkey = keys.public_key();
    let token = FriendOnlyGrantTokenV1 {
        envelope: sign_envelope_json(
            keys,
            "channel-friend-grant",
            vec![
                vec!["topic".into(), topic.as_str().to_string()],
                vec!["object".into(), "channel-friend-grant".into()],
                vec!["channel".into(), channel_id.as_str().to_string()],
                vec!["epoch".into(), epoch_id.trim().to_string()],
            ],
            &KukuriFriendOnlyGrantEnvelopeContentV1 {
                channel_id: channel_id.clone(),
                topic_id: topic.clone(),
                channel_label: channel_label.trim().to_string(),
                owner_pubkey,
                epoch_id: epoch_id.trim().to_string(),
                namespace_secret_hex: namespace_secret_hex.trim().to_string(),
                expires_at,
            },
        )?,
    };
    serde_json::to_string(&token).context("failed to encode friend-only grant token")
}

#[allow(clippy::too_many_arguments)]
pub fn build_friend_plus_share_token(
    keys: &KukuriKeys,
    topic: &TopicId,
    channel_id: &ChannelId,
    channel_label: &str,
    owner_pubkey: &Pubkey,
    epoch_id: &str,
    namespace_secret_hex: &str,
    expires_at: Option<i64>,
) -> Result<String> {
    let sponsor_pubkey = keys.public_key();
    let token = FriendPlusShareTokenV1 {
        envelope: sign_envelope_json(
            keys,
            "channel-share",
            vec![
                vec!["topic".into(), topic.as_str().to_string()],
                vec!["object".into(), "channel-share".into()],
                vec!["channel".into(), channel_id.as_str().to_string()],
                vec!["epoch".into(), epoch_id.trim().to_string()],
                vec!["owner".into(), owner_pubkey.as_str().to_string()],
            ],
            &KukuriFriendPlusShareEnvelopeContentV1 {
                channel_id: channel_id.clone(),
                topic_id: topic.clone(),
                channel_label: channel_label.trim().to_string(),
                owner_pubkey: owner_pubkey.clone(),
                sponsor_pubkey,
                epoch_id: epoch_id.trim().to_string(),
                namespace_secret_hex: namespace_secret_hex.trim().to_string(),
                expires_at,
            },
        )?,
    };
    serde_json::to_string(&token).context("failed to encode friend-plus share token")
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

pub fn build_private_channel_policy_envelope(
    keys: &KukuriKeys,
    doc: &PrivateChannelPolicyDocV1,
) -> Result<KukuriEnvelope> {
    if keys.public_key() != doc.owner_pubkey {
        bail!("channel policy owner pubkey must match signer");
    }
    let created_at = now_timestamp_millis()?;
    let encoded = serde_json::to_string(doc).context("failed to encode channel policy doc")?;
    sign_envelope_at(
        keys,
        "channel-policy",
        vec![
            vec!["topic".into(), doc.topic_id.as_str().to_string()],
            vec!["channel".into(), doc.channel_id.as_str().to_string()],
            vec!["epoch".into(), doc.epoch_id.clone()],
            vec!["object".into(), "channel-policy".into()],
        ],
        encoded,
        created_at,
    )
}

pub fn parse_private_channel_policy(
    envelope: &KukuriEnvelope,
) -> Result<Option<PrivateChannelPolicyDocV1>> {
    if envelope.kind != "channel-policy" {
        return Ok(None);
    }
    let doc: PrivateChannelPolicyDocV1 =
        serde_json::from_str(&envelope.content).context("failed to parse channel policy")?;
    validate_pubkey(doc.owner_pubkey.as_str()).context("invalid channel policy owner pubkey")?;
    if envelope.pubkey != doc.owner_pubkey {
        bail!("channel policy owner pubkey must match envelope signer");
    }
    if doc.epoch_id.trim().is_empty() {
        bail!("channel policy epoch id is required");
    }
    if doc
        .previous_epoch_id
        .as_ref()
        .is_some_and(|value| value.trim().is_empty())
    {
        bail!("channel policy previous epoch id must not be empty");
    }
    Ok(Some(doc))
}

pub fn build_private_channel_participant_envelope(
    keys: &KukuriKeys,
    doc: &PrivateChannelParticipantDocV1,
) -> Result<KukuriEnvelope> {
    if keys.public_key() != doc.participant_pubkey {
        bail!("channel participant pubkey must match signer");
    }
    let created_at = now_timestamp_millis()?;
    let encoded = serde_json::to_string(doc).context("failed to encode channel participant doc")?;
    sign_envelope_at(
        keys,
        "channel-participant",
        vec![
            vec!["topic".into(), doc.topic_id.as_str().to_string()],
            vec!["channel".into(), doc.channel_id.as_str().to_string()],
            vec!["epoch".into(), doc.epoch_id.clone()],
            vec![
                "participant".into(),
                doc.participant_pubkey.as_str().to_string(),
            ],
            vec!["object".into(), "channel-participant".into()],
        ],
        encoded,
        created_at,
    )
}

pub fn parse_private_channel_participant(
    envelope: &KukuriEnvelope,
) -> Result<Option<PrivateChannelParticipantDocV1>> {
    if envelope.kind != "channel-participant" {
        return Ok(None);
    }
    let doc: PrivateChannelParticipantDocV1 =
        serde_json::from_str(&envelope.content).context("failed to parse channel participant")?;
    validate_pubkey(doc.participant_pubkey.as_str())
        .context("invalid channel participant pubkey")?;
    if envelope.pubkey != doc.participant_pubkey {
        bail!("channel participant pubkey must match envelope signer");
    }
    if doc.epoch_id.trim().is_empty() {
        bail!("channel participant epoch id is required");
    }
    if let Some(sponsor_pubkey) = doc.sponsor_pubkey.as_ref() {
        validate_pubkey(sponsor_pubkey.as_str())
            .context("invalid channel participant sponsor pubkey")?;
    }
    if doc
        .share_token_id
        .as_ref()
        .is_some_and(|value| value.trim().is_empty())
    {
        bail!("channel participant share token id must not be empty");
    }
    Ok(Some(doc))
}

pub fn parse_friend_only_grant_token(token: &str) -> Result<FriendOnlyGrantPreview> {
    let token: FriendOnlyGrantTokenV1 =
        serde_json::from_str(token).context("failed to parse friend-only grant token")?;
    token.envelope.verify()?;
    if token.envelope.kind != "channel-friend-grant" {
        bail!("grant envelope kind must be channel-friend-grant");
    }
    let content: KukuriFriendOnlyGrantEnvelopeContentV1 =
        serde_json::from_str(token.envelope.content.as_str())
            .context("failed to decode friend-only grant content")?;
    validate_pubkey(content.owner_pubkey.as_str()).context("invalid friend-only grant owner")?;
    if token.envelope.pubkey != content.owner_pubkey {
        bail!("friend-only grant owner pubkey must match envelope signer");
    }
    if content.channel_label.trim().is_empty() {
        bail!("friend-only grant label is required");
    }
    if content.epoch_id.trim().is_empty() {
        bail!("friend-only grant epoch id is required");
    }
    let secret_bytes =
        hex::decode(content.namespace_secret_hex.trim()).context("invalid grant secret hex")?;
    if secret_bytes.len() != 32 {
        bail!("grant secret must be 32 bytes");
    }
    if let Some(expires_at) = content.expires_at
        && expires_at < now_timestamp_millis()?
    {
        bail!("friend-only grant has expired");
    }
    Ok(FriendOnlyGrantPreview {
        channel_id: content.channel_id,
        topic_id: content.topic_id,
        channel_label: content.channel_label,
        owner_pubkey: content.owner_pubkey,
        epoch_id: content.epoch_id,
        expires_at: content.expires_at,
        namespace_secret_hex: content.namespace_secret_hex,
    })
}

pub fn parse_friend_plus_share_token(token: &str) -> Result<FriendPlusSharePreview> {
    let token: FriendPlusShareTokenV1 =
        serde_json::from_str(token).context("failed to parse friend-plus share token")?;
    token.envelope.verify()?;
    if token.envelope.kind != "channel-share" {
        bail!("share envelope kind must be channel-share");
    }
    let content: KukuriFriendPlusShareEnvelopeContentV1 =
        serde_json::from_str(token.envelope.content.as_str())
            .context("failed to decode friend-plus share content")?;
    validate_pubkey(content.owner_pubkey.as_str()).context("invalid friend-plus share owner")?;
    validate_pubkey(content.sponsor_pubkey.as_str())
        .context("invalid friend-plus share sponsor")?;
    if token.envelope.pubkey != content.sponsor_pubkey {
        bail!("friend-plus share sponsor pubkey must match envelope signer");
    }
    if content.channel_label.trim().is_empty() {
        bail!("friend-plus share label is required");
    }
    if content.epoch_id.trim().is_empty() {
        bail!("friend-plus share epoch id is required");
    }
    validate_private_channel_secret_hex(
        content.namespace_secret_hex.as_str(),
        "friend-plus share secret",
    )?;
    if let Some(expires_at) = content.expires_at
        && expires_at < now_timestamp_millis()?
    {
        bail!("friend-plus share has expired");
    }
    Ok(FriendPlusSharePreview {
        channel_id: content.channel_id,
        topic_id: content.topic_id,
        channel_label: content.channel_label,
        owner_pubkey: content.owner_pubkey,
        sponsor_pubkey: content.sponsor_pubkey,
        epoch_id: content.epoch_id,
        expires_at: content.expires_at,
        namespace_secret_hex: content.namespace_secret_hex,
        share_token_id: token.envelope.id.as_str().to_string(),
    })
}

pub fn encrypt_private_channel_rotation_grant(
    owner_keys: &KukuriKeys,
    payload: &PrivateChannelRotationGrantPayloadV1,
) -> Result<PrivateChannelRotationGrantDocV1> {
    if owner_keys.public_key() != payload.owner_pubkey {
        bail!("channel rotation grant owner pubkey must match signer");
    }
    validate_pubkey(payload.recipient_pubkey.as_str())
        .context("invalid channel rotation grant recipient pubkey")?;
    if payload.old_epoch_id.trim().is_empty() {
        bail!("channel rotation grant old epoch id is required");
    }
    if payload.new_epoch_id.trim().is_empty() {
        bail!("channel rotation grant new epoch id is required");
    }
    validate_private_channel_secret_hex(
        payload.new_namespace_secret_hex.as_str(),
        "channel rotation grant secret",
    )?;
    let plaintext =
        serde_json::to_vec(payload).context("failed to encode channel rotation grant payload")?;
    let mut nonce = [0u8; 24];
    rng().fill_bytes(&mut nonce);
    let cipher = XChaCha20Poly1305::new_from_slice(
        derive_rotation_grant_key(owner_keys, &payload.recipient_pubkey, payload)?.as_slice(),
    )
    .context("failed to initialize rotation grant cipher")?;
    let ciphertext = cipher
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: plaintext.as_slice(),
                aad: rotation_grant_aad(payload).as_bytes(),
            },
        )
        .map_err(|_| anyhow::anyhow!("failed to encrypt channel rotation grant"))?;
    Ok(PrivateChannelRotationGrantDocV1 {
        channel_id: payload.channel_id.clone(),
        topic_id: payload.topic_id.clone(),
        owner_pubkey: payload.owner_pubkey.clone(),
        recipient_pubkey: payload.recipient_pubkey.clone(),
        old_epoch_id: payload.old_epoch_id.clone(),
        new_epoch_id: payload.new_epoch_id.clone(),
        nonce_hex: hex::encode(nonce),
        ciphertext_hex: hex::encode(ciphertext),
    })
}

pub fn decrypt_private_channel_rotation_grant(
    local_keys: &KukuriKeys,
    doc: &PrivateChannelRotationGrantDocV1,
) -> Result<PrivateChannelRotationGrantPayloadV1> {
    if local_keys.public_key() != doc.recipient_pubkey {
        bail!("channel rotation grant recipient pubkey must match decrypting author");
    }
    let nonce =
        hex::decode(doc.nonce_hex.trim()).context("invalid channel rotation grant nonce")?;
    if nonce.len() != 24 {
        bail!("channel rotation grant nonce must be 24 bytes");
    }
    let ciphertext = hex::decode(doc.ciphertext_hex.trim())
        .context("invalid channel rotation grant ciphertext")?;
    let payload_stub = PrivateChannelRotationGrantPayloadV1 {
        channel_id: doc.channel_id.clone(),
        topic_id: doc.topic_id.clone(),
        owner_pubkey: doc.owner_pubkey.clone(),
        recipient_pubkey: doc.recipient_pubkey.clone(),
        old_epoch_id: doc.old_epoch_id.clone(),
        new_epoch_id: doc.new_epoch_id.clone(),
        new_namespace_secret_hex: String::new(),
    };
    let cipher = XChaCha20Poly1305::new_from_slice(
        derive_rotation_grant_key(local_keys, &doc.owner_pubkey, &payload_stub)?.as_slice(),
    )
    .context("failed to initialize rotation grant cipher")?;
    let plaintext = cipher
        .decrypt(
            XNonce::from_slice(nonce.as_slice()),
            Payload {
                msg: ciphertext.as_slice(),
                aad: rotation_grant_aad(&payload_stub).as_bytes(),
            },
        )
        .map_err(|_| anyhow::anyhow!("failed to decrypt channel rotation grant"))?;
    let payload: PrivateChannelRotationGrantPayloadV1 = serde_json::from_slice(&plaintext)
        .context("failed to decode channel rotation grant payload")?;
    if payload.channel_id != doc.channel_id || payload.topic_id != doc.topic_id {
        bail!("channel rotation grant payload does not match doc identity");
    }
    if payload.owner_pubkey != doc.owner_pubkey || payload.recipient_pubkey != doc.recipient_pubkey
    {
        bail!("channel rotation grant payload does not match doc recipients");
    }
    if payload.old_epoch_id != doc.old_epoch_id || payload.new_epoch_id != doc.new_epoch_id {
        bail!("channel rotation grant payload does not match doc epochs");
    }
    validate_private_channel_secret_hex(
        payload.new_namespace_secret_hex.as_str(),
        "channel rotation grant secret",
    )?;
    Ok(payload)
}

pub fn build_private_channel_rotation_grant_envelope(
    owner_keys: &KukuriKeys,
    doc: &PrivateChannelRotationGrantDocV1,
) -> Result<KukuriEnvelope> {
    if owner_keys.public_key() != doc.owner_pubkey {
        bail!("channel rotation grant owner pubkey must match signer");
    }
    let created_at = now_timestamp_millis()?;
    let encoded =
        serde_json::to_string(doc).context("failed to encode channel rotation grant doc")?;
    sign_envelope_at(
        owner_keys,
        "channel-rotation-grant",
        vec![
            vec!["topic".into(), doc.topic_id.as_str().to_string()],
            vec!["channel".into(), doc.channel_id.as_str().to_string()],
            vec!["epoch".into(), doc.old_epoch_id.clone()],
            vec![
                "recipient".into(),
                doc.recipient_pubkey.as_str().to_string(),
            ],
            vec!["object".into(), "channel-rotation-grant".into()],
        ],
        encoded,
        created_at,
    )
}

pub fn parse_private_channel_rotation_grant(
    envelope: &KukuriEnvelope,
) -> Result<Option<PrivateChannelRotationGrantDocV1>> {
    if envelope.kind != "channel-rotation-grant" {
        return Ok(None);
    }
    let doc: PrivateChannelRotationGrantDocV1 = serde_json::from_str(&envelope.content)
        .context("failed to parse channel rotation grant")?;
    validate_pubkey(doc.owner_pubkey.as_str()).context("invalid channel rotation grant owner")?;
    validate_pubkey(doc.recipient_pubkey.as_str())
        .context("invalid channel rotation grant recipient")?;
    if envelope.pubkey != doc.owner_pubkey {
        bail!("channel rotation grant owner pubkey must match envelope signer");
    }
    if doc.old_epoch_id.trim().is_empty() {
        bail!("channel rotation grant old epoch id is required");
    }
    if doc.new_epoch_id.trim().is_empty() {
        bail!("channel rotation grant new epoch id is required");
    }
    if doc.old_epoch_id == doc.new_epoch_id {
        bail!("channel rotation grant must rotate to a new epoch");
    }
    let nonce =
        hex::decode(doc.nonce_hex.trim()).context("invalid channel rotation grant nonce")?;
    if nonce.len() != 24 {
        bail!("channel rotation grant nonce must be 24 bytes");
    }
    let _ = hex::decode(doc.ciphertext_hex.trim())
        .context("invalid channel rotation grant ciphertext")?;
    Ok(Some(doc))
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

pub fn parse_profile_post(envelope: &KukuriEnvelope) -> Result<Option<ProfilePost>> {
    if envelope.kind != "profile-post" {
        return Ok(None);
    }

    let content: KukuriProfilePostEnvelopeContentV1 =
        serde_json::from_str(&envelope.content).context("failed to parse profile post envelope")?;
    validate_pubkey(content.author_pubkey.as_str())
        .context("invalid profile post author pubkey")?;
    if content.author_pubkey != envelope.pubkey {
        bail!("profile post author pubkey must match envelope signer");
    }
    if content.profile_topic_id != author_profile_topic_id(content.author_pubkey.as_str()) {
        bail!("profile post topic id must match author profile topic");
    }
    if !matches!(content.object_kind.as_str(), "post" | "comment") {
        bail!("profile post object kind must be post or comment");
    }

    Ok(Some(ProfilePost {
        author_pubkey: content.author_pubkey,
        profile_topic_id: content.profile_topic_id,
        published_topic_id: content.published_topic_id,
        object_id: content.object_id,
        created_at: content.created_at,
        object_kind: content.object_kind,
        content: content.content,
        attachments: content.attachments,
        reply_to_object_id: content.reply_to_object_id,
        root_id: content.root_id,
        envelope_id: envelope.id.clone(),
    }))
}

pub fn parse_profile_repost(envelope: &KukuriEnvelope) -> Result<Option<ProfileRepost>> {
    if envelope.kind != "profile-repost" {
        return Ok(None);
    }

    let content: KukuriProfileRepostEnvelopeContentV1 = serde_json::from_str(&envelope.content)
        .context("failed to parse profile repost envelope")?;
    validate_pubkey(content.author_pubkey.as_str())
        .context("invalid profile repost author pubkey")?;
    validate_pubkey(content.repost_of.source_author_pubkey.as_str())
        .context("invalid profile repost source author pubkey")?;
    if content.author_pubkey != envelope.pubkey {
        bail!("profile repost author pubkey must match envelope signer");
    }
    if content.profile_topic_id != author_profile_topic_id(content.author_pubkey.as_str()) {
        bail!("profile repost topic id must match author profile topic");
    }
    if !matches!(
        content.repost_of.source_object_kind.as_str(),
        "post" | "comment"
    ) {
        bail!("profile repost source object kind must be post or comment");
    }

    Ok(Some(ProfileRepost {
        author_pubkey: content.author_pubkey,
        profile_topic_id: content.profile_topic_id,
        published_topic_id: content.published_topic_id,
        object_id: content.object_id,
        created_at: content.created_at,
        commentary: content.commentary,
        repost_of: content.repost_of,
        envelope_id: envelope.id.clone(),
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

pub fn parse_reaction(envelope: &KukuriEnvelope) -> Result<Option<ReactionDocV1>> {
    if envelope.kind != "reaction" {
        return Ok(None);
    }
    let reaction = envelope
        .to_reaction_doc()?
        .ok_or_else(|| anyhow!("failed to parse reaction doc"))?;
    validate_pubkey(reaction.author_pubkey.as_str()).context("invalid reaction author pubkey")?;
    match reaction.reaction_key_kind {
        ReactionKeyKind::Emoji => {
            let emoji = reaction
                .emoji
                .as_deref()
                .and_then(normalize_reaction_emoji)
                .ok_or_else(|| anyhow!("reaction emoji must not be empty"))?;
            if reaction.normalized_reaction_key != format!("emoji:{emoji}") {
                bail!("reaction normalized key does not match emoji value");
            }
        }
        ReactionKeyKind::CustomAsset => {
            let asset_id = reaction
                .custom_asset_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| anyhow!("reaction custom asset id must not be empty"))?;
            let snapshot = reaction
                .custom_asset_snapshot
                .as_ref()
                .ok_or_else(|| anyhow!("reaction custom asset snapshot is missing"))?;
            if snapshot.asset_id != asset_id {
                bail!("reaction custom asset snapshot id must match custom asset id");
            }
            if reaction.normalized_reaction_key != format!("custom_asset:{asset_id}") {
                bail!("reaction normalized key does not match custom asset value");
            }
        }
    }
    if !matches!(reaction.status, ObjectStatus::Active | ObjectStatus::Deleted) {
        bail!("reaction status must be active or deleted");
    }
    Ok(Some(reaction))
}

pub fn parse_custom_reaction_asset(
    envelope: &KukuriEnvelope,
) -> Result<Option<CustomReactionAssetDocV1>> {
    if envelope.kind != "custom-reaction-asset" {
        return Ok(None);
    }
    let asset = envelope
        .to_custom_reaction_asset_doc()?
        .ok_or_else(|| anyhow!("failed to parse custom reaction asset doc"))?;
    validate_pubkey(asset.author_pubkey.as_str())
        .context("invalid custom reaction asset author pubkey")?;
    if asset.author_pubkey != envelope.pubkey {
        bail!("custom reaction asset author pubkey must match envelope signer");
    }
    if asset.mime.trim().is_empty() {
        bail!("custom reaction asset mime must not be empty");
    }
    if asset.width == 0 || asset.height == 0 {
        bail!("custom reaction asset dimensions must be non-zero");
    }
    Ok(Some(asset))
}

fn validate_private_channel_secret_hex(value: &str, label: &str) -> Result<()> {
    let secret_bytes = hex::decode(value.trim()).with_context(|| format!("invalid {label} hex"))?;
    if secret_bytes.len() != 32 {
        bail!("{label} must be 32 bytes");
    }
    Ok(())
}

fn rotation_grant_aad(payload: &PrivateChannelRotationGrantPayloadV1) -> String {
    format!(
        "kukuri:rotation-grant:{}:{}:{}:{}:{}",
        payload.channel_id.as_str(),
        payload.topic_id.as_str(),
        payload.owner_pubkey.as_str(),
        payload.recipient_pubkey.as_str(),
        payload.new_epoch_id
    )
}

fn derive_rotation_grant_key(
    local_keys: &KukuriKeys,
    remote_pubkey: &Pubkey,
    payload: &PrivateChannelRotationGrantPayloadV1,
) -> Result<[u8; 32]> {
    let remote_xonly = XOnlyPublicKey::from_str(remote_pubkey.as_str())
        .context("invalid remote x-only public key")?;
    let remote_public = PublicKey::from_x_only_public_key(remote_xonly, Parity::Even);
    let keypair = Keypair::from_secret_key(SECP256K1, &local_keys.secret_key);
    let (_, parity) = keypair.x_only_public_key();
    let local_secret = if parity == Parity::Odd {
        local_keys.secret_key.negate()
    } else {
        local_keys.secret_key
    };
    let shared = SharedSecret::new(&remote_public, &local_secret);
    let hkdf = Hkdf::<Sha256>::new(
        Some(b"kukuri/private-channel/rotation-grant"),
        shared.secret_bytes().as_slice(),
    );
    let mut key = [0u8; 32];
    hkdf.expand(rotation_grant_aad(payload).as_bytes(), &mut key)
        .map_err(|_| anyhow::anyhow!("failed to derive channel rotation grant key"))?;
    Ok(key)
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
    let sig = keys.sign_schnorr(&digest).to_string();
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
    fn profile_post_envelope_roundtrip() {
        let keys = generate_keys();
        let author_pubkey = keys.public_key();
        let envelope = build_profile_post_envelope(
            &keys,
            &KukuriProfilePostEnvelopeContentV1 {
                author_pubkey: author_pubkey.clone(),
                profile_topic_id: author_profile_topic_id(author_pubkey.as_str()),
                published_topic_id: TopicId::new("kukuri:topic:demo"),
                object_id: EnvelopeId::from("post-1"),
                created_at: 42,
                object_kind: "comment".into(),
                content: "hello profile topic".into(),
                attachments: vec![AssetRef {
                    hash: BlobHash::new("hash-1"),
                    mime: "image/png".into(),
                    bytes: 12,
                    role: AssetRole::ImageOriginal,
                }],
                reply_to_object_id: Some(EnvelopeId::from("root-1")),
                root_id: Some(EnvelopeId::from("root-1")),
            },
        )
        .expect("profile post envelope");

        envelope.verify().expect("signature verification");
        let profile_post = parse_profile_post(&envelope)
            .expect("parse profile post")
            .expect("profile post");
        assert_eq!(profile_post.author_pubkey, author_pubkey);
        assert_eq!(
            profile_post.profile_topic_id,
            author_profile_topic_id(author_pubkey.as_str())
        );
        assert_eq!(
            profile_post.published_topic_id.as_str(),
            "kukuri:topic:demo"
        );
        assert_eq!(profile_post.object_id.as_str(), "post-1");
        assert_eq!(profile_post.created_at, 42);
        assert_eq!(profile_post.object_kind, "comment");
        assert_eq!(profile_post.content, "hello profile topic");
        assert_eq!(profile_post.attachments.len(), 1);
        assert_eq!(
            profile_post
                .reply_to_object_id
                .as_ref()
                .map(EnvelopeId::as_str),
            Some("root-1")
        );
        assert_eq!(
            profile_post.root_id.as_ref().map(EnvelopeId::as_str),
            Some("root-1")
        );
        assert_eq!(profile_post.envelope_id, envelope.id);
    }

    #[test]
    fn repost_envelope_roundtrip() {
        let keys = generate_keys();
        let envelope = build_repost_envelope(
            &keys,
            &TopicId::new("kukuri:topic:target"),
            RepostSourceSnapshotV1 {
                source_object_id: EnvelopeId::from("source-1"),
                source_topic_id: TopicId::new("kukuri:topic:source"),
                source_author_pubkey: generate_keys().public_key(),
                source_object_kind: "comment".into(),
                content: "quoted source".into(),
                attachments: vec![AssetRef {
                    hash: BlobHash::new("hash-1"),
                    mime: "image/png".into(),
                    bytes: 24,
                    role: AssetRole::ImageOriginal,
                }],
                reply_to_object_id: Some(EnvelopeId::from("root-1")),
                root_id: Some(EnvelopeId::from("root-1")),
            },
            Some("quote commentary"),
        )
        .expect("repost envelope");

        envelope.verify().expect("signature verification");
        let repost = envelope
            .to_post_object()
            .expect("parse repost")
            .expect("repost object");
        assert_eq!(repost.object_kind, "repost");
        assert_eq!(repost.topic_id.as_str(), "kukuri:topic:target");
        assert_eq!(
            repost
                .repost_of
                .as_ref()
                .map(|value| value.source_topic_id.as_str()),
            Some("kukuri:topic:source")
        );
        assert_eq!(
            match repost.payload_ref {
                PayloadRef::InlineText { text } => text,
                PayloadRef::BlobText { .. } => String::new(),
            },
            "quote commentary"
        );
    }

    #[test]
    fn profile_repost_envelope_roundtrip() {
        let keys = generate_keys();
        let author_pubkey = keys.public_key();
        let envelope = build_profile_repost_envelope(
            &keys,
            &KukuriProfileRepostEnvelopeContentV1 {
                author_pubkey: author_pubkey.clone(),
                profile_topic_id: author_profile_topic_id(author_pubkey.as_str()),
                published_topic_id: TopicId::new("kukuri:topic:target"),
                object_id: EnvelopeId::from("repost-1"),
                created_at: 55,
                commentary: Some("quote commentary".into()),
                repost_of: RepostSourceSnapshotV1 {
                    source_object_id: EnvelopeId::from("source-1"),
                    source_topic_id: TopicId::new("kukuri:topic:source"),
                    source_author_pubkey: generate_keys().public_key(),
                    source_object_kind: "post".into(),
                    content: "source content".into(),
                    attachments: Vec::new(),
                    reply_to_object_id: None,
                    root_id: Some(EnvelopeId::from("source-1")),
                },
            },
        )
        .expect("profile repost envelope");

        envelope.verify().expect("signature verification");
        let profile_repost = parse_profile_repost(&envelope)
            .expect("parse profile repost")
            .expect("profile repost");
        assert_eq!(profile_repost.author_pubkey, author_pubkey);
        assert_eq!(
            profile_repost.published_topic_id.as_str(),
            "kukuri:topic:target"
        );
        assert_eq!(profile_repost.object_id.as_str(), "repost-1");
        assert_eq!(
            profile_repost.commentary.as_deref(),
            Some("quote commentary")
        );
        assert_eq!(
            profile_repost.repost_of.source_topic_id.as_str(),
            "kukuri:topic:source"
        );
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

    #[test]
    fn reaction_envelope_roundtrip_for_emoji() {
        let keys = generate_keys();
        let topic = TopicId::new("kukuri:topic:demo");
        let target_object_id = EnvelopeId::from("post-1");
        let reaction_id = deterministic_reaction_id(
            &ReplicaId::new("replica-1"),
            &target_object_id,
            &keys.public_key(),
            "emoji:👍",
        );
        let envelope = build_reaction_envelope(
            &keys,
            &topic,
            None,
            &target_object_id,
            ReactionKeyV1::Emoji { emoji: " 👍 ".into() },
            &reaction_id,
            ObjectStatus::Active,
        )
        .expect("reaction envelope");

        envelope.verify().expect("signature verification");
        let reaction = parse_reaction(&envelope)
            .expect("parse reaction")
            .expect("reaction");
        assert_eq!(reaction.reaction_id, reaction_id);
        assert_eq!(reaction.target_topic_id, topic);
        assert_eq!(reaction.target_object_id, target_object_id);
        assert_eq!(reaction.reaction_key_kind, ReactionKeyKind::Emoji);
        assert_eq!(reaction.emoji.as_deref(), Some("👍"));
        assert_eq!(reaction.normalized_reaction_key, "emoji:👍");
        assert_eq!(reaction.status, ObjectStatus::Active);
    }

    #[test]
    fn custom_reaction_asset_roundtrip_and_reaction_id_stability() {
        let keys = generate_keys();
        let envelope = build_custom_reaction_asset_envelope(
            &keys,
            BlobHash::new("blob-asset-1"),
            "image/png".into(),
            128,
            128,
            128,
        )
        .expect("asset envelope");

        envelope.verify().expect("signature verification");
        let asset = parse_custom_reaction_asset(&envelope)
            .expect("parse asset")
            .expect("asset");
        assert_eq!(asset.asset_id, envelope.id.0);
        assert_eq!(asset.mime, "image/png");

        let reaction_key = ReactionKeyV1::CustomAsset {
            asset_id: asset.asset_id.clone(),
            snapshot: CustomReactionAssetSnapshotV1 {
                asset_id: asset.asset_id.clone(),
                owner_pubkey: asset.author_pubkey.clone(),
                blob_hash: asset.blob_hash.clone(),
                mime: asset.mime.clone(),
                bytes: asset.bytes,
                width: asset.width,
                height: asset.height,
            },
        };
        let normalized = reaction_key.normalized_key().expect("normalized key");
        let first = deterministic_reaction_id(
            &ReplicaId::new("replica-a"),
            &EnvelopeId::from("post-1"),
            &keys.public_key(),
            normalized.as_str(),
        );
        let second = deterministic_reaction_id(
            &ReplicaId::new("replica-a"),
            &EnvelopeId::from("post-1"),
            &keys.public_key(),
            normalized.as_str(),
        );
        let different = deterministic_reaction_id(
            &ReplicaId::new("replica-a"),
            &EnvelopeId::from("post-1"),
            &keys.public_key(),
            "emoji:🔥",
        );
        assert_eq!(first, second);
        assert_ne!(first, different);
    }

    #[test]
    fn friend_only_grant_roundtrip_and_expiry_reject() {
        let keys = generate_keys();
        let token = build_friend_only_grant_token(
            &keys,
            &TopicId::new("kukuri:topic:friends"),
            &ChannelId::new("channel-1"),
            "friends",
            "epoch-1",
            &generate_keys().export_secret_hex(),
            None,
        )
        .expect("friend-only grant");

        let preview =
            parse_friend_only_grant_token(token.as_str()).expect("parse friend-only grant");
        assert_eq!(preview.owner_pubkey, keys.public_key());
        assert_eq!(preview.epoch_id, "epoch-1");

        let expired = build_friend_only_grant_token(
            &keys,
            &TopicId::new("kukuri:topic:friends"),
            &ChannelId::new("channel-1"),
            "friends",
            "epoch-1",
            &generate_keys().export_secret_hex(),
            Some(1),
        )
        .expect("expired grant");
        let error = parse_friend_only_grant_token(expired.as_str()).expect_err("expired grant");
        assert!(error.to_string().contains("expired"));
    }

    #[test]
    fn friend_only_grant_parser_rejects_signer_mismatch() {
        let signer = generate_keys();
        let other = generate_keys();
        let token = FriendOnlyGrantTokenV1 {
            envelope: sign_envelope_json(
                &signer,
                "channel-friend-grant",
                vec![vec!["object".into(), "channel-friend-grant".into()]],
                &KukuriFriendOnlyGrantEnvelopeContentV1 {
                    channel_id: ChannelId::new("channel-1"),
                    topic_id: TopicId::new("kukuri:topic:friends"),
                    channel_label: "friends".into(),
                    owner_pubkey: other.public_key(),
                    epoch_id: "epoch-1".into(),
                    namespace_secret_hex: generate_keys().export_secret_hex(),
                    expires_at: None,
                },
            )
            .expect("grant envelope"),
        };
        let encoded = serde_json::to_string(&token).expect("encode token");
        let error =
            parse_friend_only_grant_token(encoded.as_str()).expect_err("owner mismatch must fail");
        assert!(error.to_string().contains("owner pubkey must match"));
    }

    #[test]
    fn channel_policy_and_participant_roundtrip() {
        let owner = generate_keys();
        let participant = generate_keys();
        let policy = PrivateChannelPolicyDocV1 {
            channel_id: ChannelId::new("channel-1"),
            topic_id: TopicId::new("kukuri:topic:friends"),
            audience_kind: ChannelAudienceKind::FriendOnly,
            owner_pubkey: owner.public_key(),
            epoch_id: "epoch-1".into(),
            sharing_state: ChannelSharingState::Open,
            rotated_at: None,
            previous_epoch_id: None,
        };
        let policy_envelope =
            build_private_channel_policy_envelope(&owner, &policy).expect("policy envelope");
        let parsed_policy = parse_private_channel_policy(&policy_envelope)
            .expect("parse policy")
            .expect("policy");
        assert_eq!(parsed_policy.audience_kind, ChannelAudienceKind::FriendOnly);

        let participant_doc = PrivateChannelParticipantDocV1 {
            channel_id: ChannelId::new("channel-1"),
            topic_id: TopicId::new("kukuri:topic:friends"),
            epoch_id: "epoch-1".into(),
            participant_pubkey: participant.public_key(),
            joined_at: 10,
            is_owner: false,
            join_mode: Some(PrivateChannelJoinMode::FriendOnlyGrant),
            sponsor_pubkey: Some(owner.public_key()),
            share_token_id: None,
        };
        let participant_envelope =
            build_private_channel_participant_envelope(&participant, &participant_doc)
                .expect("participant envelope");
        let parsed_participant = parse_private_channel_participant(&participant_envelope)
            .expect("parse participant")
            .expect("participant");
        assert_eq!(
            parsed_participant.participant_pubkey,
            participant.public_key()
        );
    }

    #[test]
    fn friend_plus_share_roundtrip_and_expiry_reject() {
        let owner = generate_keys();
        let sponsor = generate_keys();
        let token = build_friend_plus_share_token(
            &sponsor,
            &TopicId::new("kukuri:topic:friends-plus"),
            &ChannelId::new("channel-1"),
            "friends+",
            &owner.public_key(),
            "epoch-1",
            &generate_keys().export_secret_hex(),
            None,
        )
        .expect("friend-plus share");

        let preview =
            parse_friend_plus_share_token(token.as_str()).expect("parse friend-plus share");
        assert_eq!(preview.owner_pubkey, owner.public_key());
        assert_eq!(preview.sponsor_pubkey, sponsor.public_key());
        assert_eq!(preview.epoch_id, "epoch-1");
        assert_eq!(preview.share_token_id.len(), 64);

        let expired = build_friend_plus_share_token(
            &sponsor,
            &TopicId::new("kukuri:topic:friends-plus"),
            &ChannelId::new("channel-1"),
            "friends+",
            &owner.public_key(),
            "epoch-1",
            &generate_keys().export_secret_hex(),
            Some(1),
        )
        .expect("expired friend-plus share");
        let error = parse_friend_plus_share_token(expired.as_str()).expect_err("expired share");
        assert!(error.to_string().contains("expired"));
    }

    #[test]
    fn friend_plus_share_parser_rejects_signer_mismatch() {
        let owner = generate_keys();
        let signer = generate_keys();
        let sponsor = generate_keys();
        let token = FriendPlusShareTokenV1 {
            envelope: sign_envelope_json(
                &signer,
                "channel-share",
                vec![vec!["object".into(), "channel-share".into()]],
                &KukuriFriendPlusShareEnvelopeContentV1 {
                    channel_id: ChannelId::new("channel-1"),
                    topic_id: TopicId::new("kukuri:topic:friends-plus"),
                    channel_label: "friends+".into(),
                    owner_pubkey: owner.public_key(),
                    sponsor_pubkey: sponsor.public_key(),
                    epoch_id: "epoch-1".into(),
                    namespace_secret_hex: generate_keys().export_secret_hex(),
                    expires_at: None,
                },
            )
            .expect("share envelope"),
        };
        let encoded = serde_json::to_string(&token).expect("encode share");
        let error = parse_friend_plus_share_token(encoded.as_str())
            .expect_err("sponsor mismatch must fail");
        assert!(error.to_string().contains("sponsor pubkey must match"));
    }

    #[test]
    fn channel_rotation_grant_encrypt_decrypt_roundtrip_and_wrong_recipient_fails() {
        let owner = generate_keys();
        let recipient = generate_keys();
        let wrong_recipient = generate_keys();
        let payload = PrivateChannelRotationGrantPayloadV1 {
            channel_id: ChannelId::new("channel-1"),
            topic_id: TopicId::new("kukuri:topic:friends-plus"),
            owner_pubkey: owner.public_key(),
            recipient_pubkey: recipient.public_key(),
            old_epoch_id: "epoch-1".into(),
            new_epoch_id: "epoch-2".into(),
            new_namespace_secret_hex: generate_keys().export_secret_hex(),
        };
        let doc = encrypt_private_channel_rotation_grant(&owner, &payload)
            .expect("encrypt rotation grant");
        let envelope = build_private_channel_rotation_grant_envelope(&owner, &doc)
            .expect("rotation grant envelope");
        let parsed_doc = parse_private_channel_rotation_grant(&envelope)
            .expect("parse rotation grant")
            .expect("rotation grant");
        let decrypted = decrypt_private_channel_rotation_grant(&recipient, &parsed_doc)
            .expect("decrypt rotation grant");
        assert_eq!(decrypted.new_epoch_id, "epoch-2");
        assert_eq!(decrypted.recipient_pubkey, recipient.public_key());

        let error = decrypt_private_channel_rotation_grant(&wrong_recipient, &parsed_doc)
            .expect_err("wrong recipient must fail");
        assert!(error.to_string().contains("recipient pubkey"));
    }
}
