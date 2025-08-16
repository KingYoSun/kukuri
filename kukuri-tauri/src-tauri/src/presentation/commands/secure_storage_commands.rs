use crate::{
    presentation::{
        handlers::secure_storage_handler::{
            SecureStorageHandler, AddAccountRequest, AddAccountResponse,
            SwitchAccountResponse, GetCurrentAccountResponse,
        },
        dto::auth_dto::LoginResponse,
    },
    infrastructure::storage::secure_storage::AccountMetadata,
    state::AppState,
};
use tauri::State;

/// アカウントを追加
#[tauri::command]
pub async fn add_account(
    state: State<'_, AppState>,
    request: AddAccountRequest,
) -> Result<AddAccountResponse, String> {
    let handler = SecureStorageHandler::new(state.auth_service.clone());
    
    handler
        .add_account(request)
        .await
        .map_err(|e| e.to_string())
}

/// アカウント一覧を取得
#[tauri::command]
pub async fn list_accounts(
    state: State<'_, AppState>,
) -> Result<Vec<AccountMetadata>, String> {
    let handler = SecureStorageHandler::new(state.auth_service.clone());
    
    handler
        .list_accounts()
        .await
        .map_err(|e| e.to_string())
}

/// アカウントを切り替え
#[tauri::command]
pub async fn switch_account(
    state: State<'_, AppState>,
    npub: String,
) -> Result<SwitchAccountResponse, String> {
    let handler = SecureStorageHandler::new(state.auth_service.clone());
    
    handler
        .switch_account(npub)
        .await
        .map_err(|e| e.to_string())
}

/// アカウントを削除
#[tauri::command]
pub async fn remove_account(
    state: State<'_, AppState>,
    npub: String,
) -> Result<(), String> {
    let handler = SecureStorageHandler::new(state.auth_service.clone());
    
    handler
        .remove_account(npub)
        .await
        .map_err(|e| e.to_string())
}

/// 現在のアカウントを取得
#[tauri::command]
pub async fn get_current_account(
    state: State<'_, AppState>,
) -> Result<Option<GetCurrentAccountResponse>, String> {
    let handler = SecureStorageHandler::new(state.auth_service.clone());
    
    handler
        .get_current_account()
        .await
        .map_err(|e| e.to_string())
}

/// セキュアログイン
#[tauri::command]
pub async fn secure_login(
    state: State<'_, AppState>,
    npub: String,
) -> Result<LoginResponse, String> {
    let handler = SecureStorageHandler::new(state.auth_service.clone());
    
    handler
        .secure_login(npub)
        .await
        .map_err(|e| e.to_string())
}

/// 全てのアカウントデータをクリア（テスト用）
#[tauri::command]
pub async fn clear_all_accounts_for_test(
    _state: State<'_, AppState>,
) -> Result<(), String> {
    use crate::modules::secure_storage::SecureStorage;
    
    // プロダクションビルドでは実行を拒否
    #[cfg(not(debug_assertions))]
    {
        return Err("This command is only available in debug builds".to_string());
    }
    
    SecureStorage::clear_all_accounts()
        .map_err(|e| e.to_string())
}