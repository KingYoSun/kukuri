use crate::modules::auth::key_manager::KeyManager as OldKeyManager;
use crate::modules::bookmark::BookmarkManager;
use crate::modules::crypto::encryption::EncryptionManager;
use crate::modules::database::connection::{Database, DbPool};
use crate::modules::event::manager::EventManager;
use crate::modules::offline::OfflineManager;
use crate::modules::p2p::P2PEvent;

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
        database::{sqlite_repository::SqliteRepository, connection_pool::ConnectionPool, Repository},
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
use tauri::{Emitter, Manager};
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use std::collections::{HashSet as StdHashSet, VecDeque as StdVecDeque};

const P2P_DEDUP_MAX: usize = 8192;

/// P2P関連の状態
pub struct P2PState {
    /// Message event channel
    pub event_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<P2PEvent>>>>,
    /// GossipService 本体（UI購読導線で使用）
    pub gossip_service: Arc<dyn GossipService>,
    /// UI購読済みトピック集合（重複購読防止）
    pub ui_subscribed_topics: Arc<RwLock<std::collections::HashSet<String>>>,
    /// 受信イベントIDの重複排除用セット
    pub seen_event_ids: Arc<RwLock<StdHashSet<String>>>,
    /// 受信イベントIDの順序（容量制御用）
    pub seen_event_order: Arc<RwLock<StdVecDeque<String>>>,
}

/// アプリケーション全体の状態を管理する構造体
#[derive(Clone)]
pub struct AppState {
    pub app_handle: tauri::AppHandle,
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
        
        // リポジトリのマイグレーションを実行
        repository.initialize().await?;
        
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
        // GossipServiceの初期化（イベントチャネルを接続）
        let endpoint_arc = network_service
            .as_any()
            .downcast_ref::<IrohNetworkService>()
            .ok_or_else(|| anyhow::anyhow!("Failed to downcast NetworkService"))?
            .endpoint()
            .clone();

        let (p2p_event_tx, p2p_event_rx) = mpsc::unbounded_channel();

        let mut gossip_inner = IrohGossipService::new(endpoint_arc)
            .map_err(|e| anyhow::anyhow!("Failed to create GossipService: {}", e))?;
        gossip_inner.set_event_sender(p2p_event_tx);
        let gossip_service: Arc<dyn GossipService> = Arc::new(gossip_inner);
        // EventManagerへGossipServiceを接続（P2P配信経路の直結）
        event_manager.set_gossip_service(Arc::clone(&gossip_service)).await;
        // EventManagerへEventRepositoryを接続（参照トピック解決用）
        event_manager
            .set_event_repository(Arc::clone(&repository) as Arc<dyn crate::infrastructure::database::EventRepository>)
            .await;
        
        // UserServiceを先に初期化（他のサービスの依存）
        let user_service = Arc::new(UserService::new(
            Arc::clone(&repository) as Arc<dyn crate::infrastructure::database::UserRepository>,
        ));
        
