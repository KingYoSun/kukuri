
use nostr_sdk::prelude::*;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, error};

/// Nostrイベントハンドラー
pub struct EventHandler {
    event_callbacks: Arc<RwLock<Vec<Box<dyn Fn(Event) + Send + Sync>>>>,
}

impl EventHandler {
    /// 新しいEventHandlerインスタンスを作成
    pub fn new() -> Self {
        Self {
            event_callbacks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// イベントコールバックを追加
    pub async fn add_callback<F>(&self, callback: F)
    where
        F: Fn(Event) + Send + Sync + 'static,
    {
        let mut callbacks = self.event_callbacks.write().await;
        callbacks.push(Box::new(callback));
    }

    /// イベントを処理
    pub async fn handle_event(&self, event: Event) -> Result<()> {
        debug!("Handling event: {}", event.id);
        
        let callbacks = self.event_callbacks.read().await;
        for callback in callbacks.iter() {
            callback(event.clone());
        }
        
        // イベントの種類に応じた処理
        match event.kind {
            Kind::TextNote => {
                self.handle_text_note(&event).await?;
            }
            Kind::Metadata => {
                self.handle_metadata(&event).await?;
            }
            Kind::ContactList => {
                self.handle_contact_list(&event).await?;
            }
            Kind::Reaction => {
                self.handle_reaction(&event).await?;
            }
            _ => {
                debug!("Unhandled event kind: {:?}", event.kind);
            }
        }
        
        Ok(())
    }

    /// テキストノートイベントを処理
    async fn handle_text_note(&self, event: &Event) -> Result<()> {
        info!("Received text note from {}: {}", event.pubkey, event.content);
        // TODO: データベースに保存する処理を実装
        Ok(())
    }

    /// メタデータイベントを処理
    async fn handle_metadata(&self, event: &Event) -> Result<()> {
        info!("Received metadata update from {}", event.pubkey);
        // TODO: ユーザーメタデータを更新する処理を実装
        Ok(())
    }

    /// コンタクトリストイベントを処理
    async fn handle_contact_list(&self, event: &Event) -> Result<()> {
        info!("Received contact list from {}", event.pubkey);
        // TODO: フォロー関係を更新する処理を実装
        Ok(())
    }

    /// リアクションイベントを処理
    async fn handle_reaction(&self, event: &Event) -> Result<()> {
        info!("Received reaction from {}: {}", event.pubkey, event.content);
        // TODO: リアクションを処理する実装
        Ok(())
    }

    /// イベントを検証
    pub fn verify_event(&self, event: &Event) -> Result<bool> {
        match event.verify() {
            Ok(_) => {
                debug!("Event {} verified successfully", event.id);
                Ok(true)
            }
            Err(e) => {
                error!("Event {} verification failed: {}", event.id, e);
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_handler_creation() {
        let handler = EventHandler::new();
        assert!(handler.event_callbacks.read().await.is_empty());
    }

    #[test]
    fn test_event_verification() {
        let handler = EventHandler::new();
        let keys = Keys::generate();
        let event = EventBuilder::text_note("Test message", [])
            .sign_with_keys(&keys)
            .unwrap();
        
        assert!(handler.verify_event(&event).unwrap());
    }
}
