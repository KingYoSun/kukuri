use crate::application::services::AuthService;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub success: bool,
    pub npub: String,
    pub pubkey: String,
}

#[tauri::command]
pub async fn create_account(
    auth_service: State<'_, Arc<AuthService>>,
) -> Result<LoginResponse, String> {
    let user = auth_service
        .create_account()
        .await
        .map_err(|e| e.to_string())?;

    Ok(LoginResponse {
        success: true,
        npub: user.npub,
        pubkey: user.pubkey,
    })
}

#[tauri::command]
pub async fn login_with_nsec(
    nsec: String,
    auth_service: State<'_, Arc<AuthService>>,
) -> Result<LoginResponse, String> {
    let user = auth_service
        .login_with_nsec(&nsec)
        .await
        .map_err(|e| e.to_string())?;

    Ok(LoginResponse {
        success: true,
        npub: user.npub,
        pubkey: user.pubkey,
    })
}

#[tauri::command]
pub async fn login_with_npub(
    npub: String,
    auth_service: State<'_, Arc<AuthService>>,
) -> Result<LoginResponse, String> {
    let user = auth_service
        .login_with_npub(&npub)
        .await
        .map_err(|e| e.to_string())?;

    Ok(LoginResponse {
        success: true,
        npub: user.npub,
        pubkey: user.pubkey,
    })
}

#[tauri::command]
pub async fn logout(auth_service: State<'_, Arc<AuthService>>) -> Result<(), String> {
    auth_service.logout().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_current_user(
    auth_service: State<'_, Arc<AuthService>>,
) -> Result<Option<serde_json::Value>, String> {
    let user = auth_service
        .get_current_user()
        .await
        .map_err(|e| e.to_string())?;

    Ok(user.map(|u| serde_json::to_value(u).unwrap()))
}

#[tauri::command]
pub async fn is_authenticated(auth_service: State<'_, Arc<AuthService>>) -> Result<bool, String> {
    Ok(auth_service.is_authenticated().await)
}

#[tauri::command]
pub async fn export_private_key(
    npub: String,
    auth_service: State<'_, Arc<AuthService>>,
) -> Result<String, String> {
    auth_service
        .export_private_key(&npub)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_accounts(
    auth_service: State<'_, Arc<AuthService>>,
) -> Result<Vec<String>, String> {
    auth_service
        .list_accounts()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn switch_account(
    npub: String,
    auth_service: State<'_, Arc<AuthService>>,
) -> Result<LoginResponse, String> {
    let user = auth_service
        .switch_account(&npub)
        .await
        .map_err(|e| e.to_string())?;

    Ok(LoginResponse {
        success: true,
        npub: user.npub,
        pubkey: user.pubkey,
    })
}