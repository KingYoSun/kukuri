use crate::application::ports::auth_lifecycle::{
    AuthAccountContext, AuthLifecycleEvent, AuthLifecyclePort,
};
use crate::application::ports::key_manager::KeyManager;
use crate::domain::entities::User;
use crate::infrastructure::storage::SecureStorage;
use crate::shared::error::AppError;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AuthStatus {
    pub is_authenticated: bool,
    pub current_user: Option<User>,
    pub npub: Option<String>,
}

pub struct AuthService {
    key_manager: Arc<dyn KeyManager>,
    secure_storage: Arc<dyn SecureStorage>,
    lifecycle_port: Arc<dyn AuthLifecyclePort>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::auth_lifecycle::AuthLifecycleStage;
    use async_trait::async_trait;
    use mockall::{mock, predicate::*};

    mock! {
        pub KeyManager {}

        #[async_trait]
        impl KeyManager for KeyManager {
            async fn generate_keypair(&self) -> Result<KeyPair, AppError>;
            async fn import_private_key(&self, nsec: &str) -> Result<KeyPair, AppError>;
            async fn export_private_key(&self, npub: &str) -> Result<String, AppError>;
            async fn get_public_key(&self, npub: &str) -> Result<String, AppError>;
            async fn store_keypair(&self, keypair: &KeyPair) -> Result<(), AppError>;
            async fn delete_keypair(&self, npub: &str) -> Result<(), AppError>;
            async fn list_npubs(&self) -> Result<Vec<String>, AppError>;
            async fn current_keypair(&self) -> Result<KeyPair, AppError>;
        }
    }

