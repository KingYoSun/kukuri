use super::{handler::EventHandler, publisher::EventPublisher, nostr_client::NostrClientManager};
use crate::modules::auth::key_manager::KeyManager;
use nostr_sdk::prelude::*;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error, debug};
use std::time::Duration;
use tokio::time;
use tauri::{AppHandle, Emitter};
use serde::Serialize;

/// フロントエンドに送信するイベントペイロード
#[derive(Debug, Serialize, Clone)]
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
}

impl EventManager {
    /// 新しいEventManagerインスタンスを作成
    pub fn new() -> Self {
        Self {
            client_manager: Arc::new(RwLock::new(NostrClientManager::new())),
            event_handler: Arc::new(EventHandler::new()),
            event_publisher: Arc::new(RwLock::new(EventPublisher::new())),
            is_initialized: Arc::new(RwLock::new(false)),
            app_handle: Arc::new(RwLock::new(None)),
        }
    }

    /// AppHandleを設定
    pub async fn set_app_handle(&self, app_handle: AppHandle) {
        let mut handle = self.app_handle.write().await;
        *handle = Some(app_handle);
    }

    /// KeyManagerからの秘密鍵でマネージャーを初期化
    pub async fn initialize_with_key_manager(&self, key_manager: &KeyManager) -> Result<()> {
        let keys = key_manager.get_keys().await?;
        let secret_key = keys.secret_key();
        
        // クライアントマネージャーを初期化
        let mut client_manager = self.client_manager.write().await;
        client_manager.init_with_keys(&secret_key).await?;
        
        // パブリッシャーに鍵を設定
        let mut publisher = self.event_publisher.write().await;
        publisher.set_keys(keys);
        
        *self.is_initialized.write().await = true;
        
        info!("EventManager initialized successfully");
        Ok(())
    }

    /// デフォルトリレーに接続
    pub async fn connect_to_default_relays(&self) -> Result<()> {
        let default_relays = vec![
            "wss://relay.damus.io",
            "wss://relay.nostr.band",
            "wss://nos.lol",
            "wss://relay.snort.social",
            "wss://relay.current.fyi",
        ];
        
        let client_manager = self.client_manager.read().await;
        client_manager.add_relays(default_relays).await?;
        client_manager.connect().await?;
        
        info!("Connected to default relays");
        Ok(())
    }

    /// カスタムリレーに接続
    pub async fn add_relay(&self, url: &str) -> Result<()> {
        let client_manager = self.client_manager.read().await;
        client_manager.add_relay(url).await
    }

    /// テキストノートを投稿
    pub async fn publish_text_note(&self, content: &str) -> Result<EventId> {
        self.ensure_initialized().await?;
        
        let publisher = self.event_publisher.read().await;
        let event = publisher.create_text_note(content, vec![])?;
        
        let client_manager = self.client_manager.read().await;
        client_manager.publish_event(event.clone()).await
    }

    /// トピック投稿を作成・送信
    pub async fn publish_topic_post(&self, topic_id: &str, content: &str, reply_to: Option<EventId>) -> Result<EventId> {
        self.ensure_initialized().await?;
        
        let publisher = self.event_publisher.read().await;
        let event = publisher.create_topic_post(topic_id, content, reply_to)?;
        
        let client_manager = self.client_manager.read().await;
        client_manager.publish_event(event.clone()).await
    }

    /// リアクションを送信
    pub async fn send_reaction(&self, event_id: &EventId, reaction: &str) -> Result<EventId> {
        self.ensure_initialized().await?;
        
        let publisher = self.event_publisher.read().await;
        let event = publisher.create_reaction(event_id, reaction)?;
        
        let client_manager = self.client_manager.read().await;
        client_manager.publish_event(event.clone()).await
    }

    /// メタデータを更新
    pub async fn update_metadata(&self, metadata: Metadata) -> Result<EventId> {
        self.ensure_initialized().await?;
        
        let publisher = self.event_publisher.read().await;
        let event = publisher.create_metadata(metadata)?;
        
        let client_manager = self.client_manager.read().await;
        client_manager.publish_event(event.clone()).await
    }

    /// 特定のトピックをサブスクライブ
    pub async fn subscribe_to_topic(&self, topic_id: &str) -> Result<()> {
        self.ensure_initialized().await?;
        
        let filter = Filter::new()
            .hashtag(topic_id)
            .kind(Kind::TextNote);
        
        let client_manager = self.client_manager.read().await;
        client_manager.subscribe(vec![filter]).await?;
        
        info!("Subscribed to topic: {}", topic_id);
        Ok(())
    }

    /// ユーザーの投稿をサブスクライブ
    pub async fn subscribe_to_user(&self, pubkey: PublicKey) -> Result<()> {
        self.ensure_initialized().await?;
        
        let filter = Filter::new()
            .author(pubkey)
            .kind(Kind::TextNote);
        
        let client_manager = self.client_manager.read().await;
        client_manager.subscribe(vec![filter]).await?;
        
        info!("Subscribed to user: {}", pubkey);
        Ok(())
    }

