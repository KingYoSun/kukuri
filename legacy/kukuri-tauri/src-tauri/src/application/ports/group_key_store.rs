use crate::shared::error::AppError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupKeyEntry {
    pub topic_id: String,
    pub scope: String,
    pub epoch: i64,
    pub stored_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupKeyRecord {
    pub topic_id: String,
    pub scope: String,
    pub epoch: i64,
    pub key_b64: String,
    pub stored_at: i64,
}

#[async_trait]
pub trait GroupKeyStore: Send + Sync {
    async fn store_key(&self, record: GroupKeyRecord) -> Result<(), AppError>;
    async fn get_key(
        &self,
        topic_id: &str,
        scope: &str,
        epoch: i64,
    ) -> Result<Option<GroupKeyRecord>, AppError>;
    async fn get_latest_key(
        &self,
        topic_id: &str,
        scope: &str,
    ) -> Result<Option<GroupKeyRecord>, AppError>;
    async fn list_keys(&self) -> Result<Vec<GroupKeyEntry>, AppError>;
}
