use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::{
    AssetRef, BlobHash, ChannelId, EnvelopeId, KukuriEnvelope, KukuriKeys, Pubkey, TopicId,
};

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
pub struct ThreadRef {
    pub root: EnvelopeId,
    pub reply_to: Option<EnvelopeId>,
}

pub type CanonicalPostHeader = KukuriPostObjectV1;

impl KukuriEnvelope {
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
}

pub fn timeline_sort_key(created_at: i64, object_id: &EnvelopeId) -> String {
    format!("{created_at:020}-{}", object_id.as_str())
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
    crate::sign_envelope_json(keys, kind, tags, &content)
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
    crate::sign_envelope_json(
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
