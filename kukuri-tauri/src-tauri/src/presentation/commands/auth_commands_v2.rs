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

/// 新しいキーペアを生成する（旧generate_keypairコマンドの互換実装）
#[tauri::command]
pub async fn generate_keypair_v2(
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

/// nsecで既存アカウントにログイン（旧loginコマンドの互換実装）
#[tauri::command]
pub async fn login_v2(
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

/// ログアウト（旧logoutコマンドの互換実装）
#[tauri::command]
pub async fn logout_v2(state: State<'_, AppState>) -> Result<(), String> {
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