use std::sync::Arc;
use crate::modules::auth::key_manager::KeyManager;
use crate::modules::database::connection::{Database, DbPool};
use crate::modules::crypto::encryption::EncryptionManager;

/// アプリケーション全体の状態を管理する構造体
#[derive(Clone)]
pub struct AppState {
    pub key_manager: Arc<KeyManager>,
    pub db_pool: Arc<DbPool>,
    pub encryption_manager: Arc<EncryptionManager>,
}

impl AppState {
    pub async fn new() -> anyhow::Result<Self> {
        // Create data directory if it doesn't exist
        std::fs::create_dir_all("./data")?;
        
        let key_manager = Arc::new(KeyManager::new());
        let db_pool = Arc::new(Database::initialize("sqlite://./data/kukuri.db?mode=rwc").await?);
        let encryption_manager = Arc::new(EncryptionManager::new());

        Ok(Self {
            key_manager,
            db_pool,
            encryption_manager,
        })
    }
}