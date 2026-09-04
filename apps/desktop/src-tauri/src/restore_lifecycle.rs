use std::path::Path;

use kukuri_desktop_runtime::{finalize_pending_device_restore, rollback_pending_device_restore};
use tauri::Manager;

use crate::commands::background_notifications::OsNotificationBackground;
use crate::state::{DesktopState, build_desktop_state};
use kukuri_desktop_runtime::persist_restore_activation_phase;
pub(crate) use kukuri_desktop_runtime::{
    ClientOperationState as DesktopOperationState, RestoreActivationFailure,
    RestoreActivationOrchestrationFailure, RestoreStartupAction,
    advance_committed_restore_to_consent, orchestrate_restore_activation,
    recover_device_restore_before_startup, require_runtime_operation_ready, restore_startup_action,
    runtime_access_allowed,
};

pub(crate) async fn publish_desktop_state(
    app_handle: &tauri::AppHandle,
    next: DesktopState,
) -> Result<(), String> {
    if let Some(existing) = app_handle.try_state::<DesktopState>() {
        let previous = existing.replace_host(next.host());
        drop(existing);
        drop(next);
        previous.shutdown().await;
        return Ok(());
    }

    let host = next.host();
    if app_handle.manage(next) {
        Ok(())
    } else {
        host.shutdown().await;
        Err("desktop runtime state was initialized concurrently".to_string())
    }
}

/// 再同意済みのruntimeをjournalへ記録・cleanupしてから初めてglobal stateへ公開する。
pub(crate) async fn activate_pending_restore(
    app_handle: &tauri::AppHandle,
    app_data_dir: &Path,
    restored: DesktopState,
) -> Result<(), RestoreActivationFailure> {
    let host = restored.host();
    if let Err(failure) = persist_restore_activation_phase(app_data_dir) {
        host.shutdown().await;
        return Err(failure);
    }
    if let Err(error) = finalize_pending_device_restore(app_data_dir) {
        host.shutdown().await;
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
