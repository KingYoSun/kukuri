use super::Validate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub success: bool,
    pub npub: String,
    pub pubkey: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginWithNsecRequest {
    pub nsec: String,
}

impl Validate for LoginWithNsecRequest {
    fn validate(&self) -> Result<(), String> {
        if self.nsec.trim().is_empty() {
            return Err("秘密鍵が必要です".to_string());
        }
        if !self.nsec.starts_with("nsec1") {
            return Err("無効な秘密鍵形式です".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAccountResponse {
    pub npub: String,
    pub nsec: String,
    pub pubkey: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportPrivateKeyRequest {
    pub npub: String,
}

impl Validate for ExportPrivateKeyRequest {
    fn validate(&self) -> Result<(), String> {
        if self.npub.trim().is_empty() {
            return Err("npub is required".to_string());
        }
        if !self.npub.starts_with("npub1") {
            return Err("npub must start with npub1".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportPrivateKeyResponse {
    pub nsec: String,
}
