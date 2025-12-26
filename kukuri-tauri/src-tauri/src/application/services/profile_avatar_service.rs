use std::{path::PathBuf, str::FromStr};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use blake3::Hasher as Blake3;
use rand::TryRngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    domain::entities::{ProfileAvatarAccessLevel, ProfileAvatarDocEntry},
    infrastructure::{
        crypto::{CapabilityEncryptor, EncryptedSessionKey, StreamEncryptor},
        storage::profile_avatar_store::{ProfileAvatarStore, ProfileAvatarSyncPackage},
    },
    shared::AppError,
};

const MAX_AVATAR_BYTES: usize = 2 * 1024 * 1024;
const AES_KEY_SIZE: usize = 32;
const AES_NONCE_SIZE: usize = 12;

#[derive(Debug)]
pub struct UploadProfileAvatarInput {
    pub npub: String,
    pub bytes: Vec<u8>,
    pub format: String,
    pub access_level: ProfileAvatarAccessLevel,
}

#[derive(Debug)]
pub struct ProfileAvatarFetchResult {
    pub bytes: Vec<u8>,
    pub metadata: ProfileAvatarDocEntry,
}

#[derive(Debug, Serialize, Deserialize)]
struct ShareTicketPayload {
    version: u8,
    access_level: String,
    capability_secret: String,
}

pub struct ProfileAvatarService {
    store: ProfileAvatarStore,
}

impl ProfileAvatarService {
    pub async fn new(root_dir: PathBuf) -> Result<Self, AppError> {
        let store = ProfileAvatarStore::new(root_dir).await?;
        Ok(Self { store })
    }

    pub async fn upload_avatar(
        &self,
        input: UploadProfileAvatarInput,
    ) -> Result<ProfileAvatarDocEntry, AppError> {
        validate_format(&input.format)?;
        validate_size(input.bytes.len())?;

        let encrypted = StreamEncryptor::encrypt(&input.bytes)?;
        let blob_hash = compute_blob_hash(&encrypted.ciphertext);
        let content_sha256 = compute_content_sha256(&input.bytes);

        self.store
            .write_encrypted_blob(&blob_hash, &encrypted.ciphertext)
            .await?;

        let mut capability_secret = [0u8; AES_KEY_SIZE];
        rand::rngs::OsRng
            .try_fill_bytes(&mut capability_secret)
            .map_err(|err| {
                AppError::Crypto(format!("Failed to generate capability secret: {err}"))
            })?;

        let encrypted_session_key =
            CapabilityEncryptor::encrypt_session_key(&encrypted.session_key, &capability_secret)?;

        let share_ticket = encode_share_ticket(input.access_level, &capability_secret)?;
        let key_nonce_b64 = BASE64_STANDARD.encode(encrypted_session_key.nonce);
        let encrypted_key_b64 = BASE64_STANDARD.encode(encrypted_session_key.ciphertext);
        let encryption_nonce_b64 = BASE64_STANDARD.encode(encrypted.nonce);

        let entry = ProfileAvatarDocEntry {
            npub: input.npub,
            blob_hash,
            format: input.format,
            size_bytes: input.bytes.len() as u64,
            access_level: input.access_level,
            share_ticket,
            encrypted_key: encrypted_key_b64,
            key_nonce: key_nonce_b64,
            encryption_nonce: encryption_nonce_b64,
            content_sha256,
            updated_at: chrono::Utc::now(),
            version: 0,
        };

        self.store.upsert_entry(entry).await
    }

    pub async fn fetch_avatar(&self, npub: &str) -> Result<ProfileAvatarFetchResult, AppError> {
        let metadata =
            self.store.get_entry(npub).await?.ok_or_else(|| {
                AppError::NotFound(format!("Profile avatar not found for {npub}"))
            })?;

        let ticket = decode_share_ticket(&metadata.share_ticket)?;
        if ticket.access_level != metadata.access_level {
            return Err(AppError::validation(
                crate::shared::validation::ValidationFailureKind::Generic,
                format!(
                    "share ticket access level mismatch (ticket={}, metadata={})",
                    ticket.access_level.as_str(),
                    metadata.access_level.as_str()
                ),
            ));
        }
        let capability_secret = ticket.capability_secret;
        let encrypted_key = BASE64_STANDARD
            .decode(metadata.encrypted_key.as_bytes())
            .map_err(|err| {
                AppError::DeserializationError(format!("Invalid encrypted key: {err}"))
            })?;
        let mut key_nonce = [0u8; AES_NONCE_SIZE];
        let decoded_key_nonce = BASE64_STANDARD
            .decode(metadata.key_nonce.as_bytes())
            .map_err(|err| AppError::DeserializationError(format!("Invalid key nonce: {err}")))?;
        if decoded_key_nonce.len() != AES_NONCE_SIZE {
            return Err(AppError::DeserializationError(
                "Key nonce has invalid length".to_string(),
            ));
        }
        key_nonce.copy_from_slice(&decoded_key_nonce);

        let encrypted_session_key = EncryptedSessionKey {
            ciphertext: encrypted_key,
            nonce: key_nonce,
        };
        let session_key =
            CapabilityEncryptor::decrypt_session_key(&encrypted_session_key, &capability_secret)?;

        let mut encryption_nonce = [0u8; AES_NONCE_SIZE];
        let decoded_encryption_nonce = BASE64_STANDARD
            .decode(metadata.encryption_nonce.as_bytes())
            .map_err(|err| {
                AppError::DeserializationError(format!("Invalid encryption nonce: {err}"))
            })?;
        if decoded_encryption_nonce.len() != AES_NONCE_SIZE {
            return Err(AppError::DeserializationError(
                "Encryption nonce has invalid length".to_string(),
            ));
        }
        encryption_nonce.copy_from_slice(&decoded_encryption_nonce);

        let encrypted_blob = self.store.read_encrypted_blob(&metadata.blob_hash).await?;
        let plaintext = StreamEncryptor::decrypt(&encrypted_blob, &session_key, &encryption_nonce)?;

        Ok(ProfileAvatarFetchResult {
            bytes: plaintext,
            metadata,
        })
    }

