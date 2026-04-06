use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{BlobHash, Pubkey};

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
    ProfileAvatar,
    Attachment,
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

pub const LIVE_MANIFEST_MIME: &str = "application/vnd.kukuri.live-manifest+json";
pub const GAME_MANIFEST_MIME: &str = "application/vnd.kukuri.game-manifest+json";

pub fn blob_hash(data: impl AsRef<[u8]>) -> BlobHash {
    BlobHash::new(blake3::hash(data.as_ref()).to_hex().to_string())
}

pub fn build_media_manifest_envelope(
    keys: &crate::KukuriKeys,
    topic: &crate::TopicId,
    manifest: &KukuriMediaManifestV1,
) -> Result<crate::KukuriEnvelope> {
    crate::sign_envelope_json(
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
