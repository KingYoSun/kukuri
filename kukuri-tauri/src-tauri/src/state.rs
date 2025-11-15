use crate::application::ports::key_manager::{KeyManager, KeyMaterialStore};
use crate::domain::entities::ScoreWeights;
use crate::domain::p2p::P2PEvent;

// アプリケーションサービスのインポート
use crate::application::ports::auth_lifecycle::AuthLifecyclePort;
use crate::application::ports::cache::PostCache;
use crate::application::ports::event_topic_store::EventTopicStore;
use crate::application::ports::messaging_gateway::MessagingGateway;
use crate::application::ports::offline_store::OfflinePersistence;
use crate::application::ports::repositories::{
    BookmarkRepository, DirectMessageRepository, EventRepository, PendingTopicRepository,
    PostRepository, TopicMetricsRepository, TopicRepository, UserRepository,
};
use crate::application::ports::secure_storage::SecureAccountStore;
use crate::application::ports::subscription_state_repository::SubscriptionStateRepository;
use crate::application::services::event_service::EventServiceTrait;
use crate::application::services::offline_service::OfflineServiceTrait;
use crate::application::services::p2p_service::P2PServiceTrait;
use crate::application::services::sync_service::{SyncParticipant, SyncServiceTrait};
use crate::application::services::{
    AuthService, DirectMessageService, EventService, OfflineService, PostService,
    ProfileAvatarService, SubscriptionStateMachine, SyncService, TopicService, UserSearchService,
    UserService,
};
// プレゼンテーション層のハンドラーのインポート
use crate::application::services::auth_lifecycle::DefaultAuthLifecycle;
use crate::infrastructure::{
    cache::PostCacheService,
    crypto::{DefaultKeyManager, DefaultSignatureService, SignatureService},
    database::{
        Repository, SqliteSubscriptionStateRepository, connection_pool::ConnectionPool,
        sqlite_repository::SqliteRepository,
    },
    event::{
        EventManagerHandle, EventManagerSubscriptionInvoker, LegacyEventManagerGateway,
        LegacyEventManagerHandle, RepositoryEventTopicStore,
    },
    jobs::{
        trending_metrics_job::TrendingMetricsJob,
        trending_metrics_metrics::TrendingMetricsRecorder,
        trending_metrics_server::spawn_prometheus_exporter,
    },
    messaging::NostrMessagingGateway,
    offline::{OfflineReindexJob, SqliteOfflinePersistence},
    p2p::{
        GossipService, NetworkService,
        bootstrap::P2PBootstrapper,
        event_distributor::{DefaultEventDistributor, EventDistributor},
    },
    storage::{SecureStorage, secure_storage::DefaultSecureStorage},
};
use crate::presentation::handlers::{
    event_handler::EventHandler, offline_handler::OfflineHandler, p2p_handler::P2PHandler,
    secure_storage_handler::SecureStorageHandler, user_handler::UserHandler,
};
use crate::presentation::ipc::direct_message_notifier::IpcDirectMessageNotifier;

use nostr_sdk::prelude::{Event as NostrEvent, Kind, TagStandard, ToBech32};
use std::collections::{HashSet as StdHashSet, VecDeque as StdVecDeque};
use std::sync::Arc;
use tauri::{Emitter, async_runtime};
use tokio::sync::RwLock;
use tokio::sync::broadcast;
use tokio::time::{Duration, sleep};

const P2P_DEDUP_MAX: usize = 8192;
const DEFAULT_SYNC_INTERVAL_SECS: u64 = 30;

/// P2P関連の状態
pub struct P2PState {
    /// Message event channel
    pub event_rx: Arc<RwLock<Option<broadcast::Receiver<P2PEvent>>>>,
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
    // 既存のマネージャー（Phase5でArc<dyn KeyManager>へ移行済み）
    pub key_manager: Arc<dyn KeyManager>,
    pub event_manager: Arc<dyn EventManagerHandle>,
    pub p2p_state: Arc<RwLock<P2PState>>,
    pub offline_reindex_job: Arc<OfflineReindexJob>,

