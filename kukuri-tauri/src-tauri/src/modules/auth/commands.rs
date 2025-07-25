use anyhow::Result;
use serde::{Deserialize, Serialize};

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
pub async fn generate_keypair() -> Result<GenerateKeypairResponse, String> {
    todo!("implement generate_keypair")
}

#[tauri::command]
pub async fn login(_request: LoginRequest) -> Result<LoginResponse, String> {
    todo!("implement login")
}

#[tauri::command]
pub async fn logout() -> Result<(), String> {
    todo!("implement logout")
}
