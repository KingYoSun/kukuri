use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tokio::sync::mpsc;
use crate::modules::auth::key_manager::KeyManager;
use crate::modules::database::connection::{Database, DbPool};
use crate::modules::crypto::encryption::EncryptionManager;
use crate::modules::event::manager::EventManager;
use crate::modules::p2p::{GossipManager, GossipMessage};

/// P2P関連の状態
pub struct P2PState {
    /// GossipManager instance
    pub manager: Option<Arc<GossipManager>>,
    
    /// Active topic subscriptions
    pub topics: Arc<RwLock<HashMap<String, TopicState>>>,
    
    /// Message event channel
    pub event_tx: mpsc::UnboundedSender<P2PEvent>,
    pub event_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<P2PEvent>>>>,
}

pub struct TopicState {
    pub peers: Vec<String>,
    pub stats: TopicStats,
}

#[derive(Clone, Debug)]
pub enum P2PEvent {
    MessageReceived {
        topic_id: String,
        message: GossipMessage,
    },
    PeerJoined {
        topic_id: String,
        peer_id: String,
    },
    PeerLeft {
        topic_id: String,
        peer_id: String,
    },
}

#[derive(Clone)]
pub struct TopicStats {
    pub message_count: usize,
    pub last_activity: i64,
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
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let p2p_state = Arc::new(RwLock::new(P2PState {
            manager: None,
            topics: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            event_rx: Arc::new(RwLock::new(Some(event_rx))),
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
        use iroh::SecretKey;
        
        // 秘密鍵の生成または取得
        let secret_key = SecretKey::generate(rand::thread_rng());
        
        // GossipManagerを作成
        let manager = GossipManager::new(secret_key)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create GossipManager: {}", e))?;
        
        // P2P状態を更新
        let mut p2p_state = self.p2p_state.write().await;
        p2p_state.manager = Some(Arc::new(manager));
        
        Ok(())
    }
}