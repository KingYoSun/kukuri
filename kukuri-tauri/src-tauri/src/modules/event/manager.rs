use super::{handler::EventHandler, publisher::EventPublisher, nostr_client::NostrClientManager};
use crate::modules::auth::key_manager::KeyManager;
use nostr_sdk::prelude::*;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error, debug};

/// Nostrイベントマネージャー - イベント処理の中心的な管理者
pub struct EventManager {
    pub(crate) client_manager: Arc<RwLock<NostrClientManager>>,
    pub(crate) event_handler: Arc<EventHandler>,
    pub(crate) event_publisher: Arc<RwLock<EventPublisher>>,
    is_initialized: Arc<RwLock<bool>>,
}

impl EventManager {
    /// 新しいEventManagerインスタンスを作成
    pub fn new() -> Self {
        Self {
            client_manager: Arc::new(RwLock::new(NostrClientManager::new())),
            event_handler: Arc::new(EventHandler::new()),
            event_publisher: Arc::new(RwLock::new(EventPublisher::new())),
            is_initialized: Arc::new(RwLock::new(false)),
        }
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
        tokio::spawn(async move {
            client.handle_notifications(|notification| async {
                if let RelayPoolNotification::Event { event, .. } = notification {
                    debug!("Received event: {}", event.id);
                    if let Err(e) = event_handler.handle_event(*event).await {
                        error!("Error handling event: {}", e);
                    }
                }
                Ok(false) // Continue listening
            }).await;
        });
        
        info!("Event stream started");
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
}