    mock! {
        pub SecureStorage {}

        #[async_trait]
        impl SecureStorage for SecureStorage {
            async fn store(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
            async fn retrieve(&self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>>;
            async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
            async fn exists(&self, key: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;
            async fn list_keys(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>>;
            async fn clear(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
        }
    }

    mock! {
        pub Lifecycle {}

        #[async_trait]
        impl AuthLifecyclePort for Lifecycle {
            async fn handle(&self, event: AuthLifecycleEvent) -> Result<User, AppError>;
            async fn get_user(&self, npub: &str) -> Result<Option<User>, AppError>;
        }
    }

    use crate::application::ports::key_manager::KeyPair;

    fn sample_keypair() -> KeyPair {
        KeyPair {
            public_key: "pub".into(),
            private_key: "priv".into(),
            npub: "npub1".into(),
            nsec: "nsec1".into(),
        }
    }

    fn sample_user() -> User {
        User::new("npub1".into(), "pub".into())
    }

    #[tokio::test]
    async fn create_account_dispatches_lifecycle_event() {
        let mut key_manager = MockKeyManager::new();
        key_manager
            .expect_generate_keypair()
            .times(1)
            .returning(|| Ok(sample_keypair()));
        key_manager
            .expect_store_keypair()
            .times(1)
            .returning(|_| Ok(()));
        let mut storage = MockSecureStorage::new();
        storage
            .expect_store()
            .with(eq("current_npub"), eq("npub1"))
            .times(1)
            .returning(|_, _| Ok(()));

        let mut lifecycle = MockLifecycle::new();
        lifecycle
            .expect_handle()
            .times(1)
            .withf(|event| event.stage == AuthLifecycleStage::AccountCreated)
            .returning(|_| Ok(sample_user()));

        let service = AuthService::new(
            Arc::new(key_manager),
            Arc::new(storage),
            Arc::new(lifecycle),
        );

        let user = service.create_account().await.expect("create account");
        assert_eq!(user.npub, "npub1");
    }

    #[tokio::test]
    async fn login_with_npub_uses_lifecycle_login() {
        let mut key_manager = MockKeyManager::new();
        key_manager
            .expect_export_private_key()
            .with(eq("npub1"))
            .times(1)
            .returning(|_| Ok("nsec".into()));
        key_manager
            .expect_get_public_key()
            .with(eq("npub1"))
            .times(1)
            .returning(|_| Ok("pub".into()));
        let mut storage = MockSecureStorage::new();
        storage
            .expect_store()
            .with(eq("current_npub"), eq("npub1"))
            .times(1)
            .returning(|_, _| Ok(()));
        let mut lifecycle = MockLifecycle::new();
        lifecycle
            .expect_handle()
            .times(1)
            .withf(|event| event.stage == AuthLifecycleStage::Login)
            .returning(|_| Ok(sample_user()));

        let service = AuthService::new(
            Arc::new(key_manager),
            Arc::new(storage),
            Arc::new(lifecycle),
        );

        let user = service
            .login_with_npub("npub1")
            .await
            .expect("login success");
        assert_eq!(user.pubkey, "pub");
    }
}

impl AuthService {
    pub fn new(
        key_manager: Arc<dyn KeyManager>,
        secure_storage: Arc<dyn SecureStorage>,
        lifecycle_port: Arc<dyn AuthLifecyclePort>,
    ) -> Self {
        Self {
            key_manager,
            secure_storage,
            lifecycle_port,
        }
    }

    pub async fn create_account(&self) -> Result<User, AppError> {
        // Generate new keypair
        let keypair = self.key_manager.generate_keypair().await?;

        // Store securely
        self.key_manager.store_keypair(&keypair).await?;
        self.secure_storage
            .store("current_npub", &keypair.npub)
            .await?;

        // Create user
        let context = AuthAccountContext::from(&keypair);
        self.lifecycle_port
            .handle(AuthLifecycleEvent::account_created(context))
            .await
    }

    pub async fn login_with_nsec(&self, nsec: &str) -> Result<User, AppError> {
        // Import private key
        let keypair = self.key_manager.import_private_key(nsec).await?;

        // Store securely
        self.key_manager.store_keypair(&keypair).await?;
        self.secure_storage
            .store("current_npub", &keypair.npub)
            .await?;

        let context = AuthAccountContext::from(&keypair);
        self.lifecycle_port
            .handle(AuthLifecycleEvent::login(context))
            .await
    }

    pub async fn login_with_npub(&self, npub: &str) -> Result<User, AppError> {
        // Check if we have the private key stored
        let _private_key = self.key_manager.export_private_key(npub).await?;

        let public_key = self.key_manager.get_public_key(npub).await?;
        let context = AuthAccountContext::new(npub.to_string(), public_key);
        let user = self
            .lifecycle_port
            .handle(AuthLifecycleEvent::login(context))
            .await?;

        self.secure_storage.store("current_npub", npub).await?;

        Ok(user)
    }

    pub async fn logout(&self) -> Result<(), AppError> {
        self.secure_storage.delete("current_npub").await?;
        Ok(())
    }

    pub async fn get_current_user(&self) -> Result<Option<User>, AppError> {
        if let Some(npub) = self.secure_storage.retrieve("current_npub").await? {
            self.lifecycle_port.get_user(&npub).await
        } else {
            Ok(None)
        }
    }

    pub async fn is_authenticated(&self) -> bool {
        self.secure_storage
            .retrieve("current_npub")
            .await
            .unwrap_or(None)
            .is_some()
    }

    pub async fn get_auth_status(&self) -> Result<AuthStatus, AppError> {
        let current_user = self.get_current_user().await?;
        let npub = self.secure_storage.retrieve("current_npub").await?;

        Ok(AuthStatus {
            is_authenticated: current_user.is_some(),
            current_user,
            npub,
        })
    }

    pub async fn export_private_key(&self, npub: &str) -> Result<String, AppError> {
        self.key_manager.export_private_key(npub).await
    }

    pub async fn list_accounts(&self) -> Result<Vec<String>, AppError> {
        self.key_manager.list_npubs().await
    }

    pub async fn switch_account(&self, npub: &str) -> Result<User, AppError> {
        self.login_with_npub(npub).await
    }
}
