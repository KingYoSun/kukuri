//! #859: アカウント鍵の export / import と複数アカウントの一覧・切替。
//!
//! 平文秘密鍵は IPC に載せない(export は暗号化 envelope のみ)。KDF(argon2id)は
//! 数百 ms ブロックするため `spawn_blocking` で実行する。

use kukuri_desktop_runtime::{
    AccountKeyExport, AccountKeyImportPreview, AccountRecord, AccountsSnapshot,
    ExportAccountKeyRequest, ImportAccountKeyRequest, PreviewAccountKeyImportRequest,
    SwitchAccountRequest, account_db_path,
};
use tauri::Manager;

use crate::commands::background_notifications::OsNotificationBackground;
use crate::restore_lifecycle::{DesktopOperationState, require_runtime_operation_ready};
use crate::state::{
    CommandError, DesktopStartupState, DesktopStartupStatus, DesktopState, build_runtime, map_error,
};

#[tauri::command]
pub async fn export_account_key(
    state: tauri::State<'_, DesktopState>,
    request: ExportAccountKeyRequest,
) -> Result<AccountKeyExport, CommandError> {
    let runtime = state.runtime();
    tauri::async_runtime::spawn_blocking(move || runtime.export_account_key(request))
        .await
        .map_err(|error| CommandError::from(format!("export task failed: {error}")))?
        .map_err(map_error)
}

#[tauri::command]
pub async fn preview_account_key_import(
    state: tauri::State<'_, DesktopState>,
    request: PreviewAccountKeyImportRequest,
) -> Result<AccountKeyImportPreview, CommandError> {
    kukuri_desktop_runtime::preview_account_key_import(&state.app_data_dir, &request.export)
        .map_err(map_error)
}

#[tauri::command]
pub async fn import_account_key(
    state: tauri::State<'_, DesktopState>,
    request: ImportAccountKeyRequest,
) -> Result<AccountRecord, CommandError> {
    let app_data_dir = state.app_data_dir.clone();
    tauri::async_runtime::spawn_blocking(move || {
        kukuri_desktop_runtime::import_account_key_from_env(
            &app_data_dir,
            &request.export,
            &request.passphrase,
            request.label,
        )
    })
    .await
    .map_err(|error| CommandError::from(format!("import task failed: {error}")))?
    .map_err(map_error)
}

#[tauri::command]
pub async fn list_accounts(
    state: tauri::State<'_, DesktopState>,
) -> Result<AccountsSnapshot, CommandError> {
    kukuri_desktop_runtime::list_accounts(&state.app_data_dir).map_err(map_error)
}

/// アクティブアカウントを切り替える。新しい runtime の構築に成功してから registry と
/// state を更新し、失敗時は旧 runtime をそのまま生かす(アプリ再起動は不要)。
#[tauri::command]
pub async fn switch_account(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, DesktopState>,
    operation: tauri::State<'_, DesktopOperationState>,
    request: SwitchAccountRequest,
) -> Result<AccountRecord, CommandError> {
    let _guard = operation.switch_guard.lock().await;
    let startup = app_handle.state::<DesktopStartupState>();
    require_runtime_operation_ready(&startup.status()).map_err(CommandError::from)?;

    let snapshot = kukuri_desktop_runtime::list_accounts(&state.app_data_dir).map_err(map_error)?;
    let record = snapshot
        .accounts
        .iter()
        .find(|record| record.id == request.account_id)
        .cloned()
        .ok_or_else(|| CommandError::from(format!("unknown account `{}`", request.account_id)))?;
    if snapshot.active_account_id == request.account_id {
        return Ok(record);
    }

    startup.set_status(DesktopStartupStatus::Initializing);

    let db_path = account_db_path(&state.app_data_dir, request.account_id.as_str());
    let new_runtime = match build_runtime(&app_handle, db_path).await {
        Ok(runtime) => runtime,
        Err(error) => {
            // 旧 runtime は無傷のまま残っている。
            startup.set_status(DesktopStartupStatus::Ready);
            return Err(CommandError::from(format!(
                "failed to start the account runtime: {error}"
            )));
        }
    };

    let record = match kukuri_desktop_runtime::set_active_account(
        &state.app_data_dir,
        request.account_id.as_str(),
    ) {
        Ok(record) => record,
        Err(error) => {
            new_runtime.shutdown().await;
            startup.set_status(DesktopStartupStatus::Ready);
            return Err(map_error(error));
        }
    };

    let previous = state.replace_runtime(new_runtime);
    previous.shutdown().await;
    app_handle
        .state::<OsNotificationBackground>()
        .reset_for_account_switch();
    startup.set_status(DesktopStartupStatus::Ready);
    Ok(record)
}
