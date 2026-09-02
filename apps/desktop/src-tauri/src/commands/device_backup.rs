use std::path::PathBuf;

use kukuri_desktop_runtime::{
    CreateDeviceBackupRequest, DeviceBackupPhase, DeviceBackupPreview, DeviceBackupProgress,
    DeviceBackupRestoreResult, DeviceBackupSummary, PreviewDeviceBackupRequest,
    RestoreDeviceBackupRequest, commit_device_restore, create_device_backup,
    finalize_device_restore, install_prepared_device_restore, prepare_device_restore,
    preview_device_backup, rollback_device_restore,
};
use tauri::{Emitter, Manager};

use crate::commands::background_notifications::OsNotificationBackground;
use crate::state::{
    CommandError, DesktopStartupState, DesktopStartupStatus, DesktopState, build_runtime, map_error,
    failed_status, reset_app_consent_after_device_restore, StartupError,
};

const PROGRESS_EVENT: &str = "kukuri://device-backup-progress";

async fn rebuild_runtime(
    app_handle: &tauri::AppHandle,
    state: &DesktopState,
    db_path: PathBuf,
) -> Result<(), CommandError> {
    let runtime = build_runtime(app_handle, db_path)
        .await
        .map_err(|error| CommandError::from(format!("failed to restart desktop runtime: {error}")))?;
    let previous = state.replace_runtime(runtime);
    drop(previous);
    app_handle
        .state::<OsNotificationBackground>()
        .reset_for_account_switch();
    Ok(())
}

async fn restore_previous_runtime(
    app_handle: &tauri::AppHandle,
    state: &DesktopState,
    startup: &DesktopStartupState,
    db_path: PathBuf,
    operation_error: CommandError,
) -> CommandError {
    let failed_db_path = db_path.clone();
    match rebuild_runtime(app_handle, state, db_path).await {
        Ok(()) => {
            startup.set_status(DesktopStartupStatus::Ready);
            operation_error
        }
        Err(restart_error) => {
            let message = format!(
                "{}; additionally failed to restart the previous runtime: {}",
                operation_error.message, restart_error.message
            );
            startup.set_status(failed_status(
                StartupError::unknown(message.clone()),
                Some(failed_db_path),
            ));
            CommandError::from(message)
        }
    }
}

#[tauri::command]
pub async fn create_device_backup_command(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, DesktopState>,
    request: CreateDeviceBackupRequest,
) -> Result<DeviceBackupSummary, CommandError> {
    let _guard = state.switch_guard.lock().await;
    state.device_backup_cancellation.reset();
    let startup = app_handle.state::<DesktopStartupState>();
    startup.set_status(DesktopStartupStatus::Initializing);

    let current = state.runtime();
    let db_path = current.db_path().to_path_buf();
    current.shutdown().await;

    let restart_path = db_path.clone();
    let app_data_dir = state.app_data_dir.clone();
    let cancellation = state.device_backup_cancellation.clone();
    let progress_app = app_handle.clone();
    let operation = tauri::async_runtime::spawn_blocking(move || {
        create_device_backup(
            &app_data_dir,
            &db_path,
            &request,
            &cancellation,
            |progress| {
                let _ = progress_app.emit(PROGRESS_EVENT, progress);
            },
        )
    })
    .await;
    let operation = match operation {
        Ok(operation) => operation,
        Err(error) => {
            return Err(
                restore_previous_runtime(
                    &app_handle,
                    &state,
                    &startup,
                    restart_path,
                    CommandError::from(format!("device backup task failed: {error}")),
                )
                .await,
            );
        }
    };

    match operation.map_err(map_error) {
        Ok(summary) => {
            match rebuild_runtime(&app_handle, &state, restart_path.clone()).await {
                Ok(()) => {
                    startup.set_status(DesktopStartupStatus::Ready);
                    Ok(summary)
                }
                Err(error) => Err(restore_previous_runtime(
                    &app_handle,
                    &state,
                    &startup,
                    restart_path,
                    error,
                )
                .await),
            }
        }
        Err(error) => Err(
            restore_previous_runtime(&app_handle, &state, &startup, restart_path, error).await,
        ),
    }
}

#[tauri::command]
pub async fn preview_device_backup_command(
    state: tauri::State<'_, DesktopState>,
    request: PreviewDeviceBackupRequest,
) -> Result<DeviceBackupPreview, CommandError> {
    let app_data_dir = state.app_data_dir.clone();
    tauri::async_runtime::spawn_blocking(move || preview_device_backup(&app_data_dir, &request))
        .await
        .map_err(|error| CommandError::from(format!("device backup preview task failed: {error}")))?
        .map_err(map_error)
}

