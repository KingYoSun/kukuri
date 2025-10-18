use super::{handler::EventHandler, nostr_client::NostrClientManager, publisher::EventPublisher};
use crate::domain::entities as domain;
use crate::domain::value_objects::EventId as DomainEventId;
use crate::infrastructure::database::EventRepository as InfraEventRepository;
use crate::infrastructure::p2p::GossipService;
use crate::modules::auth::key_manager::KeyManager;
use crate::modules::database::connection::DbPool;
use crate::modules::p2p::user_topic_id;
use anyhow::Result;
use nostr_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;
use tracing::{error, info};

/// フロントエンドに送信するイベントペイロード
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NostrEventPayload {
    pub id: String,
    pub author: String,
    pub content: String,
    pub created_at: u64,
    pub kind: u32,
    pub tags: Vec<Vec<String>>,
}

/// Nostrイベントマネージャー - イベント処理の中心的な管理者
pub struct EventManager {
    pub(crate) client_manager: Arc<RwLock<NostrClientManager>>,
    pub(crate) event_handler: Arc<EventHandler>,
    pub(crate) event_publisher: Arc<RwLock<EventPublisher>>,
    is_initialized: Arc<RwLock<bool>>,
    app_handle: Arc<RwLock<Option<AppHandle>>>,
    /// P2P配信用のGossipService（任意）
    gossip_service: Arc<RwLock<Option<Arc<dyn GossipService>>>>,
    /// 非トピック系イベントの既定配信先トピック集合（複数）
    selected_default_topic_ids: Arc<RwLock<HashSet<String>>>,
    /// 参照トピック解決用のEventRepository（任意）
    event_repository: Arc<RwLock<Option<Arc<dyn InfraEventRepository>>>>,
}

