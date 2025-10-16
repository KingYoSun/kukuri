use anyhow::Result;
use nostr_sdk::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Nostrクライアントの管理構造体
pub struct NostrClientManager {
    client: Arc<RwLock<Option<Client>>>,
    keys: Option<Keys>,
}

impl NostrClientManager {
    /// 新しいNostrClientManagerインスタンスを作成
    pub fn new() -> Self {
        Self {
            client: Arc::new(RwLock::new(None)),
            keys: None,
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
        assert!(manager.disconnect().await.is_err());
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
}