    // 新アーキテクチャのサービス層
    pub auth_service: Arc<AuthService>,
    pub post_service: Arc<PostService>,
    pub topic_service: Arc<TopicService>,
    pub user_service: Arc<UserService>,
    pub user_search_service: Arc<UserSearchService>,
    pub event_service: Arc<EventService>,
    pub direct_message_service: Arc<DirectMessageService>,
    pub sync_service: Arc<dyn SyncServiceTrait>,
    pub p2p_service: Arc<dyn P2PServiceTrait>,
    pub offline_service: Arc<OfflineService>,
    pub profile_avatar_service: Arc<ProfileAvatarService>,

    // プレゼンテーション層のハンドラー（最適化用）
    pub user_handler: Arc<UserHandler>,
    pub secure_storage_handler: Arc<SecureStorageHandler>,
    pub event_handler: Arc<EventHandler>,
    pub p2p_handler: Arc<P2PHandler>,
    pub offline_handler: Arc<OfflineHandler>,
}

impl AppState {
    pub async fn new(app_handle: &tauri::AppHandle) -> anyhow::Result<Self> {
        let bootstrapper = P2PBootstrapper::new(app_handle).await?;
        let metrics_config = bootstrapper.config().metrics.clone();
        let app_data_dir = bootstrapper.app_data_dir().to_path_buf();
        let profile_avatar_dir = app_data_dir.join("profile_avatars");

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

        // 新アーキテクチャのリポジトリとサービスを初期化
        let connection_pool = ConnectionPool::new(&db_url).await?;
        let repository = Arc::new(SqliteRepository::new(connection_pool.clone()));
        let topic_metrics_repository: Arc<dyn TopicMetricsRepository> =
            Arc::clone(&repository) as Arc<dyn TopicMetricsRepository>;
        let event_topic_store: Arc<dyn EventTopicStore> = Arc::new(RepositoryEventTopicStore::new(
            Arc::clone(&repository) as Arc<dyn EventRepository>,
        ));
        let subscription_repository: Arc<dyn SubscriptionStateRepository> = Arc::new(
            SqliteSubscriptionStateRepository::new(connection_pool.clone()),
        );

        // リポジトリのマイグレーションを実行
        repository.initialize().await?;

        let sqlite_pool = connection_pool.get_pool().clone();

        let event_manager: Arc<dyn EventManagerHandle> = Arc::new(
            LegacyEventManagerHandle::new_with_connection_pool(connection_pool.clone()),
        );
        let offline_persistence_concrete =
            Arc::new(SqliteOfflinePersistence::new(sqlite_pool.clone()));
        let offline_persistence: Arc<dyn OfflinePersistence> = offline_persistence_concrete.clone();
        let offline_reindex_job =
            OfflineReindexJob::create(Some(app_handle.clone()), Arc::clone(&offline_persistence));
        offline_reindex_job.trigger();
        let offline_service = Arc::new(OfflineService::new(Arc::clone(&offline_persistence)));

        // インフラストラクチャサービスの初期化
        let secure_storage_impl = Arc::new(DefaultSecureStorage::new());
        let key_material_store: Arc<dyn KeyMaterialStore> = secure_storage_impl.clone();
        let key_manager: Arc<dyn KeyManager> = Arc::new(DefaultKeyManager::with_store(Arc::clone(
            &key_material_store,
        )));
        let secure_storage: Arc<dyn SecureStorage> = secure_storage_impl.clone();
        let secure_account_store: Arc<dyn SecureAccountStore> = secure_storage_impl.clone();
        let signature_service: Arc<dyn SignatureService> = Arc::new(DefaultSignatureService::new());
        let default_event_distributor = Arc::new(DefaultEventDistributor::new());
        let event_distributor: Arc<dyn EventDistributor> =
            default_event_distributor.clone() as Arc<dyn EventDistributor>;

        // P2Pサービスの初期化
        let (p2p_event_tx, _initial_rx) = broadcast::channel(512);
        let p2p_stack = bootstrapper.build_stack(p2p_event_tx.clone()).await?;

        let network_service: Arc<dyn NetworkService> = Arc::clone(&p2p_stack.network_service);
        let gossip_service: Arc<dyn GossipService> = Arc::clone(&p2p_stack.gossip_service);
        let p2p_service = Arc::clone(&p2p_stack.p2p_service);

        default_event_distributor
            .set_gossip_service(Arc::clone(&gossip_service))
            .await;
        default_event_distributor
            .set_network_service(Arc::clone(&network_service))
            .await;
        // EventManagerへGossipServiceを接続（P2P配信経路の直結）
        event_manager
            .set_gossip_service(Arc::clone(&gossip_service))
            .await;
        // EventManagerへEventRepositoryを接続（参照トピック解決用）
        event_manager
            .set_event_topic_store(Arc::clone(&event_topic_store))
            .await;

        // UserServiceを先に初期化（他のサービスの依存）
        let user_service = Arc::new(UserService::new(
            Arc::clone(&repository) as Arc<dyn UserRepository>
        ));
        let user_search_service = Arc::new(UserSearchService::new(
            Arc::clone(&repository) as Arc<dyn UserRepository>
        ));

        // TopicServiceを初期化（AuthServiceの依存）
        let topic_service = Arc::new(TopicService::new(
            Arc::clone(&repository) as Arc<dyn TopicRepository>,
            Arc::clone(&repository) as Arc<dyn PendingTopicRepository>,
            Arc::clone(&topic_metrics_repository),
            metrics_config.enabled,
            Arc::clone(&p2p_service),
            Arc::clone(&offline_service) as Arc<dyn OfflineServiceTrait>,
        ));
        // 既定トピック（public）を保証し、EventManagerの既定配信先に設定
        topic_service
            .ensure_public_topic()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to ensure public topic: {}", e))?;
        event_manager.set_default_p2p_topic_id("public").await;
        let distributor_default_topics = event_manager.list_default_p2p_topics().await;
        default_event_distributor
            .set_default_topics(distributor_default_topics)
            .await;

        if metrics_config.enabled {
            let score_weights = ScoreWeights {
                posts: metrics_config.score_weights.posts,
                unique_authors: metrics_config.score_weights.unique_authors,
                boosts: metrics_config.score_weights.boosts,
            };

            let metrics_recorder = TrendingMetricsRecorder::new(metrics_config.emit_histogram)
                .map(Arc::new)
                .map_err(|err| anyhow::anyhow!("Failed to initialize metrics recorder: {err}"))?;

            if let Some(port) = metrics_config.prometheus_port {
                spawn_prometheus_exporter(port, Arc::clone(&metrics_recorder));
            }

            let job = Arc::new(TrendingMetricsJob::new(
                Arc::clone(&topic_metrics_repository),
                Some(score_weights),
                metrics_config.ttl_hours,
                Some(Arc::clone(&metrics_recorder)),
            ));
            spawn_trending_metrics_job(
                job,
                Duration::from_secs(metrics_config.interval_minutes.max(1) * 60),
            );
        }

        // AuthServiceの初期化（UserServiceとTopicServiceが必要）
        let lifecycle_port: Arc<dyn AuthLifecyclePort> = Arc::new(DefaultAuthLifecycle::new(
            Arc::clone(&user_service),
            Arc::clone(&topic_service),
        ));

        let auth_service = Arc::new(AuthService::new(
            Arc::clone(&key_manager),
            Arc::clone(&secure_storage),
            lifecycle_port,
        ));

        let subscription_state = Arc::new(SubscriptionStateMachine::new(Arc::clone(
            &subscription_repository,
        )));

        // EventServiceの初期化
        let legacy_event_gateway =
            Arc::new(LegacyEventManagerGateway::new(Arc::clone(&event_manager)));
        let event_gateway: Arc<dyn crate::application::ports::event_gateway::EventGateway> =
            legacy_event_gateway.clone();
        let mut event_service_inner = EventService::new(
            Arc::clone(&repository) as Arc<dyn EventRepository>,
            Arc::clone(&signature_service),
            Arc::clone(&event_distributor),
            event_gateway,
            Arc::clone(&subscription_state)
                as Arc<dyn crate::application::services::SubscriptionStateStore>,
        );
        event_service_inner.set_subscription_invoker(Arc::new(
            EventManagerSubscriptionInvoker::new(Arc::clone(&event_manager)),
        ));
        legacy_event_gateway
            .set_app_handle(app_handle.clone())
            .await;
        let event_service = Arc::new(event_service_inner);

        let messaging_gateway: Arc<dyn MessagingGateway> = Arc::new(NostrMessagingGateway::new(
            Arc::clone(&key_manager),
            Arc::clone(&event_manager),
        ));

        let direct_message_service = Arc::new(DirectMessageService::new(
            Arc::clone(&repository) as Arc<dyn DirectMessageRepository>,
            Arc::clone(&messaging_gateway),
            Some(Arc::new(IpcDirectMessageNotifier::new(app_handle))),
        ));

        {
            let event_manager_for_dm = Arc::clone(&event_manager);
            let key_manager_for_dm = Arc::clone(&key_manager);
            let direct_message_service_for_dm = Arc::clone(&direct_message_service);

            event_manager_for_dm
                .register_event_callback(Arc::new(move |event: NostrEvent| {
                    if event.kind != Kind::EncryptedDirectMessage {
                        return;
                    }

                    let recipient_pubkey =
                        event
                            .tags
                            .iter()
                            .find_map(|tag| match tag.as_standardized() {
                                Some(TagStandard::PublicKey { public_key, .. }) => Some(public_key),
                                _ => None,
                            });

                    let Some(recipient_pubkey) = recipient_pubkey else {
                        return;
                    };

                    let recipient_hex = recipient_pubkey.to_string();
                    let key_manager = Arc::clone(&key_manager_for_dm);
                    let dm_service = Arc::clone(&direct_message_service_for_dm);
                    let event_clone = event.clone();

                    async_runtime::spawn(async move {
                        let keypair = match key_manager.current_keypair().await {
                            Ok(pair) => pair,
                            Err(err) => {
                                tracing::error!(
                                    error = %err,
                                    "Failed to load current keypair for direct message ingestion"
                                );
                                return;
                            }
                        };

                        if keypair.public_key != recipient_hex {
                            return;
                        }

                        let sender_npub = match event_clone.pubkey.to_bech32() {
                            Ok(value) => value,
                            Err(err) => {
                                tracing::error!(
                                    error = %err,
                                    "Failed to convert sender pubkey to npub for direct message"
                                );
                                return;
                            }
                        };

                        let created_at_millis =
                            (event_clone.created_at.as_u64() as i64).saturating_mul(1000);
                        if let Err(err) = dm_service
                            .ingest_incoming_message(
                                &keypair.npub,
                                &sender_npub,
                                &event_clone.content,
                                Some(event_clone.id.to_string()),
                                created_at_millis,
                            )
                            .await
                        {
                            tracing::error!(
                                error = %err,
                                sender = sender_npub,
                                owner = keypair.npub,
                                "Failed to ingest incoming direct message"
                            );
                        }
                    });
                }))
                .await;
        }

        let post_cache: Arc<dyn PostCache> = Arc::new(PostCacheService::new());
        // PostServiceの初期化
        let post_service = Arc::new(PostService::new(
            Arc::clone(&repository) as Arc<dyn PostRepository>,
            Arc::clone(&repository) as Arc<dyn BookmarkRepository>,
            Arc::clone(&event_service) as Arc<dyn EventServiceTrait>,
            Arc::clone(&post_cache),
        ));

        let post_sync_participant: Arc<dyn SyncParticipant> = post_service.clone();
        let event_sync_participant: Arc<dyn SyncParticipant> = event_service.clone();

        let sync_service: Arc<dyn SyncServiceTrait> = Arc::new(SyncService::new(
            Arc::clone(&network_service),
            post_sync_participant,
            event_sync_participant,
        ));

        let profile_avatar_service = Arc::new(
            ProfileAvatarService::new(profile_avatar_dir.clone())
                .await
                .map_err(|e| anyhow::anyhow!("Failed to initialize profile avatar service: {e}"))?,
        );

        // プレゼンテーション層のハンドラーを初期化
        let user_handler = Arc::new(UserHandler::new(Arc::clone(&user_service)));
        let secure_storage_handler = Arc::new(SecureStorageHandler::new(
            Arc::clone(&auth_service),
            Arc::clone(&secure_account_store),
        ));
        let event_handler = Arc::new(EventHandler::new(
            Arc::clone(&event_service)
                as Arc<dyn crate::application::services::event_service::EventServiceTrait>,
            Arc::clone(&key_manager),
            Arc::clone(&event_manager),
        ));
        let p2p_handler = Arc::new(P2PHandler::new(Arc::clone(&p2p_service)));
        let offline_handler = Arc::new(OfflineHandler::new(Arc::clone(&offline_service)
            as Arc<dyn crate::application::services::offline_service::OfflineServiceTrait>));

        // P2P接続イベントを監視し、再接続時に再索引ジョブをトリガー
        {
            let mut event_rx = p2p_event_tx.subscribe();
            let job = Arc::clone(&offline_reindex_job);
            tauri::async_runtime::spawn(async move {
                while let Ok(event) = event_rx.recv().await {
                    if matches!(event, P2PEvent::NetworkConnected { .. }) {
                        job.trigger();
                    }
                }
            });
        }

        {
            let mut event_rx = p2p_event_tx.subscribe();
            let event_service_clone = Arc::clone(&event_service);
            tauri::async_runtime::spawn(async move {
                while let Ok(event) = event_rx.recv().await {
                    match event {
                        P2PEvent::NetworkDisconnected { .. } => {
                            if let Err(e) = event_service_clone.handle_network_disconnected().await
                            {
                                tracing::warn!("Failed to mark subscriptions for resync: {}", e);
                            }
                        }
                        P2PEvent::NetworkConnected { .. } => {
                            if let Err(e) = event_service_clone.handle_network_connected().await {
                                tracing::warn!(
                                    "Failed to restore subscriptions after reconnect: {}",
                                    e
                                );
                            }
                        }
                        _ => {}
                    }
                }
            });
        }

        // UI向けイベント購読を確保
        let p2p_event_rx = p2p_event_tx.subscribe();

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
            event_manager,
            p2p_state,
            offline_reindex_job,
            auth_service,
            post_service,
            topic_service,
            user_service,
            user_search_service,
            event_service,
            direct_message_service,
            sync_service,
            p2p_service,
            offline_service,
            profile_avatar_service,
            user_handler,
            secure_storage_handler,
            event_handler,
            p2p_handler,
            offline_handler,
        };

