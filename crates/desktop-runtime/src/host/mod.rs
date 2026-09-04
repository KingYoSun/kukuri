mod consent;

use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
};

use tokio::{sync::broadcast, task::JoinHandle};

use crate::{
    AccountRecord, CommunityNodeConfig, DesktopRuntime, RuntimeEvent, StoreStartupError,
    account_db_path, ensure_accounts_initialized_from_env, list_accounts, set_active_account,
};

pub use consent::{
    AGE_ATTESTATION_VERSION, APP_LEGAL_AUTHORITATIVE_LANGUAGE, APP_LEGAL_DOCUMENTS,
    APP_LEGAL_EFFECTIVE_DATE, AgeAttestationRecord, AgeAttestationStatus, AppConsentDocumentRecord,
    AppConsentDocumentStatus, AppConsentStore, ClientStartupErrorKind, ClientStartupErrorView,
    ClientStartupStatus, LEGAL_BUNDLE_VERSION, age_attestation_satisfied, age_attestation_status,
    app_consent_documents_satisfied, app_consent_documents_status, app_consent_path,
    app_consent_satisfied, consent_required_status, current_unix_seconds, load_app_consent_store,
    reset_app_consent_at_path, save_app_consent_store,
};

#[derive(Debug)]
pub struct ClientStartupError {
    pub kind: ClientStartupErrorKind,
    pub message: String,
}

impl ClientStartupError {
    pub fn unknown(message: String) -> Self {
        Self {
            kind: ClientStartupErrorKind::Unknown,
            message,
        }
    }

    pub fn from_error(error: anyhow::Error) -> Self {
        let kind = match error.downcast_ref::<StoreStartupError>() {
            Some(StoreStartupError::Migration(_)) => ClientStartupErrorKind::DatabaseMigration,
            Some(StoreStartupError::Open { .. }) => ClientStartupErrorKind::DatabaseOpen,
            None => ClientStartupErrorKind::Unknown,
        };
        Self {
            kind,
            message: format!("{error:#}"),
        }
    }
}

impl std::fmt::Display for ClientStartupError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

pub fn failed_startup_status(
    error: ClientStartupError,
    db_path: Option<PathBuf>,
) -> ClientStartupStatus {
    ClientStartupStatus::Failed {
        error: ClientStartupErrorView {
            kind: error.kind,
            message: "kukuri could not open the local app database.".to_string(),
            detail: error.message,
            db_path: db_path.map(|path| path.display().to_string()),
        },
    }
}

pub struct ClientStartupState {
    status: tokio::sync::watch::Sender<ClientStartupStatus>,
}

impl ClientStartupState {
    pub fn initializing() -> Self {
        Self::new(ClientStartupStatus::Initializing)
    }

    pub fn new(status: ClientStartupStatus) -> Self {
        let (status, _) = tokio::sync::watch::channel(status);
        Self { status }
    }

    pub fn status(&self) -> ClientStartupStatus {
        self.status.borrow().clone()
    }

    pub fn set_status(&self, next: ClientStartupStatus) {
        self.status.send_replace(next);
    }

    pub fn subscribe(&self) -> tokio::sync::watch::Receiver<ClientStartupStatus> {
        self.status.subscribe()
    }
}

/// UI adapterに依存せず、一つのactive account runtimeとevent転送を所有する。
pub struct ClientHost {
    runtime: RwLock<Arc<DesktopRuntime>>,
    app_data_dir: PathBuf,
    events: broadcast::Sender<RuntimeEvent>,
    event_state: Arc<std::sync::Mutex<ClientEventState>>,
    event_task: tokio::sync::Mutex<Option<JoinHandle<()>>>,
    switch_guard: tokio::sync::Mutex<()>,
    shutdown_started: AtomicBool,
}

#[derive(Default)]
struct ClientEventState {
    latest_sync_status: Option<RuntimeEvent>,
}

/// 新規購読者には直近のsync状態を一度再送し、その後のeventをbroadcastで配信する。
pub struct ClientEventReceiver {
    initial: Option<RuntimeEvent>,
    receiver: broadcast::Receiver<RuntimeEvent>,
}

impl ClientEventReceiver {
    pub async fn recv(&mut self) -> Result<RuntimeEvent, broadcast::error::RecvError> {
        if let Some(initial) = self.initial.take() {
            return Ok(initial);
        }
        self.receiver.recv().await
    }
}

impl ClientHost {
    pub async fn start(app_data_dir: PathBuf) -> Result<Arc<Self>, ClientStartupError> {
        let db_path = ensure_accounts_initialized_from_env(&app_data_dir)
            .map_err(ClientStartupError::from_error)?;
        let runtime = Self::build_detached_runtime(db_path).await?;
        Ok(Self::from_runtime(app_data_dir, runtime).await)
    }

    pub async fn from_runtime(app_data_dir: PathBuf, runtime: Arc<DesktopRuntime>) -> Arc<Self> {
        let (events, _) = broadcast::channel(64);
        let host = Arc::new(Self {
            runtime: RwLock::new(runtime.clone()),
            app_data_dir,
            events,
            event_state: Arc::new(std::sync::Mutex::new(ClientEventState::default())),
            event_task: tokio::sync::Mutex::new(None),
            switch_guard: tokio::sync::Mutex::new(()),
            shutdown_started: AtomicBool::new(false),
        });
        runtime.start_community_node_session_scheduler().await;
        host.replace_event_task(runtime).await;
        host.runtime().start_sync_status_observer().await;
        host
    }

