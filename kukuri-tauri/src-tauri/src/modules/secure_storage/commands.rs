use crate::modules::auth::commands::LoginResponse;
use crate::modules::secure_storage::{AccountMetadata, SecureStorage};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct AddAccountRequest {
    pub nsec: String,
    pub name: String,
    pub display_name: String,
    pub picture: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddAccountResponse {
    pub npub: String,
    pub pubkey: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwitchAccountResponse {
    pub npub: String,
    pub pubkey: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetCurrentAccountResponse {
    pub npub: String,
    pub nsec: String,
    pub pubkey: String,
    pub metadata: AccountMetadata,
}

#[tauri::command]
pub async fn add_account(
    state: State<'_, AppState>,
    request: AddAccountRequest,
) -> Result<AddAccountResponse, String> {
    // nsecから公開鍵とnpubを生成
    let (pubkey, npub) = state
        .key_manager
        .login(&request.nsec)
        .await
        .map_err(|e| e.to_string())?;

    println!("Adding account: npub={}, pubkey={}", npub, pubkey);

    // セキュアストレージに保存
    SecureStorage::add_account(
        &npub,
        &request.nsec,
        &pubkey,
        &request.name,
        &request.display_name,
        request.picture,
    )
    .map_err(|e| e.to_string())?;

    println!("Account saved to secure storage successfully");

    Ok(AddAccountResponse { npub, pubkey })
}

#[tauri::command]
pub async fn list_accounts() -> Result<Vec<AccountMetadata>, String> {
    SecureStorage::list_accounts().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn switch_account(
    state: State<'_, AppState>,
    npub: String,
) -> Result<SwitchAccountResponse, String> {
    // アカウントを切り替え
    SecureStorage::switch_account(&npub).map_err(|e| e.to_string())?;

    // 秘密鍵を取得してログイン
    let nsec = SecureStorage::get_private_key(&npub)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Private key not found".to_string())?;

    let (pubkey, _) = state
        .key_manager
        .login(&nsec)
        .await
        .map_err(|e| e.to_string())?;

    Ok(SwitchAccountResponse { npub, pubkey })
}

#[tauri::command]
pub async fn remove_account(npub: String) -> Result<(), String> {
    SecureStorage::remove_account(&npub).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_current_account(
    state: State<'_, AppState>,
) -> Result<Option<GetCurrentAccountResponse>, String> {
    println!("Getting current account from secure storage...");
    
    // 現在のアカウント情報を取得
    if let Some((npub, nsec)) =
        SecureStorage::get_current_private_key().map_err(|e| e.to_string())?
    {
        println!("Found current account: npub={}", npub);
        
        // メタデータを取得
        let metadata = SecureStorage::get_accounts_metadata().map_err(|e| e.to_string())?;

        if let Some(account_metadata) = metadata.accounts.get(&npub) {
            // ログイン処理
            let (pubkey, _) = state
                .key_manager
                .login(&nsec)
                .await
                .map_err(|e| e.to_string())?;

            println!("Successfully loaded account metadata for npub={}", npub);

            Ok(Some(GetCurrentAccountResponse {
                npub,
                nsec,
                pubkey,
                metadata: account_metadata.clone(),
            }))
        } else {
            println!("No metadata found for npub={}", npub);
            Ok(None)
        }
    } else {
        println!("No current account found in secure storage");
        Ok(None)
    }
}

#[tauri::command]
pub async fn secure_login(
    state: State<'_, AppState>,
    npub: String,
) -> Result<LoginResponse, String> {
    // セキュアストレージから秘密鍵を取得
    let nsec = SecureStorage::get_private_key(&npub)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Private key not found".to_string())?;

    // アカウントを切り替え
    SecureStorage::switch_account(&npub).map_err(|e| e.to_string())?;

    // ログイン処理
    let (public_key, _) = state
        .key_manager
        .login(&nsec)
        .await
        .map_err(|e| e.to_string())?;

    Ok(LoginResponse { public_key, npub })
}
