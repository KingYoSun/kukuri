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

use crate::modules::offline::OfflineManager;
use crate::modules::offline::models::{GetOfflineActionsRequest, SyncStatusRecord};
use crate::shared::error::AppError;

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
    offline_manager: Arc<OfflineManager>,
    gate: Mutex<()>,
}

impl OfflineReindexJob {
    pub fn create(
        app_handle: Option<tauri::AppHandle>,
        offline_manager: Arc<OfflineManager>,
    ) -> Arc<Self> {
        let emitter = app_handle
            .map(|handle| Arc::new(TauriEventEmitter { handle }) as Arc<dyn ReindexEventEmitter>);
        Self::with_emitter(emitter, offline_manager)
    }

    pub fn with_emitter(
        event_emitter: Option<Arc<dyn ReindexEventEmitter>>,
        offline_manager: Arc<OfflineManager>,
    ) -> Arc<Self> {
        Arc::new(Self {
            event_emitter,
            offline_manager,
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
            .offline_manager
            .get_offline_actions(GetOfflineActionsRequest {
                user_pubkey: None,
                is_synced: Some(false),
                limit: None,
            })
            .await
            .map_err(AppError::from)?;

        let mut queued_action_count = 0usize;
        let mut queued_local_ids = Vec::new();

        for action in &unsynced {
            if self
                .offline_manager
                .ensure_offline_action_in_queue(action)
                .await
                .map_err(AppError::from)?
            {
                queued_action_count += 1;
                queued_local_ids.push(action.local_id.clone());
            }
        }

        let pending_queue = self
            .offline_manager
            .get_pending_sync_queue()
            .await
            .map_err(AppError::from)?;

        let stale_cache = self
            .offline_manager
            .get_stale_cache_entries()
            .await
            .map_err(AppError::from)?;
        let stale_cache_keys = stale_cache
            .into_iter()
            .map(|entry| entry.cache_key)
            .collect();

        let optimistic_updates = self
            .offline_manager
            .get_unconfirmed_updates()
            .await
            .map_err(AppError::from)?;
        let optimistic_update_ids = optimistic_updates
            .into_iter()
            .map(|item| item.update_id)
            .collect();

        let conflicts = self
            .offline_manager
            .get_sync_conflicts()
            .await
            .map_err(AppError::from)?;
        let conflict_digests = conflicts
            .into_iter()
            .map(SyncConflictDigest::from)
            .collect();

        let report = OfflineReindexReport {
            offline_action_count: unsynced.len(),
            queued_action_count,
            pending_queue_count: pending_queue.len(),
            stale_cache_keys,
            optimistic_update_ids,
            sync_conflicts: conflict_digests,
            queued_offline_action_ids: queued_local_ids,
            emitted_at: Utc::now().timestamp_millis(),
        };

        Ok(report)
    }

    async fn run_guarded(self: Arc<Self>) {
        let _guard = self.gate.lock().await;
        match self.reindex_once().await {
            Ok(report) => self.emit_success(&report),
            Err(err) => self.emit_failure(&err.to_string()),
        }
    }

    fn emit_success(&self, report: &OfflineReindexReport) {
        if let Some(emitter) = &self.event_emitter {
            if let Err(err) = emitter.emit_report(report) {
                tracing::warn!(
                    target: "offline::reindex",
                    error = %err,
                    "failed to emit offline reindex completion event"
                );
            }
        }
        tracing::info!(
            target: "offline::reindex",
            queued = report.queued_action_count,
            pending = report.pending_queue_count,
            "offline reindex completed"
        );
    }

    fn emit_failure(&self, message: &str) {
        if let Some(emitter) = &self.event_emitter {
            if let Err(err) = emitter.emit_failure(message) {
                tracing::warn!(
                    target: "offline::reindex",
                    error = %err,
                    "failed to emit offline reindex failure event"
                );
            }
        }
        tracing::error!(
            target: "offline::reindex",
            error = message,
            "offline reindex failed"
        );
    }
}

impl From<SyncStatusRecord> for SyncConflictDigest {
    fn from(value: SyncStatusRecord) -> Self {
        Self {
            entity_type: value.entity_type,
            entity_id: value.entity_id,
            sync_status: value.sync_status,
        }
    }
}
