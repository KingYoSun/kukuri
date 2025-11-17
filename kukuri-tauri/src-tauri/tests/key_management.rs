use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use kukuri_lib::test_support::{
    application::{
        ports::{
            auth_lifecycle::{AuthLifecycleEvent, AuthLifecyclePort},
            key_manager::KeyManager,
        },
        services::AuthService,
    },
    domain::entities::User,
    infrastructure::{
        crypto::DefaultKeyManager,
        storage::secure_storage::SecureStorage,
    },
    shared::error::AppError,
};
use tokio::sync::Mutex;

#[derive(Default)]
struct InMemorySecureStorage {
    entries: Mutex<HashMap<String, String>>,
}

#[async_trait]
impl SecureStorage for InMemorySecureStorage {
    async fn store(
        &self,
        key: &str,
        value: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.entries
            .lock()
            .await
            .insert(key.to_string(), value.to_string());
        Ok(())
    }

    async fn retrieve(
        &self,
        key: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.entries.lock().await.get(key).cloned())
    }

    async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.entries.lock().await.remove(key);
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.entries.lock().await.contains_key(key))
    }

    async fn list_keys(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.entries.lock().await.keys().cloned().collect())
    }

    async fn clear(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.entries.lock().await.clear();
        Ok(())
    }
}

#[derive(Default)]
struct TestAuthLifecycle {
    users: Mutex<HashMap<String, User>>,
}

#[async_trait]
impl AuthLifecyclePort for TestAuthLifecycle {
    async fn handle(&self, event: AuthLifecycleEvent) -> Result<User, AppError> {
        let account = event.account;
        let user = User::new(account.npub.clone(), account.public_key.clone());
        self.users
            .lock()
            .await
            .insert(account.npub.clone(), user.clone());
        Ok(user)
    }

    async fn get_user(&self, npub: &str) -> Result<Option<User>, AppError> {
        Ok(self.users.lock().await.get(npub).cloned())
    }
}

#[tokio::test]
async fn export_private_key_roundtrip() {
    let key_manager = Arc::new(DefaultKeyManager::new()) as Arc<dyn KeyManager>;
    let secure_storage = Arc::new(InMemorySecureStorage::default());
    let lifecycle = Arc::new(TestAuthLifecycle::default());

    let service = AuthService::new(
        key_manager,
        secure_storage.clone() as Arc<dyn SecureStorage>,
        lifecycle as Arc<dyn AuthLifecyclePort>,
    );

    let created_user = service.create_account().await.expect("create account");
    let exported = service
        .export_private_key(&created_user.npub)
        .await
        .expect("export key");
    assert!(exported.starts_with("nsec1"));

    service.logout().await.expect("logout");
    let imported_user = service
        .login_with_nsec(&exported)
        .await
        .expect("login with exported key");

    assert_eq!(imported_user.npub, created_user.npub);

    let stored_npub = secure_storage
        .retrieve("current_npub")
        .await
        .expect("current_npub entry");
    assert_eq!(stored_npub, Some(created_user.npub));
}