#[tauri::command]
pub async fn restore_device_backup_command(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, DesktopState>,
    request: RestoreDeviceBackupRequest,
) -> Result<DeviceBackupRestoreResult, CommandError> {
    let _guard = state.switch_guard.lock().await;
    state.device_backup_cancellation.reset();
    let startup = app_handle.state::<DesktopStartupState>();
    startup.set_status(DesktopStartupStatus::Initializing);

    let current = state.runtime();
    let previous_db_path = current.db_path().to_path_buf();
    current.shutdown().await;

    let app_data_dir = state.app_data_dir.clone();
    let cancellation = state.device_backup_cancellation.clone();
    let progress_app = app_handle.clone();
    let prepared = tauri::async_runtime::spawn_blocking(move || {
        prepare_device_restore(
            &app_data_dir,
            &request,
            &cancellation,
            |progress| {
                let _ = progress_app.emit(PROGRESS_EVENT, progress);
            },
        )
    })
    .await;
    let prepared = match prepared {
        Ok(prepared) => prepared,
        Err(error) => {
            return Err(
                restore_previous_runtime(
                    &app_handle,
                    &state,
                    &startup,
                    previous_db_path,
                    CommandError::from(format!("device restore task failed: {error}")),
                )
                .await,
            );
        }
    };
    let prepared = match prepared {
        Ok(prepared) => prepared,
        Err(error) => {
            return Err(
                restore_previous_runtime(
                    &app_handle,
                    &state,
                    &startup,
                    previous_db_path,
                    map_error(error),
                )
                .await,
            );
        }
    };

    let validation_runtime = match build_runtime(&app_handle, prepared.staging_db_path()).await {
        Ok(runtime) => runtime,
        Err(error) => {
            drop(prepared);
            return Err(
                restore_previous_runtime(
                    &app_handle,
                    &state,
                    &startup,
                    previous_db_path,
                    CommandError::from(format!("restored account validation failed: {error}")),
                )
                .await,
            );
        }
    };
    validation_runtime.shutdown().await;
    drop(validation_runtime);
    let _ = app_handle.emit(
        PROGRESS_EVENT,
        DeviceBackupProgress {
            phase: DeviceBackupPhase::Installing,
            completed_bytes: 0,
            total_bytes: 0,
        },
    );

    let install_app_data = state.app_data_dir.clone();
    let installed = tauri::async_runtime::spawn_blocking(move || {
        install_prepared_device_restore(&install_app_data, prepared)
    })
    .await;
    let installed = match installed {
        Ok(installed) => installed,
        Err(error) => {
            return Err(
                restore_previous_runtime(
                    &app_handle,
                    &state,
                    &startup,
                    previous_db_path,
                    CommandError::from(format!("device restore install task failed: {error}")),
                )
                .await,
            );
        }
    };
    let installed = match installed {
        Ok(installed) => installed,
        Err(error) => {
            return Err(
                restore_previous_runtime(
                    &app_handle,
                    &state,
                    &startup,
                    previous_db_path,
                    map_error(error),
                )
                .await,
            );
        }
    };

    let restored_runtime = match build_runtime(&app_handle, installed.db_path()).await {
        Ok(runtime) => runtime,
        Err(error) => {
            let rollback_error = rollback_device_restore(&installed).err();
            let mut message = format!("failed to start restored account: {error}");
            if let Some(error) = rollback_error {
                message.push_str(&format!("; rollback failed: {error:#}"));
            }
            return Err(
                restore_previous_runtime(
                    &app_handle,
                    &state,
                    &startup,
                    previous_db_path,
                    CommandError::from(message),
                )
                .await,
            );
        }
    };

    let result = match commit_device_restore(&installed) {
        Ok(result) => result,
        Err(error) => {
            restored_runtime.shutdown().await;
            let rollback_error = rollback_device_restore(&installed).err();
            let mut message = format!("failed to commit restored account: {error:#}");
            if let Some(error) = rollback_error {
                message.push_str(&format!("; rollback failed: {error:#}"));
            }
            return Err(
                restore_previous_runtime(
                    &app_handle,
                    &state,
                    &startup,
                    previous_db_path,
                    CommandError::from(message),
                )
                .await,
            );
        }
    };

    let consent_status = match reset_app_consent_after_device_restore(&app_handle) {
        Ok(status) => status,
        Err(error) => {
            restored_runtime.shutdown().await;
            let rollback_error = rollback_device_restore(&installed).err();
            let mut message = format!("failed to reset consent after device restore: {error}");
            if let Some(error) = rollback_error {
                message.push_str(&format!("; rollback failed: {error:#}"));
            }
            return Err(
                restore_previous_runtime(
                    &app_handle,
                    &state,
                    &startup,
                    previous_db_path,
                    CommandError::from(message),
                )
                .await,
            );
        }
    };

    let previous = state.replace_runtime(restored_runtime);
    drop(previous);
    app_handle
        .state::<OsNotificationBackground>()
        .reset_for_account_switch();
    startup.set_status(consent_status);
    if let Err(error) = finalize_device_restore(installed) {
        tracing::warn!(%error, "device restore committed but cleanup was deferred");
    }
    let _ = app_handle.emit(
        PROGRESS_EVENT,
        DeviceBackupProgress {
            phase: DeviceBackupPhase::Installing,
            completed_bytes: 1,
            total_bytes: 1,
        },
    );
    Ok(result)
}

#[tauri::command]
pub fn cancel_device_backup(state: tauri::State<'_, DesktopState>) {
    state.device_backup_cancellation.cancel();
}
