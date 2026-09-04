use std::{io::Write, path::Path};

use kukuri_desktop_runtime::{
    DeviceBackupCancellation, DeviceRestorePhase, finalize_pending_device_restore,
    mark_device_restore_activated, mark_device_restore_awaiting_consent,
    pending_device_restore_phase, recover_interrupted_restore, rollback_pending_device_restore,
};
use tauri::Manager;

use crate::commands::background_notifications::OsNotificationBackground;
use crate::state::{
    DesktopStartupStatus, DesktopState, StartupError, build_desktop_state,
    reset_app_consent_at_path,
};

/// runtimeがまだ存在しない同意待ちでも、account switch・backup・restore activationを
/// 同じ直列化境界で扱うためのprocess-lifetime state。
#[derive(Default)]
pub(crate) struct DesktopOperationState {
    pub(crate) switch_guard: tokio::sync::Mutex<()>,
    device_backup_cancellation: DeviceBackupCancellation,
    device_backup_cancel_allowed: std::sync::Mutex<bool>,
}

impl DesktopOperationState {
    pub(crate) fn begin_cancellable_device_backup(&self) {
        let mut allowed = self
            .device_backup_cancel_allowed
            .lock()
            .expect("device backup cancel gate poisoned");
        self.device_backup_cancellation.reset();
        *allowed = true;
    }

    pub(crate) fn device_backup_cancellation(&self) -> DeviceBackupCancellation {
        self.device_backup_cancellation.clone()
    }

    /// cancel commandと同じmutex下でflagを確認してgateを閉じる。これが成功した後に
    /// 到着したcancelはInstalling transactionへ伝播しない。
    pub(crate) fn close_device_backup_cancel_gate(&self) -> anyhow::Result<()> {
        let mut allowed = self
            .device_backup_cancel_allowed
            .lock()
            .expect("device backup cancel gate poisoned");
        self.device_backup_cancellation.check()?;
        *allowed = false;
        Ok(())
    }

    pub(crate) fn cancel_device_backup(&self) {
        let allowed = self
            .device_backup_cancel_allowed
            .lock()
            .expect("device backup cancel gate poisoned");
        if *allowed {
            self.device_backup_cancellation.cancel();
        }
    }
}

/// runtimeへ触れてよいのは、公開済みruntimeとstartup statusの両方がReadyの時だけ。
pub(crate) fn runtime_access_allowed(status: &DesktopStartupStatus) -> bool {
    matches!(status, DesktopStartupStatus::Ready)
}

