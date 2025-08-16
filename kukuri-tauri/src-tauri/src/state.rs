use crate::modules::auth::key_manager::KeyManager as OldKeyManager;
use crate::modules::bookmark::BookmarkManager;
use crate::modules::crypto::encryption::EncryptionManager;
use crate::modules::database::connection::{Database, DbPool};
use crate::modules::event::manager::EventManager;
use crate::modules::offline::OfflineManager;
use crate::modules::p2p::{EventSync, GossipManager, P2PEvent};

// アプリケーションサービスのインポート
use crate::application::services::{
    AuthService, EventService, PostService, SyncService, TopicService, UserService,
    P2PService, OfflineService,
};
// プレゼンテーション層のハンドラーのインポート
use crate::presentation::handlers::{
    user_handler::UserHandler,
    secure_storage_handler::SecureStorageHandler,
    event_handler::EventHandler, p2p_handler::P2PHandler,
    offline_handler::OfflineHandler,
};
use crate::infrastructure::{
    database::{sqlite_repository::SqliteRepository, connection_pool::ConnectionPool},
    p2p::{
        iroh_gossip_service::IrohGossipService, 
        iroh_network_service::IrohNetworkService,
        event_distributor::{DefaultEventDistributor, EventDistributor},
        GossipService, NetworkService,
    },
    crypto::{
        key_manager::DefaultKeyManager, 
        SignatureService, 
        DefaultSignatureService,
        KeyManager,
    },
    storage::{secure_storage::DefaultSecureStorage, SecureStorage},
};

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
    // 既存のマネージャー（後で移行予定）
    pub key_manager: Arc<OldKeyManager>,
    #[allow(dead_code)]
    pub db_pool: Arc<DbPool>,
    #[allow(dead_code)]
    pub encryption_manager: Arc<EncryptionManager>,
    pub event_manager: Arc<EventManager>,
    pub p2p_state: Arc<RwLock<P2PState>>,
    pub bookmark_manager: Arc<BookmarkManager>,
    pub offline_manager: Arc<OfflineManager>,
    
    // 新アーキテクチャのサービス層
    pub auth_service: Arc<AuthService>,
    pub post_service: Arc<PostService>,
    pub topic_service: Arc<TopicService>,
    pub user_service: Arc<UserService>,
    pub event_service: Arc<EventService>,
    pub sync_service: Arc<SyncService>,
    pub p2p_service: Arc<P2PService>,
    pub offline_service: Arc<OfflineService>,
    
    // プレゼンテーション層のハンドラー（最適化用）
    pub user_handler: Arc<UserHandler>,
    pub secure_storage_handler: Arc<SecureStorageHandler>,
    pub event_handler: Arc<EventHandler>,
    pub p2p_handler: Arc<P2PHandler>,
    pub offline_handler: Arc<OfflineHandler>,
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

        let key_manager = Arc::new(OldKeyManager::new());
        
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
            format!("sqlite://{db_path_str}?mode=rwc")
        };
        
        tracing::info!("Database URL: {db_url}");
        
        let db_pool = Arc::new(Database::initialize(&db_url).await?);
        let encryption_manager = Arc::new(EncryptionManager::new());
        let event_manager = Arc::new(EventManager::new_with_db(db_pool.clone()));
        let bookmark_manager = Arc::new(BookmarkManager::new((*db_pool).clone()));
        let offline_manager = Arc::new(OfflineManager::new((*db_pool).clone()));

        // 新アーキテクチャのリポジトリとサービスを初期化
        let pool = ConnectionPool::new(&db_url).await?;
        let repository = Arc::new(SqliteRepository::new(pool));
        
        // インフラストラクチャサービスの初期化
        let key_manager_service: Arc<dyn KeyManager> = Arc::new(DefaultKeyManager::new());
        let secure_storage: Arc<dyn SecureStorage> = Arc::new(DefaultSecureStorage::new());
        let signature_service: Arc<dyn SignatureService> = Arc::new(DefaultSignatureService::new());
        let event_distributor: Arc<dyn EventDistributor> = Arc::new(DefaultEventDistributor::new());
        
        // P2Pサービスの初期化（後で実際に初期化）
        let iroh_secret_key = iroh::SecretKey::generate(rand::thread_rng());
        let network_service: Arc<dyn NetworkService> = Arc::new(
            IrohNetworkService::new(iroh_secret_key).await
                .map_err(|e| anyhow::anyhow!("Failed to create NetworkService: {}", e))?
        );
        let gossip_service: Arc<dyn GossipService> = Arc::new(
            IrohGossipService::new(network_service.as_any().downcast_ref::<IrohNetworkService>()
                .ok_or_else(|| anyhow::anyhow!("Failed to downcast NetworkService"))?
                .endpoint().clone())
                .map_err(|e| anyhow::anyhow!("Failed to create GossipService: {}", e))?
        );
        
        // UserServiceを先に初期化（他のサービスの依存）
        let user_service = Arc::new(UserService::new(
            Arc::clone(&repository) as Arc<dyn crate::infrastructure::database::UserRepository>,
        ));
        
        // TopicServiceを初期化（AuthServiceの依存）
        let topic_service = Arc::new(TopicService::new(
            Arc::clone(&repository) as Arc<dyn crate::infrastructure::database::TopicRepository>,
            Arc::clone(&gossip_service),
        ));
        
        // AuthServiceの初期化（UserServiceとTopicServiceが必要）
        let auth_service = Arc::new(AuthService::new(
            Arc::clone(&key_manager_service),
            Arc::clone(&secure_storage),
            Arc::clone(&user_service),
            Arc::clone(&topic_service),
        ));
        
        // PostServiceの初期化
        let post_service = Arc::new(PostService::new(
            Arc::clone(&repository) as Arc<dyn crate::infrastructure::database::PostRepository>,
            Arc::clone(&event_distributor),
        ));
        
        // EventServiceの初期化
        let mut event_service_inner = EventService::new(
            Arc::clone(&repository) as Arc<dyn crate::infrastructure::database::EventRepository>,
            Arc::clone(&signature_service),
            Arc::clone(&event_distributor),
        );
        // EventManagerを設定
        event_service_inner.set_event_manager(Arc::clone(&event_manager));
        let event_service = Arc::new(event_service_inner);
        
        // SyncServiceの初期化（PostServiceとEventServiceが必要）
        let sync_service = Arc::new(SyncService::new(
            Arc::clone(&network_service),
            Arc::clone(&post_service),
            Arc::clone(&event_service),
        ));
        
        // P2PServiceの初期化
        let p2p_service = Arc::new(P2PService::new(
            Arc::clone(&network_service),
            Arc::clone(&gossip_service),
        ));
        
        // OfflineServiceの初期化
        let offline_service = Arc::new(OfflineService::new(
            Arc::clone(&repository) as Arc<dyn crate::infrastructure::database::Repository>,
        ));
        
        // プレゼンテーション層のハンドラーを初期化
        let user_handler = Arc::new(UserHandler::new(Arc::clone(&user_service)));
        let secure_storage_handler = Arc::new(SecureStorageHandler::new(Arc::clone(&auth_service)));
        let event_handler = Arc::new(EventHandler::new(Arc::clone(&event_service) as Arc<dyn crate::application::services::event_service::EventServiceTrait>));
        let p2p_handler = Arc::new(P2PHandler::new(Arc::clone(&p2p_service) as Arc<dyn crate::application::services::p2p_service::P2PServiceTrait>));
        let offline_handler = Arc::new(OfflineHandler::new(Arc::clone(&offline_service) as Arc<dyn crate::application::services::offline_service::OfflineServiceTrait>));

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
            auth_service,
            post_service,
            topic_service,
            user_service,
            event_service,
            sync_service,
            p2p_service,
            offline_service,
            user_handler,
            secure_storage_handler,
            event_handler,
            p2p_handler,
            offline_handler,
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