        // SyncService の定期実行と P2P 接続状態フックをセットアップ
        {
            let sync_service = Arc::clone(&this.sync_service);
            tauri::async_runtime::spawn(async move {
                if let Err(err) = sync_service.start_sync().await {
                    tracing::warn!(error = %err, "initial sync run failed");
                }
            });
        }

        {
            let sync_service = Arc::clone(&this.sync_service);
            tauri::async_runtime::spawn(async move {
                sync_service.schedule_sync(DEFAULT_SYNC_INTERVAL_SECS).await;
            });
        }

        {
            let mut event_rx = p2p_event_tx.subscribe();
            let sync_service = Arc::clone(&this.sync_service);
            tauri::async_runtime::spawn(async move {
                while let Ok(event) = event_rx.recv().await {
                    match event {
                        P2PEvent::NetworkConnected { .. } => {
                            if let Err(err) = sync_service.start_sync().await {
                                tracing::warn!(
                                    error = %err,
                                    "failed to trigger sync after network connect"
                                );
                            }
                        }
                        P2PEvent::NetworkDisconnected { .. } => {
                            if let Err(err) = sync_service.stop_sync().await {
                                tracing::warn!(
                                    error = %err,
                                    "failed to stop sync after network disconnect"
                                );
                            }
                        }
                        _ => {}
                    }
                }
            });
        }

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
                        struct UiMsg {
                            id: String,
                            author: String,
                            content: String,
                            timestamp: i64,
                            signature: String,
                        }
                        #[derive(serde::Serialize, Clone)]
                        struct UiP2PMessageEvent {
                            topic_id: String,
                            message: UiMsg,
                        }

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
            let user_topic = crate::domain::p2p::user_topic_id(&pk.to_string());
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

fn spawn_trending_metrics_job(job: Arc<TrendingMetricsJob>, interval: Duration) {
    tracing::info!(
        target: "metrics::trending",
        interval_seconds = interval.as_secs(),
        "starting trending metrics job loop"
    );
    async_runtime::spawn(async move {
        loop {
            if let Err(err) = job.run_once().await {
                tracing::error!(
                    target: "metrics::trending",
                    error = %err,
                    "trending metrics job run failed"
                );
            }
            sleep(interval).await;
        }
    });
}
