use crate::domain::value_objects::event_gateway::PublicKey;
use crate::domain::value_objects::offline::{
    EntityId, EntityType, OfflineActionType, OfflinePayload, SyncStatus,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// オフラインアクションを保存する際に使用するドラフト。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OfflineActionDraft {
    pub user_pubkey: PublicKey,
    pub action_type: OfflineActionType,
    pub target_id: Option<EntityId>,
    pub payload: OfflinePayload,
}

impl OfflineActionDraft {
    pub fn new(
        user_pubkey: PublicKey,
        action_type: OfflineActionType,
        target_id: Option<EntityId>,
        payload: OfflinePayload,
    ) -> Self {
        Self {
            user_pubkey,
            action_type,
            target_id,
            payload,
        }
    }
}

/// オフラインアクション取得時のフィルタ。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OfflineActionFilter {
    pub user_pubkey: Option<PublicKey>,
    pub include_synced: Option<bool>,
    pub limit: Option<u32>,
}

impl OfflineActionFilter {
    pub fn new(
        user_pubkey: Option<PublicKey>,
        include_synced: Option<bool>,
        limit: Option<u32>,
    ) -> Self {
        Self {
            user_pubkey,
            include_synced,
            limit,
        }
    }
}

/// 同期キューに追加する際のドラフト。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncQueueItemDraft {
    pub action_type: OfflineActionType,
    pub payload: OfflinePayload,
    pub priority: Option<u8>,
}

impl SyncQueueItemDraft {
    pub fn new(
        action_type: OfflineActionType,
        payload: OfflinePayload,
        priority: Option<u8>,
    ) -> Self {
        Self {
            action_type,
            payload,
            priority,
        }
    }
}

/// 楽観的更新の保存に利用するドラフト。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OptimisticUpdateDraft {
    pub entity_type: EntityType,
    pub entity_id: EntityId,
    pub original_data: Option<OfflinePayload>,
    pub updated_data: OfflinePayload,
}

impl OptimisticUpdateDraft {
    pub fn new(
        entity_type: EntityType,
        entity_id: EntityId,
        original_data: Option<OfflinePayload>,
        updated_data: OfflinePayload,
    ) -> Self {
        Self {
            entity_type,
            entity_id,
            original_data,
            updated_data,
        }
    }
}

/// 同期状態の更新に使用するコマンド。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncStatusUpdate {
    pub entity_type: EntityType,
    pub entity_id: EntityId,
    pub sync_status: SyncStatus,
    pub conflict_data: Option<OfflinePayload>,
    pub updated_at: DateTime<Utc>,
}

impl SyncStatusUpdate {
    pub fn new(
        entity_type: EntityType,
        entity_id: EntityId,
        sync_status: SyncStatus,
        conflict_data: Option<OfflinePayload>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            entity_type,
            entity_id,
            sync_status,
            conflict_data,
            updated_at,
        }
    }
}
