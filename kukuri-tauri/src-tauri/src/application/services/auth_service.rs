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
    user_service: Arc<super::UserService>,
    topic_service: Arc<super::TopicService>,
}

impl AuthService {
    pub fn new(
        key_manager: Arc<dyn KeyManager>,
        secure_storage: Arc<dyn SecureStorage>,
        user_service: Arc<super::UserService>,
        topic_service: Arc<super::TopicService>,
    ) -> Self {
        Self {
            key_manager,
            secure_storage,
            user_service,
            topic_service,
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
        let public_key = keypair.public_key.clone();
        let user = self
            .user_service
            .create_user(keypair.npub.clone(), public_key.clone())
            .await?;

        // Join public topic by default
        self.topic_service.ensure_public_topic().await?;
        self.topic_service.join_topic("public", &public_key).await?;

        Ok(user)
    }

    pub async fn login_with_nsec(&self, nsec: &str) -> Result<User, AppError> {
        // Import private key
        let keypair = self.key_manager.import_private_key(nsec).await?;

        // Store securely
        self.key_manager.store_keypair(&keypair).await?;
        self.secure_storage
            .store("current_npub", &keypair.npub)
            .await?;

        let public_key = keypair.public_key.clone();
        // Get or create user
        let user = match self.user_service.get_user(&keypair.npub).await? {
            Some(user) => user,
            None => {
                self.user_service
                    .create_user(keypair.npub.clone(), public_key.clone())
                    .await?
            }
        };

        // Join public topic by default
        self.topic_service.ensure_public_topic().await?;
        self.topic_service.join_topic("public", &public_key).await?;

        Ok(user)
    }

    pub async fn login_with_npub(&self, npub: &str) -> Result<User, AppError> {
        // Check if we have the private key stored
        let _private_key = self.key_manager.export_private_key(npub).await?;

        // Get user
        let user = self
            .user_service
            .get_user(npub)
            .await?
            .ok_or("User not found")?;

        self.secure_storage.store("current_npub", npub).await?;

        Ok(user)
    }

    pub async fn logout(&self) -> Result<(), AppError> {
        self.secure_storage.delete("current_npub").await?;
        Ok(())
    }

    pub async fn get_current_user(&self) -> Result<Option<User>, AppError> {
        if let Some(npub) = self.secure_storage.retrieve("current_npub").await? {
            self.user_service.get_user(&npub).await
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
