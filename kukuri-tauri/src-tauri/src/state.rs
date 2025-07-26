use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tokio::sync::mpsc;
use crate::modules::auth::key_manager::KeyManager;
use crate::modules::database::connection::{Database, DbPool};
use crate::modules::crypto::encryption::EncryptionManager;
use crate::modules::event::manager::EventManager;
use crate::modules::p2p::{GossipManager, P2PEvent};

/// P2P関連の状態
pub struct P2PState {
    /// GossipManager instance
    pub manager: Option<Arc<GossipManager>>,
    
    /// Message event channel
    pub event_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<P2PEvent>>>>,
}

/// アプリケーション全体の状態を管理する構造体
#[derive(Clone)]
pub struct AppState {
    pub key_manager: Arc<KeyManager>,
    pub db_pool: Arc<DbPool>,
    pub encryption_manager: Arc<EncryptionManager>,
    pub event_manager: Arc<EventManager>,
    pub p2p_state: Arc<RwLock<P2PState>>,
}

impl AppState {
    pub async fn new() -> anyhow::Result<Self> {
        // Create data directory if it doesn't exist
        std::fs::create_dir_all("./data")?;
        
        let key_manager = Arc::new(KeyManager::new());
        let db_pool = Arc::new(Database::initialize("sqlite://./data/kukuri.db?mode=rwc").await?);
        let encryption_manager = Arc::new(EncryptionManager::new());
        let event_manager = Arc::new(EventManager::new());
        
        // P2P状態の初期化
        let p2p_state = Arc::new(RwLock::new(P2PState {
            manager: None,
            event_rx: Arc::new(RwLock::new(None)),
        }));

        Ok(Self {
            key_manager,
            db_pool,
            encryption_manager,
            event_manager,
            p2p_state,
        })
    }
    
    /// P2P機能を初期化
    pub async fn initialize_p2p(&self) -> anyhow::Result<()> {
        // 秘密鍵の生成または取得
        let iroh_secret_key = iroh::SecretKey::generate(rand::thread_rng());
        let secp_secret_key = secp256k1::SecretKey::new(&mut rand::thread_rng());
        
        // イベントチャネルを作成
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        // GossipManagerを作成
        let manager = GossipManager::new(iroh_secret_key, secp_secret_key, event_tx)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create GossipManager: {}", e))?;
        
        // P2P状態を更新
        let mut p2p_state = self.p2p_state.write().await;
        p2p_state.manager = Some(Arc::new(manager));
        *p2p_state.event_rx.write().await = Some(event_rx);
        
        Ok(())
    }
}