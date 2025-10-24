use crate::{
    domain::entities::AccountMetadata,
    presentation::{
        dto::{ApiResponse, auth_dto::LoginResponse},
        handlers::secure_storage_handler::{
            AddAccountRequest, AddAccountResponse, GetCurrentAccountResponse, SecureStorageHandler,
            SwitchAccountResponse,
        },
    },
    shared::AppError,
    state::AppState,
};
use tauri::State;

/// アカウントを追加
#[tauri::command]
pub async fn add_account(
    state: State<'_, AppState>,
    request: AddAccountRequest,
) -> Result<ApiResponse<AddAccountResponse>, AppError> {
    let handler = state.secure_storage_handler.clone();
    let result = handler.add_account(request).await;
    Ok(ApiResponse::from_result(result))
}

/// アカウント一覧を取得
#[tauri::command]
pub async fn list_accounts(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<AccountMetadata>>, AppError> {
    let handler = state.secure_storage_handler.clone();
    let result = handler.list_accounts().await;
    Ok(ApiResponse::from_result(result))
}

/// アカウントを切り替え
#[tauri::command]
pub async fn switch_account(
    state: State<'_, AppState>,
    npub: String,
) -> Result<ApiResponse<SwitchAccountResponse>, AppError> {
    let handler = state.secure_storage_handler.clone();
    let result = handler.switch_account(npub).await;
    Ok(ApiResponse::from_result(result))
}

/// アカウントを削除
#[tauri::command]
pub async fn remove_account(
    state: State<'_, AppState>,
    npub: String,
) -> Result<ApiResponse<()>, AppError> {
    let handler = state.secure_storage_handler.clone();
    let result = handler.remove_account(npub).await;
    Ok(ApiResponse::from_result(result))
}

/// 現在のアカウントを取得
#[tauri::command]
pub async fn get_current_account(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Option<GetCurrentAccountResponse>>, AppError> {
    let handler = state.secure_storage_handler.clone();
    let result = handler.get_current_account().await;
    Ok(ApiResponse::from_result(result))
}

/// セキュアログイン
#[tauri::command]
pub async fn secure_login(
    state: State<'_, AppState>,
    npub: String,
) -> Result<ApiResponse<LoginResponse>, AppError> {
    let handler = state.secure_storage_handler.clone();
    let result = handler.secure_login(npub).await;
    Ok(ApiResponse::from_result(result))
}

/// 全てのアカウントデータをクリア（テスト用）
#[tauri::command]
pub async fn clear_all_accounts_for_test(
    _state: State<'_, AppState>,
) -> Result<ApiResponse<()>, AppError> {
    #[cfg(not(debug_assertions))]
    {
        return Err(AppError::ConfigurationError(
            "This command is only available in debug builds".to_string(),
        ));
    }

    #[cfg(debug_assertions)]
    {
        use crate::modules::secure_storage::SecureStorage;

        SecureStorage::clear_all_accounts().map_err(|e| AppError::Storage(e.to_string()))?;
        Ok(ApiResponse::success(()))
    }
}
