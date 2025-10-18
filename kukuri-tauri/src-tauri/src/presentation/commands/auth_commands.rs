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
use serde_json::Value;
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

/// アカウント作成（旧APIとの互換性のため）
#[tauri::command]
pub async fn create_account(
    state: State<'_, AppState>,
) -> Result<ApiResponse<LoginResponse>, AppError> {
    let user = state.auth_service.create_account().await?;

    Ok(ApiResponse::success(LoginResponse {
        success: true,
        npub: user.npub,
        pubkey: user.pubkey,
    }))
}

/// nsecでログイン（旧APIとの互換性のため）
#[tauri::command]
pub async fn login_with_nsec(
    nsec: String,
    state: State<'_, AppState>,
) -> Result<ApiResponse<LoginResponse>, AppError> {
    let user = state.auth_service.login_with_nsec(&nsec).await?;

    Ok(ApiResponse::success(LoginResponse {
        success: true,
        npub: user.npub,
        pubkey: user.pubkey,
    }))
}

/// npubでログイン
#[tauri::command]
pub async fn login_with_npub(
    npub: String,
    state: State<'_, AppState>,
) -> Result<ApiResponse<LoginResponse>, AppError> {
    let user = state.auth_service.login_with_npub(&npub).await?;

    Ok(ApiResponse::success(LoginResponse {
        success: true,
        npub: user.npub,
        pubkey: user.pubkey,
    }))
}

/// 現在のユーザーを取得
#[tauri::command]
pub async fn get_current_user(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Option<Value>>, AppError> {
    let result = state
        .auth_service
        .get_current_user()
        .await
        .map_err(AppError::from)
        .and_then(|user| {
            user.map(|u| serde_json::to_value(u).map_err(AppError::from))
                .transpose()
        });
    Ok(ApiResponse::from_result(result))
}

/// 認証状態を確認
#[tauri::command]
pub async fn is_authenticated(state: State<'_, AppState>) -> Result<ApiResponse<bool>, AppError> {
    Ok(ApiResponse::success(
        state.auth_service.is_authenticated().await,
    ))
}

/// 秘密鍵をエクスポート
#[tauri::command]
pub async fn export_private_key(
    npub: String,
    state: State<'_, AppState>,
) -> Result<ApiResponse<String>, AppError> {
    let result = state
        .auth_service
        .export_private_key(&npub)
        .await
        .map_err(AppError::from);
    Ok(ApiResponse::from_result(result))
}
