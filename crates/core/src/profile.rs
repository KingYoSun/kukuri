use anyhow::{Context, Result, bail};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::crypto::{now_timestamp_millis, validate_pubkey};
use crate::{
    AssetRef, AssetRole, BlobHash, EnvelopeId, KukuriEnvelope, KukuriKeys, Pubkey,
    RepostSourceSnapshotV1, TopicId, author_profile_topic_id,
};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub pubkey: Pubkey,
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub about: Option<String>,
    pub picture: Option<String>,
    #[serde(
        default,
        serialize_with = "serialize_profile_asset_ref",
        deserialize_with = "deserialize_profile_asset_ref"
    )]
    pub picture_asset: Option<AssetRef>,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KukuriProfileEnvelopeContentV1 {
    pub author_pubkey: Pubkey,
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub about: Option<String>,
    pub picture: Option<String>,
    #[serde(
        default,
        serialize_with = "serialize_profile_asset_ref",
        deserialize_with = "deserialize_profile_asset_ref"
    )]
    pub picture_asset: Option<AssetRef>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorProfileDocV1 {
    pub author_pubkey: Pubkey,
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub about: Option<String>,
    pub picture: Option<String>,
    #[serde(
        default,
        serialize_with = "serialize_profile_asset_ref",
        deserialize_with = "deserialize_profile_asset_ref"
    )]
    pub picture_asset: Option<AssetRef>,
    pub updated_at: i64,
    pub envelope_id: EnvelopeId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ProfileAssetRefWire {
    pub hash: BlobHash,
    pub mime: String,
    pub bytes: u64,
    pub role: String,
}

fn serialize_profile_asset_ref<S>(
    value: &Option<AssetRef>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let wire = value.as_ref().map(|asset| ProfileAssetRefWire {
        hash: asset.hash.clone(),
        mime: asset.mime.clone(),
        bytes: asset.bytes,
        role: "profile_avatar".into(),
    });
    wire.serialize(serializer)
}

fn deserialize_profile_asset_ref<'de, D>(deserializer: D) -> Result<Option<AssetRef>, D::Error>
where
    D: Deserializer<'de>,
{
    let wire = Option::<ProfileAssetRefWire>::deserialize(deserializer)?;
    wire.map(|asset| {
        if asset.role != "profile_avatar" && asset.role != "ProfileAvatar" {
            return Err(de::Error::custom(
                "profile picture asset role must be profile_avatar",
            ));
        }
        Ok(AssetRef {
            hash: asset.hash,
            mime: asset.mime,
            bytes: asset.bytes,
            role: AssetRole::ProfileAvatar,
        })
    })
    .transpose()
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
    crate::sign_envelope_at(
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
    crate::sign_envelope_at(
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
    crate::sign_envelope_at(
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
    crate::sign_envelope_at(
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
        picture_asset: metadata.picture_asset,
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