impl EventManager {
    /// 新しいEventManagerインスタンスを作成
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            client_manager: Arc::new(RwLock::new(NostrClientManager::new())),
            event_handler: Arc::new(EventHandler::new()),
            event_publisher: Arc::new(RwLock::new(EventPublisher::new())),
            is_initialized: Arc::new(RwLock::new(false)),
            app_handle: Arc::new(RwLock::new(None)),
            gossip_service: Arc::new(RwLock::new(None)),
            selected_default_topic_ids: Arc::new(RwLock::new(HashSet::from(
                ["public".to_string()],
            ))),
            event_repository: Arc::new(RwLock::new(None)),
        }
    }

    /// 新しいEventManagerインスタンスをDbPoolと共に作成
    pub fn new_with_db(db_pool: Arc<DbPool>) -> Self {
        let mut event_handler = EventHandler::new();
        event_handler.set_db_pool(db_pool);

        Self {
            client_manager: Arc::new(RwLock::new(NostrClientManager::new())),
            event_handler: Arc::new(event_handler),
            event_publisher: Arc::new(RwLock::new(EventPublisher::new())),
            is_initialized: Arc::new(RwLock::new(false)),
            app_handle: Arc::new(RwLock::new(None)),
            gossip_service: Arc::new(RwLock::new(None)),
            selected_default_topic_ids: Arc::new(RwLock::new(HashSet::from(
                ["public".to_string()],
            ))),
            event_repository: Arc::new(RwLock::new(None)),
        }
    }

    /// AppHandleを設定
    pub async fn set_app_handle(&self, app_handle: AppHandle) {
        let mut handle = self.app_handle.write().await;
        *handle = Some(app_handle);
    }

    // EventSyncは廃止（IrohGossipService経由に移行）

    /// GossipServiceを接続（P2P配信用）。未設定でも動作は継続。
    pub async fn set_gossip_service(&self, gossip: Arc<dyn GossipService>) {
        let mut gs = self.gossip_service.write().await;
        *gs = Some(gossip);
    }

    /// EventRepositoryを接続（参照トピック解決用）。未設定でも動作は継続。
    pub async fn set_event_repository(&self, repo: Arc<dyn InfraEventRepository>) {
        let mut r = self.event_repository.write().await;
        *r = Some(repo);
    }

    /// 既定の配信先トピックIDを設定
    pub async fn set_default_p2p_topic_id(&self, topic_id: impl Into<String>) {
        // 後方互換API: 単一の既定トピックに置き換える
        let mut set = self.selected_default_topic_ids.write().await;
        set.clear();
        set.insert(topic_id.into());
    }

    /// 既定配信先トピックを一括設定（複数）
    pub async fn set_default_p2p_topics(&self, topics: Vec<String>) {
        let mut set = self.selected_default_topic_ids.write().await;
        set.clear();
        for t in topics {
            if !t.is_empty() {
                set.insert(t);
            }
        }
    }

    /// 既定配信先トピックを追加
    pub async fn add_default_p2p_topic(&self, topic_id: impl Into<String>) {
        let mut set = self.selected_default_topic_ids.write().await;
        let t = topic_id.into();
        if !t.is_empty() {
            set.insert(t);
        }
    }

    /// 既定配信先トピックを削除
    pub async fn remove_default_p2p_topic(&self, topic_id: &str) {
        let mut set = self.selected_default_topic_ids.write().await;
        set.remove(topic_id);
    }

    /// 既定配信先トピック一覧を取得
    pub async fn list_default_p2p_topics(&self) -> Vec<String> {
        let set = self.selected_default_topic_ids.read().await;
        set.iter().cloned().collect()
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

    /// テキストノートを投稿
    pub async fn publish_text_note(&self, content: &str) -> Result<EventId> {
        self.ensure_initialized().await?;

        let publisher = self.event_publisher.read().await;
        let event = publisher.create_text_note(content, vec![])?;

        let client_manager = self.client_manager.read().await;
        let event_id = client_manager.publish_event(event.clone()).await?;

        // P2Pネットワークへブロードキャスト（既定トピック + ユーザー固有トピック）
        if let Some(gossip) = self.gossip_service.read().await.as_ref().cloned() {
            let mut topics: HashSet<String> = self.selected_default_topic_ids.read().await.clone();
            if let Some(pk) = self.get_public_key().await {
                topics.insert(user_topic_id(&pk.to_string()));
            }
            let topic_list: Vec<String> = topics.into_iter().collect();
            if let Err(e) = self.broadcast_to_topics(&gossip, &topic_list, &event).await {
                error!("Failed to broadcast to P2P (text_note): {}", e);
            }
        }

        Ok(event_id)
    }

    /// トピック投稿を作成・送信
    pub async fn publish_topic_post(
        &self,
        topic_id: &str,
        content: &str,
        reply_to: Option<EventId>,
    ) -> Result<EventId> {
        self.ensure_initialized().await?;

        let publisher = self.event_publisher.read().await;
        let event = publisher.create_topic_post(topic_id, content, reply_to)?;

        let client_manager = self.client_manager.read().await;
        let event_id = client_manager.publish_event(event.clone()).await?;

        // P2Pネットワークへブロードキャスト（対象トピック）
        if let Some(gossip) = self.gossip_service.read().await.as_ref().cloned() {
            if let Err(e) = self.broadcast_to_topic(&gossip, topic_id, &event).await {
                error!("Failed to broadcast to P2P (topic {}): {}", topic_id, e);
            }
        }

        // 参照マッピングをDBへ保存（可能な場合、冪等）
        if let Some(repo) = self.event_repository.read().await.as_ref().cloned() {
            let _ = repo.add_event_topic(&event.id.to_string(), topic_id).await;
        }

        Ok(event_id)
    }

    /// リアクションを送信
    pub async fn send_reaction(&self, event_id: &EventId, reaction: &str) -> Result<EventId> {
        self.ensure_initialized().await?;

        let publisher = self.event_publisher.read().await;
        let event = publisher.create_reaction(event_id, reaction)?;

        let client_manager = self.client_manager.read().await;
        let result_id = client_manager.publish_event(event.clone()).await?;

        // P2Pネットワークへブロードキャスト（参照イベントの属するトピックがあればそこへ。未解決なら既定+ユーザー）
        if let Some(gossip) = self.gossip_service.read().await.as_ref().cloned() {
            let referenced = event_id.to_hex();
            let resolved_topics = self.resolve_topics_for_referenced_event(&referenced).await;
            let topics: HashSet<String> = if let Some(mut ts) = resolved_topics {
                ts.drain(..).collect()
            } else {
                // フォールバック: 既定 + ユーザー
                let mut set = self.selected_default_topic_ids.read().await.clone();
                if let Some(pk) = self.get_public_key().await {
                    set.insert(user_topic_id(&pk.to_string()));
                }
                set
            };
            let topic_list: Vec<String> = topics.into_iter().collect();
            if let Err(e) = self.broadcast_to_topics(&gossip, &topic_list, &event).await {
                error!("Failed to broadcast reaction to P2P: {}", e);
            }
        }

        Ok(result_id)
    }

    /// リポスト（ブースト）を送信
    /// 任意のイベントを発行
    #[allow(dead_code)]
    pub async fn publish_event(&self, event: Event) -> Result<EventId> {
        self.ensure_initialized().await?;

        let client_manager = self.client_manager.read().await;
        let event_id = client_manager.publish_event(event.clone()).await?;

        // P2Pネットワークへブロードキャスト（既定トピック + ユーザー固有トピック）
        if let Some(gossip) = self.gossip_service.read().await.as_ref().cloned() {
            let mut topics: HashSet<String> = self.selected_default_topic_ids.read().await.clone();
            if let Some(pk) = self.get_public_key().await {
                topics.insert(user_topic_id(&pk.to_string()));
            }
            let topic_list: Vec<String> = topics.into_iter().collect();
            if let Err(e) = self.broadcast_to_topics(&gossip, &topic_list, &event).await {
                error!("Failed to broadcast generic event to P2P: {}", e);
            }
        }

        Ok(event_id)
    }

    /// メタデータを更新
    pub async fn update_metadata(&self, metadata: Metadata) -> Result<EventId> {
        self.ensure_initialized().await?;

        let publisher = self.event_publisher.read().await;
        let event = publisher.create_metadata(metadata)?;

        let client_manager = self.client_manager.read().await;
        let result_id = client_manager.publish_event(event.clone()).await?;

        // P2Pネットワークへブロードキャスト（既定トピック + ユーザー固有トピック）
        if let Some(gossip) = self.gossip_service.read().await.as_ref().cloned() {
            let mut topics: HashSet<String> = self.selected_default_topic_ids.read().await.clone();
            if let Some(pk) = self.get_public_key().await {
                topics.insert(user_topic_id(&pk.to_string()));
            }
            let topic_list: Vec<String> = topics.into_iter().collect();
            if let Err(e) = self.broadcast_to_topics(&gossip, &topic_list, &event).await {
                error!("Failed to broadcast metadata to P2P: {}", e);
            }
        }

        Ok(result_id)
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
    async fn ensure_initialized(&self) -> Result<()> {
        if !*self.is_initialized.read().await {
            Err(anyhow::anyhow!("EventManager not initialized"))
        } else {
            Ok(())
        }
    }

    /// 公開鍵を取得
    pub async fn get_public_key(&self) -> Option<PublicKey> {
        let publisher = self.event_publisher.read().await;
        publisher.get_public_key()
    }

    /// P2Pネットワークから受信したNostrイベントを処理
    pub async fn handle_p2p_event(&self, event: Event) -> Result<()> {
        // 既に処理済みのイベントでないか確認（重複チェックはEventHandlerで行う）
        if let Err(e) = self.event_handler.handle_event(event.clone()).await {
            error!("Error handling P2P event: {}", e);
            return Err(e);
        }

        // フロントエンドにイベントを送信
        if let Some(ref handle) = *self.app_handle.read().await {
            let payload = NostrEventPayload {
                id: event.id.to_string(),
                author: event.pubkey.to_string(),
                content: event.content.clone(),
                created_at: event.created_at.as_u64(),
                kind: event.kind.as_u16() as u32,
                tags: event.tags.iter().map(|tag| tag.clone().to_vec()).collect(),
            };
            let _ = handle.emit("nostr://event/p2p", payload);
        }

        Ok(())
    }

    /// 切断
    pub async fn disconnect(&self) -> Result<()> {
        let client_manager = self.client_manager.read().await;
        client_manager.disconnect().await?;
        *self.is_initialized.write().await = false;
        Ok(())
    }
}

impl EventManager {
    async fn broadcast_to_topic(
        &self,
        gossip: &Arc<dyn GossipService>,
        topic_id: &str,
        nostr_event: &nostr_sdk::Event,
    ) -> Result<()> {
        // 変換：nostr_sdk::Event -> domain::entities::Event
        let domain_event = Self::to_domain_event(nostr_event)?;
        // トピックに参加していない場合は参加（冪等）
        let _ = gossip.join_topic(topic_id, vec![]).await;
        // ブロードキャスト
        gossip
            .broadcast(topic_id, &domain_event)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(())
    }

    /// 複数トピックへ冪等Join + 重複排除つきでブロードキャスト
    async fn broadcast_to_topics(
        &self,
        gossip: &Arc<dyn GossipService>,
        topics: &[String],
        nostr_event: &nostr_sdk::Event,
    ) -> Result<()> {
        // 重複排除
        let mut uniq: HashSet<String> = HashSet::new();
        for t in topics {
            if !t.is_empty() {
                uniq.insert(t.clone());
            }
        }
        if uniq.is_empty() {
            return Ok(());
        }
        // 変換
        let domain_event = Self::to_domain_event(nostr_event)?;
        // 冪等Join + 送信
        for topic in uniq.into_iter() {
            let _ = gossip.join_topic(&topic, vec![]).await; // 冪等
            if let Err(e) = gossip.broadcast(&topic, &domain_event).await {
                error!("Failed to broadcast to topic {}: {}", topic, e);
            }
        }
        Ok(())
    }

    /// 参照イベント（eタグ等）から配信先トピックを解決（Phase A: 仮実装）
    async fn resolve_topics_for_referenced_event(&self, event_id: &str) -> Option<Vec<String>> {
        if let Some(repo) = self.event_repository.read().await.as_ref().cloned() {
            match repo.get_event_topics(event_id).await {
                Ok(v) if !v.is_empty() => return Some(v),
                _ => {}
            }
        }
        None
    }

    fn to_domain_event(nostr: &nostr_sdk::Event) -> Result<domain::Event> {
        // id
        let id_hex = nostr.id.to_string();
        let id = DomainEventId::from_hex(&id_hex).map_err(|e| anyhow::anyhow!(e))?;
        // created_at
        let secs = nostr.created_at.as_u64() as i64;
        let created_at = chrono::DateTime::<chrono::Utc>::from_timestamp(secs, 0)
            .ok_or_else(|| anyhow::anyhow!("invalid timestamp"))?;
        // kind
        let kind = nostr.kind.as_u16() as u32;
        // tags
        let tags: Vec<Vec<String>> = nostr.tags.iter().map(|t| t.clone().to_vec()).collect();
        // sig
        let sig = nostr.sig.to_string();
        // build
        let event = domain::Event::new_with_id(
            id,
            nostr.pubkey.to_string(),
            nostr.content.clone(),
            kind,
            tags,
            created_at,
            sig,
        );
        Ok(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::p2p::TopicStats;
    use crate::shared::error::AppError;
    use async_trait::async_trait;

    struct TestGossipService {
        joined: Arc<RwLock<HashSet<String>>>,
        broadcasts: Arc<RwLock<Vec<(String, domain::Event)>>>,
    }

    impl TestGossipService {
        fn new() -> Self {
            Self {
                joined: Arc::new(RwLock::new(HashSet::new())),
                broadcasts: Arc::new(RwLock::new(Vec::new())),
            }
        }

        async fn joined_topics(&self) -> HashSet<String> {
            self.joined.read().await.clone()
        }

        async fn broadcasted_topics(&self) -> Vec<String> {
            self.broadcasts
                .read()
                .await
                .iter()
                .map(|(t, _)| t.clone())
                .collect()
        }
    }

    #[async_trait]
    impl GossipService for TestGossipService {
        async fn join_topic(
            &self,
            topic: &str,
            _initial_peers: Vec<String>,
        ) -> Result<(), AppError> {
            let mut j = self.joined.write().await;
            j.insert(topic.to_string());
            Ok(())
        }
        async fn leave_topic(&self, _topic: &str) -> Result<(), AppError> {
            Ok(())
        }
        async fn broadcast(&self, topic: &str, event: &domain::Event) -> Result<(), AppError> {
            let mut b = self.broadcasts.write().await;
            b.push((topic.to_string(), event.clone()));
            Ok(())
        }
        async fn subscribe(
            &self,
            _topic: &str,
        ) -> Result<tokio::sync::mpsc::Receiver<domain::Event>, AppError> {
            let (_tx, rx) = tokio::sync::mpsc::channel(1);
            Ok(rx)
        }
        async fn get_joined_topics(&self) -> Result<Vec<String>, AppError> {
            Ok(vec![])
        }
        async fn get_topic_peers(&self, _topic: &str) -> Result<Vec<String>, AppError> {
            Ok(vec![])
        }

        async fn get_topic_stats(&self, _topic: &str) -> Result<Option<TopicStats>, AppError> {
            Ok(None)
        }
        async fn broadcast_message(&self, _topic: &str, _message: &[u8]) -> Result<(), AppError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_event_manager_initialization() {
        let manager = EventManager::new();
        let key_manager = KeyManager::new();

        // 新しい鍵ペアを生成
        let _ = key_manager.generate_keypair().await.unwrap();

        assert!(
            manager
                .initialize_with_key_manager(&key_manager)
                .await
                .is_ok()
        );
        assert!(manager.get_public_key().await.is_some());
    }

    #[tokio::test]
    async fn test_event_manager_not_initialized() {
        let manager = EventManager::new();

        // 初期化前はエラーになることを確認
        assert!(manager.publish_text_note("test").await.is_err());
        assert!(
            manager
                .publish_topic_post("topic", "content", None)
                .await
                .is_err()
        );
        assert!(manager.subscribe_to_topic("topic", None).await.is_err());
    }

    #[tokio::test]
    async fn test_initialize_and_disconnect() {
        let manager = EventManager::new();
        let key_manager = KeyManager::new();

        // 鍵ペアを生成
        key_manager.generate_keypair().await.unwrap();

        // 初期化
        manager
            .initialize_with_key_manager(&key_manager)
            .await
            .unwrap();
        assert!(*manager.is_initialized.read().await);

        // 切断
        manager.disconnect().await.unwrap();
        assert!(!*manager.is_initialized.read().await);
    }

    #[tokio::test]
    async fn test_get_public_key() {
        let manager = EventManager::new();
        let key_manager = KeyManager::new();

        // 初期化前は公開鍵がない
        assert!(manager.get_public_key().await.is_none());

        // 初期化後は公開鍵が取得できる
        key_manager.generate_keypair().await.unwrap();
        manager
            .initialize_with_key_manager(&key_manager)
            .await
            .unwrap();

        let public_key = manager.get_public_key().await.unwrap();
        assert_eq!(
            public_key,
            key_manager.get_keys().await.unwrap().public_key()
        );
    }

    #[tokio::test]
    async fn test_create_events() {
        let manager = EventManager::new();
        let key_manager = KeyManager::new();

        // 初期化
        key_manager.generate_keypair().await.unwrap();
        manager
            .initialize_with_key_manager(&key_manager)
            .await
            .unwrap();

        // 各種イベントの作成をテスト
        let publisher = manager.event_publisher.read().await;

        // テキストノート
        let text_event = publisher.create_text_note("Test note", vec![]).unwrap();
        assert_eq!(text_event.kind, Kind::TextNote);

        // メタデータ
        let metadata = Metadata::new().name("Test User");
        let metadata_event = publisher.create_metadata(metadata).unwrap();
        assert_eq!(metadata_event.kind, Kind::Metadata);

        // リアクション
        let event_id = EventId::from_slice(&[1; 32]).unwrap();
        let reaction_event = publisher.create_reaction(&event_id, "+").unwrap();
        assert_eq!(reaction_event.kind, Kind::Reaction);
    }

    #[tokio::test]
    async fn test_ensure_initialized() {
        let manager = EventManager::new();

        // 初期化前
        assert!(manager.ensure_initialized().await.is_err());

        // 初期化後
        let key_manager = KeyManager::new();
        key_manager.generate_keypair().await.unwrap();
        manager
            .initialize_with_key_manager(&key_manager)
            .await
            .unwrap();

        assert!(manager.ensure_initialized().await.is_ok());
    }

    #[tokio::test]
    async fn test_event_payload_creation() {
        let keys = Keys::generate();
        let event = EventBuilder::text_note("Test content")
            .tags(vec![Tag::hashtag("test")])
            .sign_with_keys(&keys)
            .unwrap();

        let payload = NostrEventPayload {
            id: event.id.to_string(),
            author: event.pubkey.to_string(),
            content: event.content.clone(),
            created_at: event.created_at.as_u64(),
            kind: event.kind.as_u16() as u32,
            tags: event.tags.iter().map(|tag| tag.clone().to_vec()).collect(),
        };

        assert_eq!(payload.id, event.id.to_string());
        assert_eq!(payload.content, "Test content");
        assert_eq!(payload.kind, 1); // TextNote
        assert!(!payload.tags.is_empty());
    }

    #[tokio::test]
    async fn test_default_topics_api() {
        let manager = EventManager::new();

        // 初期はpublicのみ
        let mut topics = manager.list_default_p2p_topics().await;
        topics.sort();
        assert_eq!(topics, vec!["public".to_string()]);

        // 一括設定
        manager
            .set_default_p2p_topics(vec!["a".into(), "b".into()])
            .await;
        let mut topics = manager.list_default_p2p_topics().await;
        topics.sort();
        assert_eq!(topics, vec!["a".to_string(), "b".to_string()]);

        // 追加と削除
        manager.add_default_p2p_topic("c").await;
        manager.remove_default_p2p_topic("b").await;
        let mut topics = manager.list_default_p2p_topics().await;
        topics.sort();
        assert_eq!(topics, vec!["a".to_string(), "c".to_string()]);
    }

    #[tokio::test]
    async fn test_routing_non_topic_includes_user_and_defaults() {
        let manager = EventManager::new();
        let key_manager = KeyManager::new();
        // 鍵生成と初期化
        key_manager.generate_keypair().await.unwrap();
        manager
            .initialize_with_key_manager(&key_manager)
            .await
            .unwrap();

        // 既定トピックを2つ設定
        manager
            .set_default_p2p_topics(vec!["t1".into(), "t2".into()])
            .await;

        // テスト用GossipServiceを設定
        let gossip = Arc::new(TestGossipService::new());
        manager.set_gossip_service(gossip.clone()).await;

        // イベント作成（Nostrへのpublishは避け、直接P2Pブロードキャスト経路を検証）
        let publisher = manager.event_publisher.read().await;
        let nostr_event = publisher.create_text_note("hello", vec![]).unwrap();
        let mut topics = manager.list_default_p2p_topics().await;
        if let Some(pk) = manager.get_public_key().await {
            topics.push(user_topic_id(&pk.to_string()));
        }
        manager
            .broadcast_to_topics(
                &(gossip.clone() as Arc<dyn GossipService>),
                &topics,
                &nostr_event,
            )
            .await
            .unwrap();

        // 参加済トピックにt1, t2, userトピックが含まれる
        let joined = gossip.joined_topics().await;
        let pubkey = manager.get_public_key().await.unwrap();
        let user_topic = user_topic_id(&pubkey.to_string());
        assert!(joined.contains("t1"));
        assert!(joined.contains("t2"));
        assert!(joined.contains(&user_topic));

        // ブロードキャスト先も同様に3件（重複なし）
        let mut b = gossip.broadcasted_topics().await;
        b.sort();
        assert_eq!(b, {
            let mut v = vec!["t1".to_string(), "t2".to_string(), user_topic];
            v.sort();
            v
        });
    }
}