    pub async fn export_sync_package(
        &self,
        npub: &str,
    ) -> Result<Option<ProfileAvatarSyncPackage>, AppError> {
        self.store.export_sync_package(npub).await
    }

    pub async fn import_sync_package(
        &self,
        package: ProfileAvatarSyncPackage,
    ) -> Result<ProfileAvatarDocEntry, AppError> {
        self.store.import_sync_package(package).await
    }

    pub async fn entries_snapshot(&self) -> Vec<ProfileAvatarDocEntry> {
        self.store.entries_snapshot().await
    }
}

fn validate_format(format: &str) -> Result<(), AppError> {
    if !format.starts_with("image/") {
        return Err(AppError::validation(
            crate::shared::validation::ValidationFailureKind::Generic,
            "Profile avatar must be an image format",
        ));
    }
    Ok(())
}

fn validate_size(size: usize) -> Result<(), AppError> {
    if size == 0 {
        return Err(AppError::validation(
            crate::shared::validation::ValidationFailureKind::Generic,
            "Profile avatar file is empty",
        ));
    }
    if size > MAX_AVATAR_BYTES {
        return Err(AppError::validation(
            crate::shared::validation::ValidationFailureKind::ContentTooLarge,
            format!("Profile avatar size exceeds limit ({size} bytes > {MAX_AVATAR_BYTES} bytes)"),
        ));
    }
    Ok(())
}

fn compute_blob_hash(data: &[u8]) -> String {
    let mut hasher = Blake3::new();
    hasher.update(data);
    hasher.finalize().to_hex().to_string()
}

fn compute_content_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

fn encode_share_ticket(
    access_level: ProfileAvatarAccessLevel,
    capability_secret: &[u8; AES_KEY_SIZE],
) -> Result<String, AppError> {
    let payload = ShareTicketPayload {
        version: 1,
        access_level: access_level.as_str().to_string(),
        capability_secret: BASE64_STANDARD.encode(capability_secret),
    };
    let json = serde_json::to_string(&payload).map_err(|err| {
        AppError::SerializationError(format!("Failed to encode share ticket: {err}"))
    })?;
    Ok(BASE64_STANDARD.encode(json))
}

struct DecodedShareTicket {
    pub access_level: ProfileAvatarAccessLevel,
    pub capability_secret: [u8; AES_KEY_SIZE],
}

fn decode_share_ticket(ticket: &str) -> Result<DecodedShareTicket, AppError> {
    let json_bytes = BASE64_STANDARD
        .decode(ticket.as_bytes())
        .map_err(|err| AppError::DeserializationError(format!("Invalid share ticket: {err}")))?;
    let payload: ShareTicketPayload = serde_json::from_slice(&json_bytes).map_err(|err| {
        AppError::DeserializationError(format!("Failed to parse share ticket payload: {err}"))
    })?;
    let access_level = ProfileAvatarAccessLevel::from_str(&payload.access_level).map_err(|_| {
        AppError::DeserializationError("Unknown access level in share ticket".to_string())
    })?;
    let secret_bytes = BASE64_STANDARD
        .decode(payload.capability_secret.as_bytes())
        .map_err(|err| {
            AppError::DeserializationError(format!("Invalid capability secret: {err}"))
        })?;
    if secret_bytes.len() != AES_KEY_SIZE {
        return Err(AppError::DeserializationError(
            "Capability secret has invalid length".to_string(),
        ));
    }
    let mut capability_secret = [0u8; AES_KEY_SIZE];
    capability_secret.copy_from_slice(&secret_bytes);
    Ok(DecodedShareTicket {
        access_level,
        capability_secret,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn upload_and_fetch_roundtrip() {
        let temp = tempdir().unwrap();
        let service = ProfileAvatarService::new(temp.path().to_path_buf())
            .await
            .expect("service init");
        let npub = "npub1avataruser";
        let bytes = vec![1, 2, 3, 4, 5, 6, 7, 8];

        let entry = service
            .upload_avatar(UploadProfileAvatarInput {
                npub: npub.to_string(),
                bytes: bytes.clone(),
                format: "image/png".to_string(),
                access_level: ProfileAvatarAccessLevel::Public,
            })
            .await
            .expect("upload");

        assert_eq!(entry.npub, npub);
        assert_eq!(entry.size_bytes, bytes.len() as u64);

        let fetched = service.fetch_avatar(npub).await.expect("fetch");
        assert_eq!(fetched.metadata.blob_hash, entry.blob_hash);
        assert_eq!(fetched.bytes, bytes);
    }
}
