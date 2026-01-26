use crate::application::ports::group_key_store::{GroupKeyEntry, GroupKeyRecord, GroupKeyStore};
use crate::infrastructure::storage::SecureStorage;
use crate::shared::error::AppError;
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const GROUP_KEY_INDEX_KEY: &str = "group_key_index_v1";

#[derive(Debug, Serialize, Deserialize, Default)]
struct GroupKeyIndex {
    version: u32,
    entries: Vec<GroupKeyEntry>,
}

pub struct SecureGroupKeyStore {
    secure_storage: Arc<dyn SecureStorage>,
}

impl SecureGroupKeyStore {
    pub fn new(secure_storage: Arc<dyn SecureStorage>) -> Self {
        Self { secure_storage }
    }

    fn storage_key(topic_id: &str, scope: &str, epoch: i64) -> String {
        format!("group_key:{topic_id}:{scope}:{epoch}")
    }

    async fn load_index(&self) -> Result<GroupKeyIndex, AppError> {
        let Some(raw) = self.secure_storage.retrieve(GROUP_KEY_INDEX_KEY).await? else {
            return Ok(GroupKeyIndex::default());
        };
        serde_json::from_str(&raw).map_err(|err| AppError::DeserializationError(err.to_string()))
    }

    async fn save_index(&self, index: &GroupKeyIndex) -> Result<(), AppError> {
        let json = serde_json::to_string(index)
            .map_err(|err| AppError::SerializationError(err.to_string()))?;
        self.secure_storage
            .store(GROUP_KEY_INDEX_KEY, &json)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl GroupKeyStore for SecureGroupKeyStore {
    async fn store_key(&self, record: GroupKeyRecord) -> Result<(), AppError> {
        let key = Self::storage_key(&record.topic_id, &record.scope, record.epoch);
        self.secure_storage.store(&key, &record.key_b64).await?;

        let mut index = self.load_index().await?;
        let now = Utc::now().timestamp();
        let mut updated = false;
        for entry in index.entries.iter_mut() {
            if entry.topic_id == record.topic_id
                && entry.scope == record.scope
                && entry.epoch == record.epoch
            {
                entry.stored_at = now;
                updated = true;
                break;
            }
        }
        if !updated {
            index.entries.push(GroupKeyEntry {
                topic_id: record.topic_id.clone(),
                scope: record.scope.clone(),
                epoch: record.epoch,
                stored_at: now,
            });
        }
        if index.version == 0 {
            index.version = 1;
        }
        self.save_index(&index).await?;
        Ok(())
    }

    async fn get_key(
        &self,
        topic_id: &str,
        scope: &str,
        epoch: i64,
    ) -> Result<Option<GroupKeyRecord>, AppError> {
        let key = Self::storage_key(topic_id, scope, epoch);
        let Some(value) = self.secure_storage.retrieve(&key).await? else {
            return Ok(None);
        };
        let stored_at = Utc::now().timestamp();
        Ok(Some(GroupKeyRecord {
            topic_id: topic_id.to_string(),
            scope: scope.to_string(),
            epoch,
            key_b64: value,
            stored_at,
        }))
    }

    async fn get_latest_key(
        &self,
        topic_id: &str,
        scope: &str,
    ) -> Result<Option<GroupKeyRecord>, AppError> {
        let index = self.load_index().await?;
        let mut latest_epoch = None;
        for entry in index.entries.iter() {
            if entry.topic_id == topic_id && entry.scope == scope {
                latest_epoch = match latest_epoch {
                    Some(epoch) if epoch >= entry.epoch => Some(epoch),
                    _ => Some(entry.epoch),
                };
            }
        }
        let Some(epoch) = latest_epoch else {
            return Ok(None);
        };
        self.get_key(topic_id, scope, epoch).await
    }

    async fn list_keys(&self) -> Result<Vec<GroupKeyEntry>, AppError> {
        let index = self.load_index().await?;
        Ok(index.entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::storage::secure_storage::DefaultSecureStorage;

    #[tokio::test]
    async fn store_and_load_group_key_index() {
        let secure = Arc::new(DefaultSecureStorage::new());
        let store = SecureGroupKeyStore::new(secure);
        let record = GroupKeyRecord {
            topic_id: "kukuri:topic1".to_string(),
            scope: "friend".to_string(),
            epoch: 3,
            key_b64: "dGVzdGtleQ==".to_string(),
            stored_at: Utc::now().timestamp(),
        };

        store.store_key(record.clone()).await.expect("store key");

        let loaded = store
            .get_key(&record.topic_id, &record.scope, record.epoch)
            .await
            .expect("load key");
        assert!(loaded.is_some());
        let index = store.list_keys().await.expect("list keys");
        assert_eq!(index.len(), 1);
        assert_eq!(index[0].epoch, 3);
    }
}