    /// runtimeを構築するが、schedulerとobserverはまだ開始しない。
    /// `from_runtime`または`replace_runtime`へ渡してhostのevent購読後に有効化する。
    pub async fn build_detached_runtime(
        db_path: impl AsRef<Path>,
    ) -> Result<Arc<DesktopRuntime>, ClientStartupError> {
        let initial_community_node_config = distribution_community_node_config()
            .map_err(|error| ClientStartupError::unknown(error.to_string()))?;
        let runtime = DesktopRuntime::from_env(db_path, initial_community_node_config)
            .await
            .map_err(ClientStartupError::from_error)?;
        Ok(Arc::new(runtime))
    }

    pub fn app_data_dir(&self) -> &Path {
        &self.app_data_dir
    }

    pub fn runtime(&self) -> Arc<DesktopRuntime> {
        self.runtime.read().expect("runtime lock poisoned").clone()
    }

    pub fn subscribe_events(&self) -> ClientEventReceiver {
        subscribe_host_events(&self.events, &self.event_state)
    }

    pub async fn replace_runtime(&self, next: Arc<DesktopRuntime>) -> Arc<DesktopRuntime> {
        next.start_community_node_session_scheduler().await;
        let previous = std::mem::replace(
            &mut *self.runtime.write().expect("runtime lock poisoned"),
            next.clone(),
        );
        self.replace_event_task(next.clone()).await;
        next.start_sync_status_observer().await;
        previous
    }

    pub async fn restart_runtime(
        &self,
        db_path: impl AsRef<Path>,
    ) -> Result<(), ClientStartupError> {
        let next = Self::build_detached_runtime(db_path).await?;
        let previous = self.replace_runtime(next).await;
        previous.shutdown().await;
        Ok(())
    }

    pub async fn switch_account(&self, account_id: &str) -> anyhow::Result<AccountRecord> {
        let _guard = self.switch_guard.lock().await;
        let snapshot = list_accounts(&self.app_data_dir)?;
        let record = snapshot
            .accounts
            .iter()
            .find(|record| record.id == account_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("unknown account `{account_id}`"))?;
        if snapshot.active_account_id == account_id {
            return Ok(record);
        }

        let db_path = account_db_path(&self.app_data_dir, account_id);
        let next = Self::build_detached_runtime(db_path)
            .await
            .map_err(|error| anyhow::anyhow!("failed to start the account runtime: {error}"))?;
        let record = match set_active_account(&self.app_data_dir, account_id) {
            Ok(record) => record,
            Err(error) => {
                next.shutdown().await;
                return Err(error);
            }
        };
        let previous = self.replace_runtime(next).await;
        previous.shutdown().await;
        Ok(record)
    }

    pub async fn shutdown(&self) {
        if self.shutdown_started.swap(true, Ordering::AcqRel) {
            return;
        }
        if let Some(task) = self.event_task.lock().await.take() {
            task.abort();
            let _ = task.await;
        }
        self.runtime().shutdown().await;
    }

    async fn replace_event_task(&self, runtime: Arc<DesktopRuntime>) {
        let mut task = self.event_task.lock().await;
        if let Some(previous) = task.take() {
            previous.abort();
            let _ = previous.await;
        }
        let mut events = runtime.subscribe_events();
        let sender = self.events.clone();
        let event_state = self.event_state.clone();
        *task = Some(tokio::spawn(async move {
            while let Ok(event) = events.recv().await {
                forward_host_event(&sender, &event_state, event);
            }
        }));
    }
}

fn subscribe_host_events(
    sender: &broadcast::Sender<RuntimeEvent>,
    event_state: &std::sync::Mutex<ClientEventState>,
) -> ClientEventReceiver {
    let state = event_state
        .lock()
        .expect("client event state lock poisoned");
    let receiver = sender.subscribe();
    let initial = state.latest_sync_status.clone();
    ClientEventReceiver { initial, receiver }
}

fn forward_host_event(
    sender: &broadcast::Sender<RuntimeEvent>,
    event_state: &std::sync::Mutex<ClientEventState>,
    event: RuntimeEvent,
) {
    let mut state = event_state
        .lock()
        .expect("client event state lock poisoned");
    if matches!(event, RuntimeEvent::SyncStatusChanged { .. }) {
        state.latest_sync_status = Some(event.clone());
    }
    let _ = sender.send(event);
}

pub fn distribution_community_node_config() -> Result<CommunityNodeConfig, serde_json::Error> {
    serde_json::from_str(include_str!(
        "../../../../apps/desktop/src-tauri/distribution/community-nodes.json"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn late_subscriber_receives_latest_sync_status_once() {
        let (sender, _) = broadcast::channel(4);
        let event_state = std::sync::Mutex::new(ClientEventState::default());
        forward_host_event(
            &sender,
            &event_state,
            RuntimeEvent::SyncStatusChanged {
                sync_status: None,
                community_node_statuses: None,
            },
        );

        let mut receiver = subscribe_host_events(&sender, &event_state);
        assert!(matches!(
            receiver.recv().await,
            Ok(RuntimeEvent::SyncStatusChanged { .. })
        ));
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), receiver.recv())
                .await
                .is_err()
        );
    }
}
