use crate::modules::auth::key_manager::KeyManager;
use crate::modules::bookmark::BookmarkManager;
use crate::modules::crypto::encryption::EncryptionManager;
use crate::modules::database::connection::{Database, DbPool};
use crate::modules::event::manager::EventManager;
use crate::modules::offline::OfflineManager;
use crate::modules::p2p::{EventSync, GossipManager, P2PEvent};
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::mpsc;
use tokio::sync::RwLock;

/// P2P関連の状態
pub struct P2PState {
    /// GossipManager instance
    pub manager: Option<Arc<GossipManager>>,

    /// EventSync instance for Nostr-P2P integration
    pub event_sync: Option<Arc<EventSync>>,

    /// Message event channel
    pub event_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<P2PEvent>>>>,
}

/// アプリケーション全体の状態を管理する構造体
#[derive(Clone)]
pub struct AppState {
    pub key_manager: Arc<KeyManager>,
    #[allow(dead_code)]
    pub db_pool: Arc<DbPool>,
    #[allow(dead_code)]
    pub encryption_manager: Arc<EncryptionManager>,
    pub event_manager: Arc<EventManager>,
    pub p2p_state: Arc<RwLock<P2PState>>,
    pub bookmark_manager: Arc<BookmarkManager>,
    pub offline_manager: Arc<OfflineManager>,
}

impl AppState {
    pub async fn new(app_handle: &tauri::AppHandle) -> anyhow::Result<Self> {
        // Get app data directory
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| anyhow::anyhow!("Failed to get app data dir: {}", e))?;
        
        // Debug logging
        tracing::info!("App data directory: {:?}", app_data_dir);
        
        // Create data directory if it doesn't exist
        if !app_data_dir.exists() {
            tracing::info!("Creating app data directory...");
            std::fs::create_dir_all(&app_data_dir)
                .map_err(|e| anyhow::anyhow!("Failed to create app data dir: {}", e))?;
            tracing::info!("App data directory created successfully");
        } else {
            tracing::info!("App data directory already exists");
        }

        let key_manager = Arc::new(KeyManager::new());
        
        // Use absolute path for database
        let db_path = app_data_dir.join("kukuri.db");
        
        // Debug logging
        tracing::info!("Database path: {:?}", db_path);
        
        // Ensure the database file path is canonical
        let db_path_str = db_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid database path encoding"))?;
        
        // Format database URL for sqlx
        // On Windows, sqlx may need a specific format
        let db_url = if cfg!(windows) {
            // Try Windows-specific format
            tracing::info!("Using Windows database URL format");
            format!("sqlite:{}?mode=rwc", db_path_str.replace('\\', "/"))
        } else {
            format!("sqlite://{}?mode=rwc", db_path_str)
        };
        
        tracing::info!("Database URL: {}", db_url);
        
        let db_pool = Arc::new(Database::initialize(&db_url).await?);
        let encryption_manager = Arc::new(EncryptionManager::new());
        let event_manager = Arc::new(EventManager::new());
        let bookmark_manager = Arc::new(BookmarkManager::new((*db_pool).clone()));
        let offline_manager = Arc::new(OfflineManager::new((*db_pool).clone()));

        // P2P状態の初期化
        let p2p_state = Arc::new(RwLock::new(P2PState {
            manager: None,
            event_sync: None,
            event_rx: Arc::new(RwLock::new(None)),
        }));

        Ok(Self {
            key_manager,
            db_pool,
            encryption_manager,
            event_manager,
            p2p_state,
            bookmark_manager,
            offline_manager,
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

        let manager_arc = Arc::new(manager);

        // EventSyncを作成
        let event_sync = EventSync::new(Arc::clone(&self.event_manager), Arc::clone(&manager_arc));
        let event_sync_arc = Arc::new(event_sync);

        // EventManagerにEventSyncを設定
        self.event_manager
            .set_event_sync(Arc::clone(&event_sync_arc))
            .await;

        // P2P状態を更新
        let mut p2p_state = self.p2p_state.write().await;
        p2p_state.manager = Some(manager_arc);
        p2p_state.event_sync = Some(Arc::clone(&event_sync_arc));
        *p2p_state.event_rx.write().await = Some(event_rx);

        // P2Pイベント処理ループを開始
        self.start_p2p_event_loop(event_sync_arc).await?;

        Ok(())
    }

    /// P2Pイベント処理ループを開始
    async fn start_p2p_event_loop(&self, event_sync: Arc<EventSync>) -> anyhow::Result<()> {
        let p2p_state = self.p2p_state.clone();

        tokio::spawn(async move {
            loop {
                let event_rx = {
                    let state = p2p_state.read().await;
                    let mut event_rx_guard = state.event_rx.write().await;
                    event_rx_guard.take()
                };

                if let Some(mut rx) = event_rx {
                    while let Some(event) = rx.recv().await {
                        match event {
                            P2PEvent::MessageReceived {
                                topic_id, message, ..
                            } => {
                                tracing::debug!("Received P2P message on topic {}", topic_id);

                                // EventSyncを使用してメッセージを処理
                                if let Err(e) = event_sync.handle_gossip_message(message).await {
                                    tracing::error!("Failed to handle gossip message: {}", e);
                                }
                            }
                            P2PEvent::PeerJoined { topic_id, peer_id } => {
                                tracing::info!("Peer joined topic {}: {:?}", topic_id, peer_id);
                            }
                            P2PEvent::PeerLeft { topic_id, peer_id } => {
                                tracing::info!("Peer left topic {}: {:?}", topic_id, peer_id);
                            }
                        }
                    }

                    // チャネルを元に戻す
                    *p2p_state.read().await.event_rx.write().await = Some(rx);
                }

                // 少し待機してから再試行
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });

        Ok(())
    }
}
