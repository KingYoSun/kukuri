use crate::application::ports::join_request_store::{
    InviteUsageRecord, JoinRequestRecord, JoinRequestStore,
};
use crate::infrastructure::storage::SecureStorage;
use crate::shared::error::AppError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const JOIN_REQUEST_INDEX_PREFIX: &str = "join_request_index_v1";
const INVITE_USAGE_PREFIX: &str = "invite_usage_v1";

#[derive(Debug, Serialize, Deserialize, Default)]
struct JoinRequestIndex {
    version: u32,
    event_ids: Vec<String>,
}

pub struct SecureJoinRequestStore {
    secure_storage: Arc<dyn SecureStorage>,
}

impl SecureJoinRequestStore {
    pub fn new(secure_storage: Arc<dyn SecureStorage>) -> Self {
        Self { secure_storage }
    }

    fn index_key(owner_pubkey: &str) -> String {
        format!("{JOIN_REQUEST_INDEX_PREFIX}:{owner_pubkey}")
    }

    fn storage_key(owner_pubkey: &str, event_id: &str) -> String {
        format!("join_request:{owner_pubkey}:{event_id}")
    }

    fn invite_usage_key(owner_pubkey: &str, invite_event_id: &str) -> String {
        format!("{INVITE_USAGE_PREFIX}:{owner_pubkey}:{invite_event_id}")
    }

    async fn load_index(&self, owner_pubkey: &str) -> Result<JoinRequestIndex, AppError> {
        let key = Self::index_key(owner_pubkey);
        let Some(raw) = self.secure_storage.retrieve(&key).await? else {
            return Ok(JoinRequestIndex::default());
        };
        serde_json::from_str(&raw).map_err(|err| AppError::DeserializationError(err.to_string()))
    }

    async fn save_index(
        &self,
        owner_pubkey: &str,
        index: &JoinRequestIndex,
    ) -> Result<(), AppError> {
        let key = Self::index_key(owner_pubkey);
        let json = serde_json::to_string(index)
            .map_err(|err| AppError::SerializationError(err.to_string()))?;
        self.secure_storage.store(&key, &json).await?;
        Ok(())
    }
}

#[async_trait]
impl JoinRequestStore for SecureJoinRequestStore {
    async fn upsert_request(
        &self,
        owner_pubkey: &str,
        record: JoinRequestRecord,
    ) -> Result<(), AppError> {
        let key = Self::storage_key(owner_pubkey, &record.event.id);
        let json = serde_json::to_string(&record)
            .map_err(|err| AppError::SerializationError(err.to_string()))?;
        self.secure_storage.store(&key, &json).await?;

        let mut index = self.load_index(owner_pubkey).await?;
        if !index.event_ids.iter().any(|id| id == &record.event.id) {
            index.event_ids.push(record.event.id.clone());
        }
        if index.version == 0 {
            index.version = 1;
        }
        self.save_index(owner_pubkey, &index).await?;
        Ok(())
    }

    async fn list_requests(&self, owner_pubkey: &str) -> Result<Vec<JoinRequestRecord>, AppError> {
        let mut index = self.load_index(owner_pubkey).await?;
        let mut records = Vec::with_capacity(index.event_ids.len());
        let mut retained_ids = Vec::with_capacity(index.event_ids.len());

        for event_id in index.event_ids.iter() {
            let key = Self::storage_key(owner_pubkey, event_id);
            if let Some(raw) = self.secure_storage.retrieve(&key).await? {
                match serde_json::from_str::<JoinRequestRecord>(&raw) {
                    Ok(record) => {
                        retained_ids.push(event_id.clone());
                        records.push(record);
                    }
                    Err(_) => {
                        let _ = self.secure_storage.delete(&key).await;
                    }
                }
            }
        }

        if retained_ids.len() != index.event_ids.len() {
            index.event_ids = retained_ids;
            if index.version == 0 {
                index.version = 1;
            }
            self.save_index(owner_pubkey, &index).await?;
        }

        records.sort_by(|a, b| b.received_at.cmp(&a.received_at));
        Ok(records)
    }

