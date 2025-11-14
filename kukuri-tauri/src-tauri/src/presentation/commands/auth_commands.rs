use crate::{
    presentation::{
        dto::{
            ApiResponse,
            auth_dto::{LoginResponse, LoginWithNsecRequest},
        },
        handlers::AuthHandler,
    },
    shared::AppError,
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
) -> Result<ApiResponse<GenerateKeypairResponse>, AppError> {
    let handler = AuthHandler::new(state.auth_service.clone());
    let response = handler.create_account().await?;

    Ok(ApiResponse::success(GenerateKeypairResponse {
        public_key: response.pubkey,
        nsec: response.nsec,
        npub: response.npub,
    }))
}

/// nsecで既存アカウントにログイン
#[tauri::command]
pub async fn login(
    state: State<'_, AppState>,
    request: LoginRequest,
) -> Result<ApiResponse<LoginResponse>, AppError> {
    let handler = AuthHandler::new(state.auth_service.clone());
    let login_request = LoginWithNsecRequest { nsec: request.nsec };

    let result = handler.login_with_nsec(login_request).await;
    Ok(ApiResponse::from_result(result))
}

/// ログアウト
#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> Result<ApiResponse<()>, AppError> {
    let handler = AuthHandler::new(state.auth_service.clone());
    let current_user = handler.get_current_user().await?;

    if let Some(user) = current_user {
        handler.logout(user.npub).await?;
    }

    Ok(ApiResponse::success(()))
}
