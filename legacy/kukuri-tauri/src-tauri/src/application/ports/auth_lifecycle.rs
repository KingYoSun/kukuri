use crate::application::ports::key_manager::KeyPair;
use crate::domain::entities::User;
use crate::shared::error::AppError;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct AuthAccountContext {
    pub npub: String,
    pub public_key: String,
}

impl AuthAccountContext {
    pub fn new(npub: impl Into<String>, public_key: impl Into<String>) -> Self {
        Self {
            npub: npub.into(),
            public_key: public_key.into(),
        }
    }
}

impl From<&KeyPair> for AuthAccountContext {
    fn from(keypair: &KeyPair) -> Self {
        Self {
            npub: keypair.npub.clone(),
            public_key: keypair.public_key.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthLifecycleStage {
    AccountCreated,
    Login,
}

#[derive(Debug, Clone)]
pub struct AuthLifecycleEvent {
    pub stage: AuthLifecycleStage,
    pub account: AuthAccountContext,
}

impl AuthLifecycleEvent {
    pub fn new(stage: AuthLifecycleStage, account: AuthAccountContext) -> Self {
        Self { stage, account }
    }

    pub fn account_created(account: AuthAccountContext) -> Self {
        Self::new(AuthLifecycleStage::AccountCreated, account)
    }

    pub fn login(account: AuthAccountContext) -> Self {
        Self::new(AuthLifecycleStage::Login, account)
    }
}

#[async_trait]
pub trait AuthLifecyclePort: Send + Sync {
    async fn handle(&self, event: AuthLifecycleEvent) -> Result<User, AppError>;
    async fn get_user(&self, npub: &str) -> Result<Option<User>, AppError>;
}