    async fn get_request(
        &self,
        owner_pubkey: &str,
        event_id: &str,
    ) -> Result<Option<JoinRequestRecord>, AppError> {
        let key = Self::storage_key(owner_pubkey, event_id);
        let Some(raw) = self.secure_storage.retrieve(&key).await? else {
            return Ok(None);
        };
        let record: JoinRequestRecord = serde_json::from_str(&raw)
            .map_err(|err| AppError::DeserializationError(err.to_string()))?;
        Ok(Some(record))
    }

    async fn delete_request(&self, owner_pubkey: &str, event_id: &str) -> Result<(), AppError> {
        let key = Self::storage_key(owner_pubkey, event_id);
        let _ = self.secure_storage.delete(&key).await;

        let mut index = self.load_index(owner_pubkey).await?;
        index.event_ids.retain(|id| id != event_id);
        if index.version == 0 {
            index.version = 1;
        }
        self.save_index(owner_pubkey, &index).await?;
        Ok(())
    }

    async fn get_invite_usage(
        &self,
        owner_pubkey: &str,
        invite_event_id: &str,
    ) -> Result<Option<InviteUsageRecord>, AppError> {
        let key = Self::invite_usage_key(owner_pubkey, invite_event_id);
        let Some(raw) = self.secure_storage.retrieve(&key).await? else {
            return Ok(None);
        };
        let record: InviteUsageRecord = serde_json::from_str(&raw)
            .map_err(|err| AppError::DeserializationError(err.to_string()))?;
        Ok(Some(record))
    }

    async fn upsert_invite_usage(
        &self,
        owner_pubkey: &str,
        record: InviteUsageRecord,
    ) -> Result<(), AppError> {
        let key = Self::invite_usage_key(owner_pubkey, &record.invite_event_id);
        let json = serde_json::to_string(&record)
            .map_err(|err| AppError::SerializationError(err.to_string()))?;
        self.secure_storage.store(&key, &json).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::Event;
    use crate::infrastructure::storage::secure_storage::DefaultSecureStorage;
    use chrono::Utc;

    #[tokio::test]
    async fn store_and_load_join_request() {
        let storage = Arc::new(DefaultSecureStorage::new());
        let store = SecureJoinRequestStore::new(storage);

        let event = Event {
            id: "event-1".to_string(),
            pubkey: "pubkey-req".to_string(),
            created_at: Utc::now(),
            kind: 39022,
            tags: vec![],
            content: "{}".to_string(),
            sig: "sig".to_string(),
        };
        let record = JoinRequestRecord {
            event: event.clone(),
            topic_id: "kukuri:topic1".to_string(),
            scope: "friend".to_string(),
            requester_pubkey: "pubkey-req".to_string(),
            target_pubkey: None,
            requested_at: Some(100),
            received_at: Utc::now().timestamp(),
            invite_event_json: None,
        };

        store
            .upsert_request("owner", record.clone())
            .await
            .expect("store");

        let loaded = store.get_request("owner", &event.id).await.expect("get");
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().event.id, event.id);

        let list = store.list_requests("owner").await.expect("list");
        assert_eq!(list.len(), 1);

        store
            .delete_request("owner", &event.id)
            .await
            .expect("delete");
        let list = store.list_requests("owner").await.expect("list empty");
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn store_and_load_invite_usage() {
        let storage = Arc::new(DefaultSecureStorage::new());
        let store = SecureJoinRequestStore::new(storage);
        let record = InviteUsageRecord {
            invite_event_id: "invite-1".to_string(),
            max_uses: 2,
            used_count: 1,
            last_used_at: Utc::now().timestamp(),
        };

        store
            .upsert_invite_usage("owner", record.clone())
            .await
            .expect("store");

        let loaded = store
            .get_invite_usage("owner", "invite-1")
            .await
            .expect("get");
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().used_count, 1);
    }
}
