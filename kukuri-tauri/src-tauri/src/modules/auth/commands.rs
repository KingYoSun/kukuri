use serde::{Deserialize, Serialize};
use tauri::State;
use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateKeypairResponse {
    pub public_key: String,
    pub nsec: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub nsec: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub public_key: String,
    pub npub: String,
}

#[tauri::command]
pub async fn generate_keypair(
    state: State<'_, AppState>
) -> Result<GenerateKeypairResponse, String> {
    let (public_key, nsec) = state.key_manager
        .generate_keypair()
        .await
        .map_err(|e| e.to_string())?;

    Ok(GenerateKeypairResponse {
        public_key,
        nsec,
    })
}

#[tauri::command]
pub async fn login(
    state: State<'_, AppState>,
    request: LoginRequest
) -> Result<LoginResponse, String> {
    let (public_key, npub) = state.key_manager
        .login(&request.nsec)
        .await
        .map_err(|e| e.to_string())?;

    Ok(LoginResponse {
        public_key,
        npub,
    })
}

#[tauri::command]
pub async fn logout(
    state: State<'_, AppState>
) -> Result<(), String> {
    state.key_manager
        .logout()
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(())
}
