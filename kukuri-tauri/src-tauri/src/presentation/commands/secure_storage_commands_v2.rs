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

/// アカウントを追加（旧add_accountコマンドの互換実装）
#[tauri::command]
pub async fn add_account_v2(
    state: State<'_, AppState>,
    request: AddAccountRequest,
) -> Result<AddAccountResponse, String> {
    let handler = SecureStorageHandler::new(state.auth_service.clone());
    
    handler
        .add_account(request)
        .await
        .map_err(|e| e.to_string())
}

/// アカウント一覧を取得（旧list_accountsコマンドの互換実装）
#[tauri::command]
pub async fn list_accounts_v2(
    state: State<'_, AppState>,
) -> Result<Vec<AccountMetadata>, String> {
    let handler = SecureStorageHandler::new(state.auth_service.clone());
    
    handler
        .list_accounts()
        .await
        .map_err(|e| e.to_string())
}

/// アカウントを切り替え（旧switch_accountコマンドの互換実装）
#[tauri::command]
pub async fn switch_account_v2(
    state: State<'_, AppState>,
    npub: String,
) -> Result<SwitchAccountResponse, String> {
    let handler = SecureStorageHandler::new(state.auth_service.clone());
    
    handler
        .switch_account(npub)
        .await
        .map_err(|e| e.to_string())
}

/// アカウントを削除（旧remove_accountコマンドの互換実装）
#[tauri::command]
pub async fn remove_account_v2(
    state: State<'_, AppState>,
    npub: String,
) -> Result<(), String> {
    let handler = SecureStorageHandler::new(state.auth_service.clone());
    
    handler
        .remove_account(npub)
        .await
        .map_err(|e| e.to_string())
}

/// 現在のアカウントを取得（旧get_current_accountコマンドの互換実装）
#[tauri::command]
pub async fn get_current_account_v2(
    state: State<'_, AppState>,
) -> Result<Option<GetCurrentAccountResponse>, String> {
    let handler = SecureStorageHandler::new(state.auth_service.clone());
    
    handler
        .get_current_account()
        .await
        .map_err(|e| e.to_string())
}

/// セキュアログイン（旧secure_loginコマンドの互換実装）
#[tauri::command]
pub async fn secure_login_v2(
    state: State<'_, AppState>,
    npub: String,
) -> Result<LoginResponse, String> {
    let handler = SecureStorageHandler::new(state.auth_service.clone());
    
    handler
        .secure_login(npub)
        .await
        .map_err(|e| e.to_string())
}