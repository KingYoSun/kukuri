use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use serde::{Deserialize, Serialize};

use crate::application::services::ProfileAvatarFetchResult;
use crate::domain::entities::{ProfileAvatarAccessLevel, ProfileAvatarDocEntry};

#[derive(Debug, Deserialize)]
pub struct UploadProfileAvatarRequest {
    pub npub: String,
    pub bytes: Vec<u8>,
    pub format: String,
    pub access_level: ProfileAvatarAccessLevel,
}

#[derive(Debug, Serialize)]
pub struct UploadProfileAvatarResponse {
    pub npub: String,
    pub blob_hash: String,
    pub format: String,
    pub size_bytes: u64,
    pub access_level: ProfileAvatarAccessLevel,
    pub share_ticket: String,
    pub doc_version: u64,
    pub updated_at: String,
    pub content_sha256: String,
}

impl From<ProfileAvatarDocEntry> for UploadProfileAvatarResponse {
    fn from(value: ProfileAvatarDocEntry) -> Self {
        Self {
            npub: value.npub,
            blob_hash: value.blob_hash,
            format: value.format,
            size_bytes: value.size_bytes,
            access_level: value.access_level,
            share_ticket: value.share_ticket,
            doc_version: value.version,
            updated_at: value.updated_at.to_rfc3339(),
            content_sha256: value.content_sha256,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct FetchProfileAvatarRequest {
    pub npub: String,
}

#[derive(Debug, Serialize)]
pub struct FetchProfileAvatarResponse {
    pub npub: String,
    pub blob_hash: String,
    pub format: String,
    pub size_bytes: u64,
    pub access_level: ProfileAvatarAccessLevel,
    pub share_ticket: String,
    pub doc_version: u64,
    pub updated_at: String,
    pub content_sha256: String,
    pub data_base64: String,
}

impl From<ProfileAvatarFetchResult> for FetchProfileAvatarResponse {
    fn from(value: ProfileAvatarFetchResult) -> Self {
        let metadata = value.metadata;
        Self {
            npub: metadata.npub,
            blob_hash: metadata.blob_hash,
            format: metadata.format,
            size_bytes: metadata.size_bytes,
            access_level: metadata.access_level,
            share_ticket: metadata.share_ticket,
            doc_version: metadata.version,
            updated_at: metadata.updated_at.to_rfc3339(),
            content_sha256: metadata.content_sha256,
            data_base64: BASE64_STANDARD.encode(value.bytes),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ProfileAvatarSyncRequest {
    pub npub: String,
    pub known_doc_version: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct ProfileAvatarSyncResponse {
    pub npub: String,
    pub current_version: Option<u64>,
    pub updated: bool,
    pub avatar: Option<FetchProfileAvatarResponse>,
}