    /// イベントストリームを開始
    pub async fn start_event_stream(&self) -> Result<()> {
        self.ensure_initialized().await?;
        
        let client_manager = self.client_manager.read().await;
        let client = client_manager.get_client().await
            .ok_or_else(|| anyhow::anyhow!("Client not initialized"))?;
        
        let event_handler = Arc::clone(&self.event_handler);
        
        // イベントストリームを非同期で処理
        let app_handle = Arc::clone(&self.app_handle);
        tokio::spawn(async move {
            let _ = client.handle_notifications(|notification| async {
                if let RelayPoolNotification::Event { event, .. } = notification {
                    debug!("Received event: {}", event.id);
                    
                    // フロントエンドにイベントを送信
                    if let Some(ref handle) = *app_handle.read().await {
                        let payload = NostrEventPayload {
                            id: event.id.to_string(),
                            author: event.pubkey.to_string(),
                            content: event.content.clone(),
                            created_at: event.created_at.as_u64(),
                            kind: event.kind.as_u16() as u32,
                            tags: event.tags.iter().map(|tag| {
                                tag.clone().to_vec()
                            }).collect(),
                        };
                        let _ = handle.emit("nostr://event", payload);
                    }
                    
                    if let Err(e) = event_handler.handle_event(*event).await {
                        error!("Error handling event: {}", e);
                    }
                }
                Ok(false) // Continue listening
            }).await;
        });
        
        // 定期的なヘルスチェックを開始
        self.start_health_check_loop().await?;
        
        info!("Event stream started");
        Ok(())
    }

    /// 定期的なヘルスチェックループを開始
    async fn start_health_check_loop(&self) -> Result<()> {
        let client_manager = Arc::clone(&self.client_manager);
        
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(60)); // 60秒ごとにチェック
            
            loop {
                interval.tick().await;
                
                let manager = client_manager.read().await;
                if let Err(e) = manager.health_check().await {
                    error!("Health check failed: {}", e);
                }
                
                // 失敗したリレーに再接続を試みる
                if let Err(e) = manager.reconnect_failed_relays().await {
                    error!("Failed to reconnect to relays: {}", e);
                }
            }
        });
        
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

    /// 切断
    pub async fn disconnect(&self) -> Result<()> {
        let client_manager = self.client_manager.read().await;
        client_manager.disconnect().await?;
        *self.is_initialized.write().await = false;
        Ok(())
    }

    /// リレーの接続状態を取得
    pub async fn get_relay_status(&self) -> Result<Vec<(String, String)>> {
        let client_manager = self.client_manager.read().await;
        let status = client_manager.get_relay_status().await;
        
        let result: Vec<(String, String)> = status
            .into_iter()
            .map(|(url, status)| {
                let status_str = match status {
                    super::nostr_client::RelayStatus::Connecting => "connecting".to_string(),
                    super::nostr_client::RelayStatus::Connected => "connected".to_string(),
                    super::nostr_client::RelayStatus::Disconnected => "disconnected".to_string(),
                    super::nostr_client::RelayStatus::Error(e) => format!("error: {}", e),
                };
                (url, status_str)
            })
            .collect();
        
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_manager_initialization() {
        let manager = EventManager::new();
        let key_manager = KeyManager::new();
        
        // 新しい鍵ペアを生成
        let _ = key_manager.generate_keypair().await.unwrap();
        
        assert!(manager.initialize_with_key_manager(&key_manager).await.is_ok());
        assert!(manager.get_public_key().await.is_some());
    }

    #[tokio::test]
    async fn test_event_manager_not_initialized() {
        let manager = EventManager::new();
        
        // 初期化前はエラーになることを確認
        assert!(manager.publish_text_note("test").await.is_err());
        assert!(manager.publish_topic_post("topic", "content", None).await.is_err());
        assert!(manager.subscribe_to_topic("topic").await.is_err());
    }

    #[tokio::test]
    async fn test_initialize_and_disconnect() {
        let manager = EventManager::new();
        let key_manager = KeyManager::new();
        
        // 鍵ペアを生成
        key_manager.generate_keypair().await.unwrap();
        
        // 初期化
        manager.initialize_with_key_manager(&key_manager).await.unwrap();
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
        manager.initialize_with_key_manager(&key_manager).await.unwrap();
        
        let public_key = manager.get_public_key().await.unwrap();
        assert_eq!(public_key, key_manager.get_keys().await.unwrap().public_key());
    }

    #[tokio::test]
    async fn test_relay_operations() {
        let manager = EventManager::new();
        let key_manager = KeyManager::new();
        
        // 初期化
        key_manager.generate_keypair().await.unwrap();
        manager.initialize_with_key_manager(&key_manager).await.unwrap();
        
        // リレーを追加
        // 注: 実際のリレーに接続しないようにテスト用URLを使用
        assert!(manager.add_relay("wss://test.relay").await.is_ok());
    }

    #[tokio::test]
    async fn test_create_events() {
        let manager = EventManager::new();
        let key_manager = KeyManager::new();
        
        // 初期化
        key_manager.generate_keypair().await.unwrap();
        manager.initialize_with_key_manager(&key_manager).await.unwrap();
        
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
    async fn test_get_relay_status() {
        let manager = EventManager::new();
        let key_manager = KeyManager::new();
        
        // 初期化
        key_manager.generate_keypair().await.unwrap();
        manager.initialize_with_key_manager(&key_manager).await.unwrap();
        
        // リレーステータスを取得
        let status = manager.get_relay_status().await.unwrap();
        assert!(status.is_empty()); // 初期状態は空
    }

    #[tokio::test]
    async fn test_ensure_initialized() {
        let manager = EventManager::new();
        
        // 初期化前
        assert!(manager.ensure_initialized().await.is_err());
        
        // 初期化後
        let key_manager = KeyManager::new();
        key_manager.generate_keypair().await.unwrap();
        manager.initialize_with_key_manager(&key_manager).await.unwrap();
        
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
}