pub(crate) fn require_runtime_operation_ready(status: &DesktopStartupStatus) -> Result<(), String> {
    if runtime_access_allowed(status) {
        Ok(())
    } else {
        Err(format!(
            "desktop runtime operation requires Ready startup state; current state is {status:?}"
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RestoreStartupAction {
    Normal,
    ResetConsent,
    AwaitConsent,
    Activate,
    Reject(DeviceRestorePhase),
}

impl RestoreStartupAction {
    pub(crate) fn initializes_runtime(self) -> bool {
        matches!(self, Self::Normal | Self::Activate)
    }
}

pub(crate) fn restore_startup_action(
    pending_phase: Option<DeviceRestorePhase>,
    consent_satisfied: bool,
) -> RestoreStartupAction {
    match pending_phase {
        None if consent_satisfied => RestoreStartupAction::Normal,
        None => RestoreStartupAction::AwaitConsent,
        Some(DeviceRestorePhase::Committed) => RestoreStartupAction::ResetConsent,
        Some(DeviceRestorePhase::AwaitingConsent) if consent_satisfied => {
            RestoreStartupAction::Activate
        }
        Some(DeviceRestorePhase::AwaitingConsent) => RestoreStartupAction::AwaitConsent,
        Some(unexpected) => RestoreStartupAction::Reject(unexpected),
    }
}

/// 起動時の最初の永続状態操作。未完了installをrollbackしてから、残ったpending
/// phaseを返す。同意pathの解決・読取はこの関数の完了後にだけ行う。
pub(crate) fn recover_device_restore_before_startup(
    app_data_dir: &Path,
) -> Result<Option<DeviceRestorePhase>, StartupError> {
    recover_interrupted_restore(app_data_dir).map_err(|error| {
        StartupError::unknown(format!("device restore recovery failed: {error:#}"))
    })?;
    pending_device_restore_phase(app_data_dir).map_err(|error| {
        StartupError::unknown(format!(
            "failed to inspect pending device restore: {error:#}"
        ))
    })
}

pub(crate) fn advance_committed_restore_to_consent(
    app_data_dir: &Path,
    consent_db_path: &Path,
) -> Result<DesktopStartupStatus, StartupError> {
    advance_committed_restore_to_consent_with(
        || reset_app_consent_at_path(consent_db_path),
        || mark_device_restore_awaiting_consent(app_data_dir).map_err(|error| format!("{error:#}")),
    )
}

fn advance_committed_restore_to_consent_with<Reset, Mark>(
    reset_consent: Reset,
    mark_awaiting_consent: Mark,
) -> Result<DesktopStartupStatus, StartupError>
where
    Reset: FnOnce() -> Result<DesktopStartupStatus, String>,
    Mark: FnOnce() -> Result<(), String>,
{
    let status = reset_consent().map_err(|error| {
        StartupError::unknown(format!("device restore consent reset failed: {error}"))
    })?;
    mark_awaiting_consent().map_err(|error| {
        StartupError::unknown(format!(
            "failed to persist device restore consent gate: {error}"
        ))
    })?;
    Ok(status)
}

pub(crate) enum RestoreActivationFailure {
    RollbackAllowed(String),
    FinishForward(String),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RestoreActivationOrchestrationFailure {
    RolledBack(String),
    RollbackFailed(String),
    FinishForward(String),
}

async fn roll_back_failed_activation<Rollback, RollbackFuture>(
    operation_error: String,
    rollback: Rollback,
) -> RestoreActivationOrchestrationFailure
where
    Rollback: FnOnce() -> RollbackFuture,
    RollbackFuture: std::future::Future<Output = Result<(), String>>,
{
    match rollback().await {
        Ok(()) => RestoreActivationOrchestrationFailure::RolledBack(operation_error),
        Err(rollback_error) => RestoreActivationOrchestrationFailure::RollbackFailed(format!(
            "{operation_error}; additionally {rollback_error}"
        )),
    }
}

/// startup recoveryと明示的な同意commandのruntime構築、durable activation、
/// activation前rollbackを同じ順序へ集約する。rollbackできるのは`RollbackAllowed`だけで、
/// Activatedへ進んだtransactionは必ずfinish-forwardする。
pub(crate) async fn orchestrate_restore_activation<
    Restored,
    Build,
    BuildFuture,
    Activate,
    ActivateFuture,
    Rollback,
    RollbackFuture,
>(
    build: Build,
    activate: Activate,
    rollback: Rollback,
) -> Result<(), RestoreActivationOrchestrationFailure>
where
    Build: FnOnce() -> BuildFuture,
    BuildFuture: std::future::Future<Output = Result<Restored, String>>,
    Activate: FnOnce(Restored) -> ActivateFuture,
    ActivateFuture: std::future::Future<Output = Result<(), RestoreActivationFailure>>,
    Rollback: FnOnce() -> RollbackFuture,
    RollbackFuture: std::future::Future<Output = Result<(), String>>,
{
    let restored = match build().await {
        Ok(restored) => restored,
        Err(message) => {
            return Err(roll_back_failed_activation(message, rollback).await);
        }
    };
    match activate(restored).await {
        Ok(()) => Ok(()),
        Err(RestoreActivationFailure::RollbackAllowed(message)) => {
            Err(roll_back_failed_activation(message, rollback).await)
        }
        Err(RestoreActivationFailure::FinishForward(message)) => Err(
            RestoreActivationOrchestrationFailure::FinishForward(message),
        ),
    }
}

pub(crate) async fn publish_desktop_state(
    app_handle: &tauri::AppHandle,
    next: DesktopState,
) -> Result<(), String> {
    if let Some(existing) = app_handle.try_state::<DesktopState>() {
        let previous = existing.replace_runtime(next.runtime());
        drop(existing);
        drop(next);
        previous.shutdown().await;
        return Ok(());
    }

    let runtime = next.runtime();
    if app_handle.manage(next) {
        Ok(())
    } else {
        runtime.shutdown().await;
        Err("desktop runtime state was initialized concurrently".to_string())
    }
}

fn persist_restore_activation_phase(app_data_dir: &Path) -> Result<(), RestoreActivationFailure> {
    mark_device_restore_activated(app_data_dir).map_err(|error| {
        RestoreActivationFailure::RollbackAllowed(format!(
            "failed to persist restored runtime activation: {error:#}"
        ))
    })
}

/// 再同意済みのruntimeをjournalへ記録・cleanupしてから初めてglobal stateへ公開する。
pub(crate) async fn activate_pending_restore(
    app_handle: &tauri::AppHandle,
    app_data_dir: &Path,
    restored: DesktopState,
) -> Result<(), RestoreActivationFailure> {
    let runtime = restored.runtime();
    if let Err(failure) = persist_restore_activation_phase(app_data_dir) {
        runtime.shutdown().await;
        return Err(failure);
    }
    if let Err(error) = finalize_pending_device_restore(app_data_dir) {
        runtime.shutdown().await;
        return Err(RestoreActivationFailure::FinishForward(format!(
            "failed to finalize restored runtime activation: {error:#}"
        )));
    }
    if let Err(error) = publish_desktop_state(app_handle, restored).await {
        return Err(RestoreActivationFailure::FinishForward(format!(
            "failed to publish restored runtime: {error}"
        )));
    }
    app_handle
        .state::<OsNotificationBackground>()
        .reset_for_account_switch();
    Ok(())
}

/// activation前の失敗だけが使うrollback path。registry/directoryを戻した後に旧runtimeを
/// 再構築し、成功した場合だけglobal stateへ公開する。
pub(crate) async fn rollback_pending_restore_and_rebuild(
    app_handle: &tauri::AppHandle,
    app_data_dir: &Path,
) -> Result<(), String> {
    rollback_pending_device_restore(app_data_dir)
        .map_err(|error| format!("failed to roll back the pending restore: {error:#}"))?;
    let previous = build_desktop_state(app_handle)
        .await
        .map_err(|error| format!("failed to rebuild the previous runtime: {error}"))?;
    publish_desktop_state(app_handle, previous).await?;
    app_handle
        .state::<OsNotificationBackground>()
        .reset_for_account_switch();
    Ok(())
}

/// 同意fileは破損時も未同意へfail-closedするため、同一fileへの同期writeで十分。
/// journal phaseはこのfsyncが成功した後にだけ進める。
pub(crate) fn write_file_durably(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map_err(|error| {
            format!(
                "failed to open consent record `{}`: {error}",
                path.display()
            )
        })?;
    file.write_all(bytes).map_err(|error| {
        format!(
            "failed to write consent record `{}`: {error}",
            path.display()
        )
    })?;
    file.sync_all().map_err(|error| {
        format!(
            "failed to sync consent record `{}`: {error}",
            path.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{
        AGE_ATTESTATION_VERSION, APP_LEGAL_DOCUMENTS, AgeAttestationRecord,
        AppConsentDocumentRecord, AppConsentStore, DesktopStartupErrorKind, app_consent_satisfied,
        consent_required_status, failed_status, load_app_consent_store, reset_app_consent_at_path,
        save_app_consent_store,
    };

    #[test]
    fn runtime_access_is_allowed_only_for_ready() {
        assert!(runtime_access_allowed(&DesktopStartupStatus::Ready));
        assert!(!runtime_access_allowed(&DesktopStartupStatus::Initializing));
        assert!(!runtime_access_allowed(&consent_required_status(
            &Default::default()
        )));
        assert!(!runtime_access_allowed(&failed_status(
            StartupError {
                kind: DesktopStartupErrorKind::Unknown,
                message: "failed".to_string(),
            },
            None,
        )));

        assert!(require_runtime_operation_ready(&DesktopStartupStatus::Ready).is_ok());
        assert!(require_runtime_operation_ready(&DesktopStartupStatus::Initializing).is_err());
    }

    #[test]
    fn startup_action_covers_every_safe_restore_boundary() {
        assert_eq!(
            restore_startup_action(None, true),
            RestoreStartupAction::Normal
        );
        assert_eq!(
            restore_startup_action(None, false),
            RestoreStartupAction::AwaitConsent
        );
        assert_eq!(
            restore_startup_action(Some(DeviceRestorePhase::Committed), true),
            RestoreStartupAction::ResetConsent
        );
        assert_eq!(
            restore_startup_action(Some(DeviceRestorePhase::AwaitingConsent), false),
            RestoreStartupAction::AwaitConsent
        );
        assert!(
            !restore_startup_action(Some(DeviceRestorePhase::AwaitingConsent), false)
                .initializes_runtime()
        );
        assert_eq!(
            restore_startup_action(Some(DeviceRestorePhase::AwaitingConsent), true),
            RestoreStartupAction::Activate
        );
        for unexpected in [
            DeviceRestorePhase::Installing,
            DeviceRestorePhase::Installed,
            DeviceRestorePhase::Activated,
        ] {
            assert_eq!(
                restore_startup_action(Some(unexpected), false),
                RestoreStartupAction::Reject(unexpected)
            );
        }
    }

    #[test]
    fn install_cancel_gate_serializes_cancel_at_the_boundary() {
        let operation = DesktopOperationState::default();
        operation.begin_cancellable_device_backup();
        operation.cancel_device_backup();
        assert!(operation.close_device_backup_cancel_gate().is_err());

        operation.begin_cancellable_device_backup();
        operation
            .close_device_backup_cancel_gate()
            .expect("close cancel gate before install");
        operation.cancel_device_backup();
        operation
            .device_backup_cancellation()
            .check()
            .expect("cancel arriving after install boundary must be ignored");
    }

    #[test]
    fn restore_consent_reset_is_persisted_before_consent_required_status() {
        let dir = std::env::temp_dir().join(format!(
            "kukuri-consent-restore-reset-test-{}-{}",
            std::process::id(),
            crate::state::current_unix_seconds()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let db_path = dir.join("kukuri.db");
        let accepted = AppConsentStore {
            records: APP_LEGAL_DOCUMENTS
                .iter()
                .map(|(slug, version)| AppConsentDocumentRecord {
                    slug: (*slug).to_string(),
                    version: *version,
                    accepted_at: 1,
                    language: "ja".to_string(),
                    app_version: "0.1.8".to_string(),
                })
                .collect(),
            age_attestations: vec![AgeAttestationRecord {
                version: AGE_ATTESTATION_VERSION,
                attested_at: 1,
                language: "ja".to_string(),
                app_version: "0.1.8".to_string(),
            }],
        };
        save_app_consent_store(&db_path, &accepted).expect("save accepted consent");

        let status = reset_app_consent_at_path(&db_path).expect("reset restored consent");
        assert!(matches!(
            status,
            DesktopStartupStatus::ConsentRequired { .. }
        ));
        assert!(!app_consent_satisfied(&load_app_consent_store(&db_path)));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn consent_reset_failure_leaves_committed_phase_unchanged() {
        let dir = std::env::temp_dir().join(format!(
            "kukuri-consent-reset-failure-test-{}-{}",
            std::process::id(),
            crate::state::current_unix_seconds()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let blocked_parent = dir.join("not-a-directory");
        std::fs::write(&blocked_parent, b"block directory creation").expect("create blocking file");
        let db_path = blocked_parent.join("kukuri.db");
        let phase = std::cell::Cell::new(DeviceRestorePhase::Committed);
        let result = advance_committed_restore_to_consent_with(
            || reset_app_consent_at_path(&db_path),
            || {
                phase.set(DeviceRestorePhase::AwaitingConsent);
                Ok(())
            },
        );

        assert!(result.is_err());
        assert_eq!(phase.get(), DeviceRestorePhase::Committed);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn accept_activation_success_and_phase_write_failure_use_the_same_rollback_boundary() {
        let rollback_count = std::cell::Cell::new(0);
        let success = orchestrate_restore_activation(
            || std::future::ready(Ok::<_, String>(())),
            |_| std::future::ready(Ok::<_, RestoreActivationFailure>(())),
            || {
                rollback_count.set(rollback_count.get() + 1);
                std::future::ready(Ok(()))
            },
        )
        .await;
        assert_eq!(success, Ok(()));
        assert_eq!(rollback_count.get(), 0);

        let missing_journal_dir = std::env::temp_dir().join(format!(
            "kukuri-missing-restore-journal-test-{}-{}",
            std::process::id(),
            crate::state::current_unix_seconds()
        ));
        let phase_write_failure = orchestrate_restore_activation(
            || std::future::ready(Ok::<_, String>(())),
            |_| std::future::ready(persist_restore_activation_phase(&missing_journal_dir)),
            || {
                rollback_count.set(rollback_count.get() + 1);
                std::future::ready(Ok(()))
            },
        )
        .await;
        assert_eq!(
            phase_write_failure,
            Err(RestoreActivationOrchestrationFailure::RolledBack(
                "failed to persist restored runtime activation: device restore journal is missing"
                    .to_string()
            ))
        );
        assert_eq!(rollback_count.get(), 1);
    }
}
