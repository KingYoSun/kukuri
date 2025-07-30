use anyhow::Result;
use nostr_sdk::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// リレーの接続状態
#[derive(Debug, Clone, PartialEq)]
pub enum RelayStatus {
    Connecting,
    Connected,
    Disconnected,
    Error(String),
}

/// Nostrクライアントの管理構造体
pub struct NostrClientManager {
    client: Arc<RwLock<Option<Client>>>,
    keys: Option<Keys>,
    relay_status: Arc<RwLock<HashMap<String, RelayStatus>>>,
}

impl NostrClientManager {
    /// 新しいNostrClientManagerインスタンスを作成
    pub fn new() -> Self {
        Self {
            client: Arc::new(RwLock::new(None)),
            keys: None,
            relay_status: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 秘密鍵からクライアントを初期化
    pub async fn init_with_keys(&mut self, secret_key: &SecretKey) -> Result<()> {
        let keys = Keys::new(secret_key.clone());
        self.keys = Some(keys.clone());

        let client = Client::new(keys.clone());

        *self.client.write().await = Some(client);

        info!("Nostr client initialized with keys");
        Ok(())
    }

    /// リレーに接続
    pub async fn add_relay(&self, url: &str) -> Result<()> {
        // クライアントが初期化されているかチェック
        if self.client.read().await.is_none() {
            return Err(anyhow::anyhow!("Client not initialized"));
        }

        // 既存のNostrリレーへの接続を無効化
        info!("Skipping relay connection to {} (disabled)", url);

        // 接続状態を「接続済み」に設定（モック）
        {
            let mut status = self.relay_status.write().await;
            status.insert(url.to_string(), RelayStatus::Connected);
        }

        Ok(())
    }

    /// 複数のリレーに接続
    pub async fn add_relays(&self, urls: Vec<&str>) -> Result<()> {
        // 既存のNostrリレーへの接続を無効化
        for url in urls {
            info!("Skipping relay connection to {} (disabled)", url);

            // 接続状態を「接続済み」に設定（モック）
            let mut status = self.relay_status.write().await;
            status.insert(url.to_string(), RelayStatus::Connected);
        }
        Ok(())
    }

    /// 全てのリレーに接続
    pub async fn connect(&self) -> Result<()> {
        // クライアントが初期化されているかチェック
        if self.client.read().await.is_none() {
            return Err(anyhow::anyhow!("Client not initialized"));
        }

        // 既存のNostrリレーへの接続を無効化
        info!("Skipping connection to all relays (disabled)");
        Ok(())
    }

    /// 全てのリレーから切断
    pub async fn disconnect(&self) -> Result<()> {
        let client_guard = self.client.read().await;
        if let Some(client) = client_guard.as_ref() {
            client.disconnect().await;
            info!("Disconnected from all relays");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Client not initialized"))
        }
    }

    /// テキストノートを投稿
    #[allow(dead_code)]
    pub async fn publish_text_note(&self, content: &str, tags: Vec<Tag>) -> Result<EventId> {
        let client_guard = self.client.read().await;
        if let Some(client) = client_guard.as_ref() {
            // イベントを作成
            let keys = self
                .keys
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Keys not set"))?;
            let event = EventBuilder::text_note(content)
                .tags(tags)
                .sign_with_keys(keys)?;

            let output = client.send_event(&event).await?;
            let event_id = output.id();
            info!("Published text note: {}", event_id);
            Ok(*event_id)
        } else {
            Err(anyhow::anyhow!("Client not initialized"))
        }
    }

    /// カスタムイベントを投稿
    pub async fn publish_event(&self, event: Event) -> Result<EventId> {
        let client_guard = self.client.read().await;
        if let Some(client) = client_guard.as_ref() {
            let output = client.send_event(&event).await?;
            let event_id = output.id();
            info!("Published event: {}", event_id);
            Ok(*event_id)
        } else {
            Err(anyhow::anyhow!("Client not initialized"))
        }
    }

    /// イベントをサブスクライブ
    pub async fn subscribe(&self, filters: Vec<Filter>) -> Result<()> {
        let client_guard = self.client.read().await;
        if let Some(client) = client_guard.as_ref() {
            for filter in filters {
                client.subscribe(filter, None).await?;
            }
            info!("Subscribed to filters");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Client not initialized"))
        }
    }

    /// 公開鍵を取得
    #[allow(dead_code)]
    pub fn get_public_key(&self) -> Option<PublicKey> {
        self.keys.as_ref().map(|k| k.public_key())
    }

    /// Clientへの参照を取得
    pub async fn get_client(&self) -> Option<Client> {
        self.client.read().await.clone()
    }

    /// リレーの接続状態を取得
    pub async fn get_relay_status(&self) -> HashMap<String, RelayStatus> {
        self.relay_status.read().await.clone()
    }

    /// 全リレーのヘルスチェックを実行
    pub async fn health_check(&self) -> Result<HashMap<String, bool>> {
        let client_guard = self.client.read().await;
        let mut health_status = HashMap::new();

        if let Some(client) = client_guard.as_ref() {
            let relays = client.pool().relays().await;

            for (url, relay) in relays {
                // リレーの接続状態を確認
                let is_connected = relay.is_connected();

                if is_connected {
                    info!("Relay {} is healthy", url);
                    health_status.insert(url.to_string(), true);

                    let mut status = self.relay_status.write().await;
                    status.insert(url.to_string(), RelayStatus::Connected);
                } else {
                    warn!("Relay {} is not connected", url);
                    health_status.insert(url.to_string(), false);

                    let mut status = self.relay_status.write().await;
                    status.insert(url.to_string(), RelayStatus::Disconnected);
                }
            }

            Ok(health_status)
        } else {
            Err(anyhow::anyhow!("Client not initialized"))
        }
    }

    /// 切断されたリレーに再接続を試みる
    pub async fn reconnect_failed_relays(&self) -> Result<()> {
        // 既存のNostrリレーへの再接続を無効化
        info!("Skipping reconnection to failed relays (disabled)");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_initialization() {
        let mut manager = NostrClientManager::new();
        let secret_key = SecretKey::generate();

        assert!(manager.init_with_keys(&secret_key).await.is_ok());
        assert!(manager.get_public_key().is_some());
    }

    #[tokio::test]
    async fn test_client_not_initialized_error() {
        let manager = NostrClientManager::new();

        // クライアントが初期化されていない状態でのテスト
        assert!(manager.add_relay("wss://relay.test").await.is_err());
        assert!(manager.connect().await.is_err());
        assert!(manager.disconnect().await.is_err());
        assert!(manager.publish_text_note("test", vec![]).await.is_err());
    }

    #[tokio::test]
    async fn test_relay_status_management() {
        let mut manager = NostrClientManager::new();
        let secret_key = SecretKey::generate();

        // クライアントを初期化
        manager.init_with_keys(&secret_key).await.unwrap();

        // リレーステータスの初期状態を確認
        let status = manager.get_relay_status().await;
        assert!(status.is_empty());
    }

    #[tokio::test]
    async fn test_public_key_generation() {
        let mut manager = NostrClientManager::new();
        let secret_key = SecretKey::generate();

        // 初期化前は公開鍵がない
        assert!(manager.get_public_key().is_none());

        // 初期化後は公開鍵が取得できる
        manager.init_with_keys(&secret_key).await.unwrap();
        let public_key = manager.get_public_key().unwrap();
        assert_eq!(public_key, Keys::new(secret_key).public_key());
    }

    #[tokio::test]
    async fn test_client_reinitialization() {
        let mut manager = NostrClientManager::new();
        let secret_key1 = SecretKey::generate();
        let secret_key2 = SecretKey::generate();

        // 最初の初期化
        manager.init_with_keys(&secret_key1).await.unwrap();
        let public_key1 = manager.get_public_key().unwrap();

        // 再初期化
        manager.init_with_keys(&secret_key2).await.unwrap();
        let public_key2 = manager.get_public_key().unwrap();

        // 公開鍵が更新されていることを確認
        assert_ne!(public_key1, public_key2);
        assert_eq!(public_key2, Keys::new(secret_key2).public_key());
    }

    #[tokio::test]
    async fn test_get_client() {
        let mut manager = NostrClientManager::new();
        let secret_key = SecretKey::generate();

        // 初期化前はクライアントがない
        assert!(manager.get_client().await.is_none());

        // 初期化後はクライアントが取得できる
        manager.init_with_keys(&secret_key).await.unwrap();
        assert!(manager.get_client().await.is_some());
    }
}
