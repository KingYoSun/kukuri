use nostr_sdk::prelude::*;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error};

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

    /// リレーに接続
    pub async fn add_relay(&self, url: &str) -> Result<()> {
        let client_guard = self.client.read().await;
        if let Some(client) = client_guard.as_ref() {
            client.add_relay(url).await?;
            info!("Added relay: {}", url);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Client not initialized"))
        }
    }

    /// 複数のリレーに接続
    pub async fn add_relays(&self, urls: Vec<&str>) -> Result<()> {
        let client_guard = self.client.read().await;
        if let Some(client) = client_guard.as_ref() {
            for url in urls {
                match client.add_relay(url).await {
                    Ok(_) => info!("Added relay: {}", url),
                    Err(e) => error!("Failed to add relay {}: {}", url, e),
                }
            }
            Ok(())
        } else {
            Err(anyhow::anyhow!("Client not initialized"))
        }
    }

    /// 全てのリレーに接続
    pub async fn connect(&self) -> Result<()> {
        let client_guard = self.client.read().await;
        if let Some(client) = client_guard.as_ref() {
            client.connect().await;
            info!("Connected to all relays");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Client not initialized"))
        }
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
    pub async fn publish_text_note(&self, content: &str, tags: Vec<Tag>) -> Result<EventId> {
        let client_guard = self.client.read().await;
        if let Some(client) = client_guard.as_ref() {
            // イベントを作成
            let keys = self.keys.as_ref()
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
    pub fn get_public_key(&self) -> Option<PublicKey> {
        self.keys.as_ref().map(|k| k.public_key())
    }

    /// Clientへの参照を取得
    pub async fn get_client(&self) -> Option<Client> {
        self.client.read().await.clone()
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
}