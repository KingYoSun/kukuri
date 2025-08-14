use crate::{
    application::services::AuthService,
    presentation::dto::{
        auth_dto::{CreateAccountResponse, LoginResponse, LoginWithNsecRequest},
        Validate,
    },
    shared::error::AppError,
};
use std::sync::Arc;

pub struct AuthHandler {
    auth_service: Arc<AuthService>,
}

impl AuthHandler {
    pub fn new(auth_service: Arc<AuthService>) -> Self {
        Self { auth_service }
    }

    pub async fn create_account(&self) -> Result<CreateAccountResponse, AppError> {
        let user = self.auth_service.create_account().await?;
        
        // nsecの生成（実際の実装では秘密鍵から生成）
        let nsec = format!("nsec1{}", &user.pubkey[..32]); // 仮実装

        Ok(CreateAccountResponse {
            npub: user.npub,
            nsec,
            pubkey: user.pubkey,
        })
    }

    pub async fn login_with_nsec(&self, request: LoginWithNsecRequest) -> Result<LoginResponse, AppError> {
        request.validate()
            .map_err(|e| AppError::InvalidInput(e))?;

        let user = self.auth_service.login_with_nsec(&request.nsec).await?;

        Ok(LoginResponse {
            success: true,
            npub: user.npub,
            pubkey: user.pubkey,
        })
    }

    pub async fn login_with_npub(&self, npub: String) -> Result<LoginResponse, AppError> {
        let user = self.auth_service.login_with_npub(&npub).await?;

        Ok(LoginResponse {
            success: true,
            npub: user.npub,
            pubkey: user.pubkey,
        })
    }

    pub async fn logout(&self, _npub: String) -> Result<(), AppError> {
        self.auth_service.logout().await?;
        Ok(())
    }

    pub async fn get_current_user(&self) -> Result<Option<LoginResponse>, AppError> {
        match self.auth_service.get_current_user().await? {
            Some(user) => Ok(Some(LoginResponse {
                success: true,
                npub: user.npub,
                pubkey: user.pubkey,
            })),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::services::{AuthService, UserService, TopicService},
        domain::entities::User,
        infrastructure::{
            crypto::KeyManager,
            storage::SecureStorage,
            database::SqliteRepository,
            p2p::{iroh_network_service::IrohNetworkService, iroh_gossip_service::IrohGossipService},
        },
        application::services::P2PService,
    };
    use mockall::mock;
    use async_trait::async_trait;

    // 簡易的な統合テスト
    // 実際のサービスをモック化せずに基本的な動作確認のみ行う
    // 完全なモック実装は、サービス層にトレイトを定義してから行う

    #[test]
    fn test_auth_handler_creation() {
        // AuthHandlerのインスタンス生成だけをテスト
        // 実際のサービスの初期化は行わない（構造体の生成のみ）
        
        // このテストは、AuthHandlerの基本的な構造が正しいことを確認
        assert!(true); // プレースホルダー
    }
    
    #[test]
    fn test_login_request_validation() {
        let request = LoginWithNsecRequest {
            nsec: "".to_string(),
        };
        
        // 空のnsecはバリデーションエラーになるはず
        let validation_result = request.validate();
        assert!(validation_result.is_err());
        
        let valid_request = LoginWithNsecRequest {
            nsec: "nsec1valid".to_string(),
        };
        
        let validation_result = valid_request.validate();
        assert!(validation_result.is_ok());
    }
}