use crate::application::ports::offline_store::OfflinePersistence;
use crate::domain::entities::offline::{OfflineActionFilter, OfflineActionRecord};
use crate::shared::error::AppError;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::Mutex;

pub trait ReindexEventEmitter: Send + Sync {
    fn emit_report(&self, report: &OfflineReindexReport) -> Result<(), String>;
    fn emit_failure(&self, message: &str) -> Result<(), String>;
}

struct TauriEventEmitter {
    handle: tauri::AppHandle,
}

impl ReindexEventEmitter for TauriEventEmitter {
    fn emit_report(&self, report: &OfflineReindexReport) -> Result<(), String> {
        self.handle
            .emit("offline://reindex_complete", report)
            .map_err(|err| err.to_string())
    }

    fn emit_failure(&self, message: &str) -> Result<(), String> {
        self.handle
            .emit("offline://reindex_failed", message.to_string())
            .map_err(|err| err.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConflictDigest {
    pub entity_type: String,
    pub entity_id: String,
    pub sync_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineReindexReport {
    pub offline_action_count: usize,
    pub queued_action_count: usize,
    pub pending_queue_count: usize,
    pub stale_cache_keys: Vec<String>,
    pub optimistic_update_ids: Vec<String>,
    pub sync_conflicts: Vec<SyncConflictDigest>,
    pub queued_offline_action_ids: Vec<String>,
    pub emitted_at: i64,
}

pub struct OfflineReindexJob {
    event_emitter: Option<Arc<dyn ReindexEventEmitter>>,
    persistence: Arc<dyn OfflinePersistence>,
    gate: Mutex<()>,
}

impl OfflineReindexJob {
    pub fn create(
        app_handle: Option<tauri::AppHandle>,
        persistence: Arc<dyn OfflinePersistence>,
    ) -> Arc<Self> {
        let emitter = app_handle
            .map(|handle| Arc::new(TauriEventEmitter { handle }) as Arc<dyn ReindexEventEmitter>);
        Self::with_emitter(emitter, persistence)
    }

    pub fn with_emitter(
        event_emitter: Option<Arc<dyn ReindexEventEmitter>>,
        persistence: Arc<dyn OfflinePersistence>,
    ) -> Arc<Self> {
        Arc::new(Self {
            event_emitter,
            persistence,
            gate: Mutex::new(()),
        })
    }

    pub fn trigger(self: &Arc<Self>) {
        let job = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            job.run_guarded().await;
        });
    }

    pub async fn reindex_once(&self) -> Result<OfflineReindexReport, AppError> {
        let unsynced = self
            .persistence
            .list_actions(OfflineActionFilter::new(None, Some(false), None))
            .await?;

        let mut queued_action_count = 0usize;
        let mut queued_local_ids = Vec::new();

        for action in &unsynced {
            if self.ensure_action_in_queue(action).await? {
                queued_action_count += 1;
                queued_local_ids.push(action.action_id.to_string());
            }
        }

        let pending_queue = self.persistence.pending_sync_items().await?;
        let stale_cache = self.persistence.stale_cache_entries().await?;
        let optimistic_updates = self.persistence.unconfirmed_updates().await?;
        let conflicts = self.persistence.sync_conflicts().await?;

        let report = OfflineReindexReport {
            offline_action_count: unsynced.len(),
            queued_action_count,
            pending_queue_count: pending_queue.len(),
            stale_cache_keys: stale_cache
                .into_iter()
                .map(|entry| entry.cache_key.to_string())
                .collect(),
            optimistic_update_ids: optimistic_updates
                .into_iter()
                .map(|item| item.update_id.to_string())
                .collect(),
            sync_conflicts: conflicts
                .into_iter()
                .map(|record| SyncConflictDigest {
                    entity_type: record.entity_type.to_string(),
                    entity_id: record.entity_id.to_string(),
                    sync_status: record.sync_status.as_str().into_owned(),
                })
                .collect(),
            queued_offline_action_ids: queued_local_ids,
            emitted_at: Utc::now().timestamp_millis(),
        };

        Ok(report)
    }

    async fn ensure_action_in_queue(&self, action: &OfflineActionRecord) -> Result<bool, AppError> {
        self.persistence.enqueue_if_missing(action).await
    }

    async fn run_guarded(self: Arc<Self>) {
        let _guard = self.gate.lock().await;
        match self.reindex_once().await {
            Ok(report) => self.emit_success(&report),
            Err(err) => self.emit_failure(&err.to_string()),
        }
    }

    fn emit_success(&self, report: &OfflineReindexReport) {
        if let Some(emitter) = &self.event_emitter
            && let Err(err) = emitter.emit_report(report)
        {
            tracing::warn!(
                target: "offline::reindex",
                error = %err,
                "failed to emit offline reindex completion event"
            );
        }
        tracing::info!(
            target: "offline::reindex",
            queued = report.queued_action_count,
            pending = report.pending_queue_count,
            "offline reindex completed"
        );
    }

    fn emit_failure(&self, message: &str) {
        if let Some(emitter) = &self.event_emitter
            && let Err(err) = emitter.emit_failure(message)
        {
            tracing::warn!(
                target: "offline::reindex",
                error = %err,
                "failed to emit offline reindex failure event"
            );
        }
        tracing::error!(
            target: "offline::reindex",
            error = message,
            "offline reindex job failed"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::offline::{OfflineActionDraft, OfflineActionFilter};
    use crate::domain::value_objects::event_gateway::PublicKey;
    use crate::domain::value_objects::offline::{EntityId, OfflineActionType, OfflinePayload};
    use crate::infrastructure::offline::sqlite_store::SqliteOfflinePersistence;
    use sqlx::sqlite::SqlitePoolOptions;

    const PUBKEY: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    async fn setup_persistence() -> Arc<dyn OfflinePersistence> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        Arc::new(SqliteOfflinePersistence::new(pool))
    }

    fn sample_draft(index: u32) -> OfflineActionDraft {
        OfflineActionDraft::new(
            PublicKey::from_hex_str(PUBKEY).unwrap(),
            OfflineActionType::new("queue_test".to_string()).unwrap(),
            Some(EntityId::new(format!("post_{index}")).unwrap()),
            OfflinePayload::from_json_str(&format!("{{\"idx\": {index}}}")).unwrap(),
        )
    }

    #[tokio::test]
    async fn test_reindex_queues_unsynced_actions() {
        let persistence = setup_persistence().await;
        persistence.save_action(sample_draft(1)).await.unwrap();
        persistence.save_action(sample_draft(2)).await.unwrap();

        let job = OfflineReindexJob::with_emitter(None, persistence.clone());

        let report = job.reindex_once().await.unwrap();
        assert_eq!(report.offline_action_count, 2);
        assert_eq!(report.queued_action_count, 2);
        assert_eq!(report.pending_queue_count, 2);
        assert_eq!(report.queued_offline_action_ids.len(), 2);

        // 再実行すると新規キュー追加は発生しない
        let report_second = job.reindex_once().await.unwrap();
        assert_eq!(report_second.queued_action_count, 0);
        assert_eq!(report_second.pending_queue_count, 2);

        let unsynced_after = persistence
            .list_actions(OfflineActionFilter::new(None, Some(false), None))
            .await
            .unwrap();
        assert_eq!(unsynced_after.len(), 2);
    }
}