        // TopicServiceを初期化（AuthServiceの依存）
        let topic_service = Arc::new(TopicService::new(
            Arc::clone(&repository) as Arc<dyn crate::infrastructure::database::TopicRepository>,
            Arc::clone(&gossip_service),
        ));
        // 既定トピック（public）を保証し、EventManagerの既定配信先に設定
        topic_service.ensure_public_topic().await
            .map_err(|e| anyhow::anyhow!("Failed to ensure public topic: {}", e))?;
        event_manager.set_default_p2p_topic_id("public").await;
        
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
            event_rx: Arc::new(RwLock::new(Some(p2p_event_rx))),
            gossip_service: Arc::clone(&gossip_service),
            ui_subscribed_topics: Arc::new(RwLock::new(Default::default())),
            seen_event_ids: Arc::new(RwLock::new(Default::default())),
            seen_event_order: Arc::new(RwLock::new(Default::default())),
        }));

        // 既定トピック`public`に対するUI購読を張る（冪等）
        // TopicService.ensure_public_topic でjoinは保証済
        let this_handle = app_handle.clone();
        let this = Self {
            app_handle: this_handle,
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
        };

        // 起動時に既定＋ユーザー固有トピックの購読を確立
        {
            let this_clone = this.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = this_clone.ensure_default_and_user_subscriptions().await {
                    tracing::warn!("Failed to ensure default/user subscriptions: {}", e);
                }
            });
        }

        Ok(this)
    }

    /// P2P機能を初期化
    pub async fn initialize_p2p(&self) -> anyhow::Result<()> {
        // 旧GossipManager経路は無効化。IrohGossipService経由で運用。
        // 互換のため成功扱いで何もしない。
        Ok(())
    }

    // Event loop for P2P messages is now handled via UI emitter in lib.rs using event_rx

    /// UI向けに指定トピックの購読を確立（冪等）
    pub async fn ensure_ui_subscription(&self, topic_id: &str) -> anyhow::Result<()> {
        // 重複購読チェック
        {
            let p2p_state = self.p2p_state.read().await;
            let subs = p2p_state.ui_subscribed_topics.read().await;
            if subs.contains(topic_id) {
                return Ok(());
            }
        }

        // 購読開始（joinはTopicService側で行われるが、冪等joinは吸収される）
        let (gossip, event_manager, p2p_state_arc, app_handle, topic) = {
            let p2p_state = self.p2p_state.read().await;
            (
                Arc::clone(&p2p_state.gossip_service),
                Arc::clone(&self.event_manager),
                Arc::clone(&self.p2p_state),
                self.app_handle.clone(),
                topic_id.to_string(),
            )
        };

        // 先にフラグを立てる（競合回避）
        {
            let ui_arc = {
                let p2p = p2p_state_arc.read().await;
                Arc::clone(&p2p.ui_subscribed_topics)
            };
            let mut subs = ui_arc.write().await;
            subs.insert(topic.clone());
        }

        tauri::async_runtime::spawn(async move {
            match gossip.subscribe(&topic).await {
                Ok(mut rx) => {
                    tracing::info!("UI subscribed to topic {}", topic);
                    while let Some(evt) = rx.recv().await {
                        // 重複排除（イベントID）
                        let evt_id = evt.id.clone();
                        let (set_arc, order_arc) = {
                            let p2p = p2p_state_arc.read().await;
                            (
                                Arc::clone(&p2p.seen_event_ids),
                                Arc::clone(&p2p.seen_event_order),
                            )
                        };
                        {
                            let mut set = set_arc.write().await;
                            if set.contains(&evt_id) {
                                continue;
                            }
                            set.insert(evt_id.clone());
                        }
                        {
                            let mut order = order_arc.write().await;
                            order.push_back(evt_id.clone());
                            if order.len() > P2P_DEDUP_MAX {
                                if let Some(old_id) = order.pop_front() {
                                    let mut set = set_arc.write().await;
                                    set.remove(&old_id);
                                }
                            }
                        }
                        // 受信: domain::entities::Event
                        // UIへemit（p2p://message）
                        #[derive(serde::Serialize, Clone)]
                        struct UiMsg { id: String, author: String, content: String, timestamp: i64, signature: String }
                        #[derive(serde::Serialize, Clone)]
                        struct UiP2PMessageEvent { topic_id: String, message: UiMsg }

                        let payload = UiP2PMessageEvent {
                            topic_id: topic.clone(),
                            message: UiMsg {
                                id: evt.id.clone(),
                                author: evt.pubkey.clone(),
                                content: evt.content.clone(),
                                timestamp: evt.created_at.timestamp_millis(),
                                signature: evt.sig.clone(),
                            },
                        };
                        if let Err(e) = app_handle.emit("p2p://message", payload) {
                            tracing::error!("Failed to emit UI P2P message: {}", e);
                        }

                        // 既存Nostr系導線へも流す（必要に応じて）
                        // domain::Event -> NostrEventPayload 相当はEventManager内にあるが、
                        // ここではDB保存・加工は後段で検討するためスキップ
                        let _ = event_manager; // 未来の拡張用プレースホルダ
                    }
                    // チャネルクローズ時、購読フラグを解除
                    let ui_arc = {
                        let p2p = p2p_state_arc.read().await;
                        Arc::clone(&p2p.ui_subscribed_topics)
                    };
                    let mut subs = ui_arc.write().await;
                    subs.remove(&topic);
                    tracing::info!("UI subscription ended for topic {}", topic);
                }
                Err(e) => {
                    tracing::error!("Failed to subscribe to topic {}: {}", topic, e);
                    let ui_arc = {
                        let p2p = p2p_state_arc.read().await;
                        Arc::clone(&p2p.ui_subscribed_topics)
                    };
                    let mut subs = ui_arc.write().await;
                    subs.remove(&topic);
                }
            }
        });

        Ok(())
    }

    /// 既定トピックとユーザー固有トピックの購読を確立（冪等）
    pub async fn ensure_default_and_user_subscriptions(&self) -> anyhow::Result<()> {
        let mut topics = self.event_manager.list_default_p2p_topics().await;
        if let Some(pk) = self.event_manager.get_public_key().await {
            let user_topic = crate::modules::p2p::user_topic_id(&pk.to_string());
            topics.push(user_topic);
        }
        for t in topics {
            if let Err(e) = self.ensure_ui_subscription(&t).await {
                tracing::warn!("Failed to ensure subscription for {}: {}", t, e);
            }
        }
        Ok(())
    }

    /// UI向け購読を停止（存在しなければ何もしない）
    pub async fn stop_ui_subscription(&self, topic_id: &str) -> anyhow::Result<()> {
        // フラグのみ除去（購読タスクはチャネルクローズにより自然終了）
        let ui_subs_arc = {
            let p2p_state = self.p2p_state.read().await;
            Arc::clone(&p2p_state.ui_subscribed_topics)
        };
        let mut subs = ui_subs_arc.write().await;
        subs.remove(topic_id);
        Ok(())
    }
}
