use crate::{
    presentation::{
        dto::{
            auth_dto::{CreateAccountResponse, LoginResponse, LoginWithNsecRequest},
        },
        handlers::AuthHandler,
    },
    state::AppState,
};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateKeypairResponse {
    pub public_key: String,
    pub nsec: String,
    pub npub: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub nsec: String,
}

/// 新しいキーペアを生成する
#[tauri::command]
pub async fn generate_keypair(
    state: State<'_, AppState>,
) -> Result<GenerateKeypairResponse, String> {
    let handler = AuthHandler::new(state.auth_service.clone());
    let response = handler
        .create_account()
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(GenerateKeypairResponse {
        public_key: response.pubkey,
        nsec: response.nsec,
        npub: response.npub,
    })
}

/// nsecで既存アカウントにログイン
#[tauri::command]
pub async fn login(
    state: State<'_, AppState>,
    request: LoginRequest,
) -> Result<LoginResponse, String> {
    let handler = AuthHandler::new(state.auth_service.clone());
    let login_request = LoginWithNsecRequest {
        nsec: request.nsec,
    };
    
    handler
        .login_with_nsec(login_request)
        .await
        .map_err(|e| e.to_string())
}

/// ログアウト
#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> Result<(), String> {
    let handler = AuthHandler::new(state.auth_service.clone());
    // 現在のユーザーを取得してからログアウト
    let current_user = handler
        .get_current_user()
        .await
        .map_err(|e| e.to_string())?;
    
    if let Some(user) = current_user {
        handler
            .logout(user.npub)
            .await
            .map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

/// アカウント作成（旧APIとの互換性のため）
#[tauri::command]
pub async fn create_account(
    state: State<'_, AppState>,
) -> Result<LoginResponse, String> {
    let user = state.auth_service
        .create_account()
        .await
        .map_err(|e| e.to_string())?;

    Ok(LoginResponse {
        success: true,
        npub: user.npub,
        pubkey: user.pubkey,
    })
}

/// nsecでログイン（旧APIとの互換性のため）
#[tauri::command]
pub async fn login_with_nsec(
    nsec: String,
    state: State<'_, AppState>,
) -> Result<LoginResponse, String> {
    let user = state.auth_service
        .login_with_nsec(&nsec)
        .await
        .map_err(|e| e.to_string())?;

    Ok(LoginResponse {
        success: true,
        npub: user.npub,
        pubkey: user.pubkey,
    })
}

/// npubでログイン
#[tauri::command]
pub async fn login_with_npub(
    npub: String,
    state: State<'_, AppState>,
) -> Result<LoginResponse, String> {
    let user = state.auth_service
        .login_with_npub(&npub)
        .await
        .map_err(|e| e.to_string())?;

    Ok(LoginResponse {
        success: true,
        npub: user.npub,
        pubkey: user.pubkey,
    })
}

/// 現在のユーザーを取得
#[tauri::command]
pub async fn get_current_user(
    state: State<'_, AppState>,
) -> Result<Option<serde_json::Value>, String> {
    let user = state.auth_service
        .get_current_user()
        .await
        .map_err(|e| e.to_string())?;

    Ok(user.map(|u| serde_json::to_value(u).unwrap()))
}

/// 認証状態を確認
#[tauri::command]
pub async fn is_authenticated(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.auth_service.is_authenticated().await)
}

/// 秘密鍵をエクスポート
#[tauri::command]
pub async fn export_private_key(
    npub: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    state.auth_service
        .export_private_key(&npub)
        .await
        .map_err(|e| e.to_string())
}