use crate::application::shared::default_topics::DefaultTopicsRegistry;
use crate::application::shared::nostr::EventPublisher;
use crate::infrastructure::database::{
    EventRepository as InfraEventRepository, connection_pool::ConnectionPool,
};
use crate::infrastructure::p2p::GossipService;
use crate::modules::auth::key_manager::KeyManager;
use crate::modules::event::handler::EventHandler;
use crate::modules::event::nostr_client::NostrClientManager;
use anyhow::{Result, anyhow};
use nostr_sdk::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Nostrイベントマネージャー - イベント処理の中心的な管理者
pub struct EventManager {
    pub(crate) client_manager: Arc<RwLock<NostrClientManager>>,
    pub(crate) event_handler: Arc<EventHandler>,
    pub(crate) event_publisher: Arc<RwLock<EventPublisher>>,
    pub(crate) default_topics: Arc<DefaultTopicsRegistry>,
    is_initialized: Arc<RwLock<bool>>,
    /// P2P配信用のGossipService（任意）
    pub(crate) gossip_service: Arc<RwLock<Option<Arc<dyn GossipService>>>>,
    /// 参照トピック解決用のEventRepository（任意）
    pub(crate) event_repository: Arc<RwLock<Option<Arc<dyn InfraEventRepository>>>>,
}

impl EventManager {
    /// 新しいEventManagerインスタンスを作成
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            client_manager: Arc::new(RwLock::new(NostrClientManager::new())),
            event_handler: Arc::new(EventHandler::new()),
            event_publisher: Arc::new(RwLock::new(EventPublisher::new())),
            default_topics: Arc::new(DefaultTopicsRegistry::with_topics(["public".into()])),
            is_initialized: Arc::new(RwLock::new(false)),
            gossip_service: Arc::new(RwLock::new(None)),
            event_repository: Arc::new(RwLock::new(None)),
        }
    }

    /// 新しいEventManagerインスタンスをConnectionPoolと共に作成
    pub fn new_with_connection_pool(pool: ConnectionPool) -> Self {
        let mut event_handler = EventHandler::new();
        event_handler.set_connection_pool(pool);

        Self {
            client_manager: Arc::new(RwLock::new(NostrClientManager::new())),
            event_handler: Arc::new(event_handler),
            event_publisher: Arc::new(RwLock::new(EventPublisher::new())),
            default_topics: Arc::new(DefaultTopicsRegistry::with_topics(["public".into()])),
            is_initialized: Arc::new(RwLock::new(false)),
            gossip_service: Arc::new(RwLock::new(None)),
            event_repository: Arc::new(RwLock::new(None)),
        }
    }

    /// 既定の配信先トピックIDを設定
    pub async fn set_default_p2p_topic_id(&self, topic_id: impl Into<String>) {
        self.default_topics
            .replace_with_single(topic_id.into())
            .await;
    }

    /// 既定配信先トピックを一括設定（複数）
    pub async fn set_default_p2p_topics(&self, topics: Vec<String>) {
        self.default_topics.replace_all(topics).await;
    }

    /// 既定配信先トピックを追加
    pub async fn add_default_p2p_topic(&self, topic_id: impl Into<String>) {
        self.default_topics.add(topic_id.into()).await;
    }

    /// 既定配信先トピックを削除
    pub async fn remove_default_p2p_topic(&self, topic_id: &str) {
        self.default_topics.remove(topic_id).await;
    }

    /// 既定配信先トピック一覧を取得
    pub async fn list_default_p2p_topics(&self) -> Vec<String> {
        self.default_topics.list().await
    }

    /// KeyManagerからの秘密鍵でマネージャーを初期化
    pub async fn initialize_with_key_manager(&self, key_manager: &KeyManager) -> Result<()> {
        let keys = key_manager.get_keys().await?;
        let secret_key = keys.secret_key();

        // クライアントマネージャーを初期化
        let mut client_manager = self.client_manager.write().await;
        client_manager.init_with_keys(secret_key).await?;

        // パブリッシャーに鍵を設定
        let mut publisher = self.event_publisher.write().await;
        publisher.set_keys(keys);

        *self.is_initialized.write().await = true;

        info!("EventManager initialized successfully");
        Ok(())
    }

    /// 特定のトピックをサブスクライブ
    pub async fn subscribe_to_topic(&self, topic_id: &str, since: Option<Timestamp>) -> Result<()> {
        self.ensure_initialized().await?;

        let mut filter = Filter::new().hashtag(topic_id).kind(Kind::TextNote);
        if let Some(since_ts) = since {
            filter = filter.since(since_ts);
        }

        let client_manager = self.client_manager.read().await;
        client_manager.subscribe(vec![filter]).await?;

        info!("Subscribed to topic: {}", topic_id);
        Ok(())
    }

    /// ユーザーの投稿をサブスクライブ
    pub async fn subscribe_to_user(
        &self,
        pubkey: PublicKey,
        since: Option<Timestamp>,
    ) -> Result<()> {
        self.ensure_initialized().await?;

        let mut filter = Filter::new().author(pubkey).kind(Kind::TextNote);
        if let Some(since_ts) = since {
            filter = filter.since(since_ts);
        }

        let client_manager = self.client_manager.read().await;
        client_manager.subscribe(vec![filter]).await?;

        info!("Subscribed to user: {}", pubkey);
        Ok(())
    }

    /// 初期化状態を確認
    pub(crate) async fn ensure_initialized(&self) -> Result<()> {
        if !*self.is_initialized.read().await {
            Err(anyhow!("EventManager not initialized"))
        } else {
            Ok(())
        }
    }

    /// 公開鍵を取得
    pub async fn get_public_key(&self) -> Option<PublicKey> {
        let publisher = self.event_publisher.read().await;
        publisher.get_public_key()
    }

    /// 切断
    pub async fn disconnect(&self) -> Result<()> {
        let client_manager = self.client_manager.read().await;
        client_manager.disconnect().await?;
        *self.is_initialized.write().await = false;
        Ok(())
    }
}

impl Default for EventManager {
    fn default() -> Self {
        Self::new()
    }
}
