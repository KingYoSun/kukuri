use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use tokio::{fs, sync::Mutex};

use crate::{domain::entities::ProfileAvatarDocEntry, shared::AppError};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProfileAvatarDocument {
    next_version: u64,
    entries: HashMap<String, ProfileAvatarDocEntry>,
}

impl Default for ProfileAvatarDocument {
    fn default() -> Self {
        Self {
            next_version: 1,
            entries: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileAvatarSyncPackage {
    pub entry: ProfileAvatarDocEntry,
    pub encrypted_blob: Vec<u8>,
}

pub struct ProfileAvatarStore {
    root_dir: PathBuf,
    blobs_dir: PathBuf,
    doc_path: PathBuf,
    document: Mutex<ProfileAvatarDocument>,
}

impl ProfileAvatarStore {
    pub async fn new(root_dir: PathBuf) -> Result<Self, AppError> {
        fs::create_dir_all(&root_dir).await.map_err(|err| {
            AppError::Storage(format!("Failed to create profile avatar dir: {err}"))
        })?;

        let blobs_dir = root_dir.join("blobs");
        fs::create_dir_all(&blobs_dir)
            .await
            .map_err(|err| AppError::Storage(format!("Failed to create blobs dir: {err}")))?;

        let doc_path = root_dir.join("doc.json");
        let document = if fs::metadata(&doc_path).await.is_ok() {
            let bytes = fs::read(&doc_path)
                .await
                .map_err(|err| AppError::Storage(format!("Failed to read avatar doc: {err}")))?;
            if bytes.is_empty() {
                ProfileAvatarDocument::default()
            } else {
                serde_json::from_slice(&bytes).map_err(|err| {
                    AppError::DeserializationError(format!("Failed to parse avatar doc: {err}"))
                })?
            }
        } else {
            ProfileAvatarDocument::default()
        };

        Ok(Self {
            root_dir,
            blobs_dir,
            doc_path,
            document: Mutex::new(document),
        })
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    pub async fn write_encrypted_blob(
        &self,
        blob_hash: &str,
        data: &[u8],
    ) -> Result<PathBuf, AppError> {
        let path = self.blobs_dir.join(blob_hash);
        fs::write(&path, data)
            .await
            .map_err(|err| AppError::Storage(format!("Failed to write encrypted blob: {err}")))?;
        Ok(path)
    }

    pub async fn read_encrypted_blob(&self, blob_hash: &str) -> Result<Vec<u8>, AppError> {
        let path = self.blobs_dir.join(blob_hash);
        fs::read(&path)
            .await
            .map_err(|err| AppError::Storage(format!("Failed to read encrypted blob: {err}")))
    }

    pub async fn encrypted_blob_exists(&self, blob_hash: &str) -> bool {
        fs::metadata(self.blobs_dir.join(blob_hash)).await.is_ok()
    }

    pub async fn upsert_entry(
        &self,
        mut entry: ProfileAvatarDocEntry,
    ) -> Result<ProfileAvatarDocEntry, AppError> {
        let mut document = self.document.lock().await;
        let version = document.next_version;
        document.next_version += 1;
        entry.version = version;
        document.entries.insert(entry.npub.clone(), entry.clone());
        self.persist(&document).await?;
        Ok(entry)
    }

    pub async fn get_entry(&self, npub: &str) -> Result<Option<ProfileAvatarDocEntry>, AppError> {
        let document = self.document.lock().await;
        Ok(document.entries.get(npub).cloned())
    }

    pub async fn export_sync_package(
        &self,
        npub: &str,
    ) -> Result<Option<ProfileAvatarSyncPackage>, AppError> {
        let entry = {
            let document = self.document.lock().await;
            match document.entries.get(npub) {
                Some(entry) => entry.clone(),
                None => return Ok(None),
            }
        };
        let blob = self.read_encrypted_blob(&entry.blob_hash).await?;
        Ok(Some(ProfileAvatarSyncPackage {
            entry,
            encrypted_blob: blob,
        }))
    }

    pub async fn import_sync_package(
        &self,
        package: ProfileAvatarSyncPackage,
    ) -> Result<ProfileAvatarDocEntry, AppError> {
        let ProfileAvatarSyncPackage {
            entry,
            encrypted_blob,
        } = package;

        // 既存エントリのバージョンより新しい場合のみ更新
        let mut document = self.document.lock().await;
        let should_update = match document.entries.get(&entry.npub) {
            Some(current) => entry.version > current.version,
            None => true,
        };

        if !should_update {
            return Ok(document
                .entries
                .get(&entry.npub)
                .cloned()
                .expect("entry must exist"));
        }

        self.write_encrypted_blob(&entry.blob_hash, &encrypted_blob)
            .await?;
        document.entries.insert(entry.npub.clone(), entry.clone());
        document.next_version = document.next_version.max(entry.version + 1);
        self.persist(&document).await?;
        Ok(entry)
    }

    pub async fn entries_snapshot(&self) -> Vec<ProfileAvatarDocEntry> {
        let document = self.document.lock().await;
        document.entries.values().cloned().collect()
    }

    async fn persist(&self, document: &ProfileAvatarDocument) -> Result<(), AppError> {
        let json = serde_json::to_vec_pretty(document).map_err(|err| {
            AppError::SerializationError(format!("Failed to serialize avatar doc: {err}"))
        })?;
        fs::write(&self.doc_path, json)
            .await
            .map_err(|err| AppError::Storage(format!("Failed to persist avatar doc: {err}")))
    }
}
