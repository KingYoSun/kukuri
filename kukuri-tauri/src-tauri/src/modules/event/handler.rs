use anyhow::Result;
use nostr_sdk::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// イベントコールバックの型エイリアス
type EventCallback = Box<dyn Fn(Event) + Send + Sync>;

/// Nostrイベントハンドラー
pub struct EventHandler {
    event_callbacks: Arc<RwLock<Vec<EventCallback>>>,
}

impl EventHandler {
    /// 新しいEventHandlerインスタンスを作成
    pub fn new() -> Self {
        Self {
            event_callbacks: Arc::new(RwLock::new(Vec::new())),
        }
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
        info!(
            "Received text note from {}: {}",
            event.pubkey, event.content
        );
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


}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_handler_creation() {
        let handler = EventHandler::new();
        assert!(handler.event_callbacks.read().await.is_empty());
    }





    #[tokio::test]
    async fn test_handle_text_note() {
        let handler = EventHandler::new();
        let keys = Keys::generate();

        let event = EventBuilder::text_note("Test text note")
            .sign_with_keys(&keys)
            .unwrap();

        // テキストノートの処理が正常に完了することを確認
        assert!(handler.handle_event(event).await.is_ok());
    }

    #[tokio::test]
    async fn test_handle_metadata() {
        let handler = EventHandler::new();
        let keys = Keys::generate();

        let metadata = Metadata::new().name("Test User").about("Test about");

        let event = EventBuilder::metadata(&metadata)
            .sign_with_keys(&keys)
            .unwrap();

        // メタデータイベントの処理が正常に完了することを確認
        assert!(handler.handle_event(event).await.is_ok());
    }

    #[tokio::test]
    async fn test_handle_reaction() {
        let handler = EventHandler::new();
        let keys = Keys::generate();
        let _target_event_id = EventId::from_slice(&[1; 32]).unwrap();

        // リアクション用の疑似イベントを作成
        let target_event = EventBuilder::text_note("dummy")
            .sign_with_keys(&keys)
            .unwrap();
        let event = EventBuilder::reaction(&target_event, "+")
            .sign_with_keys(&keys)
            .unwrap();

        // リアクションイベントの処理が正常に完了することを確認
        assert!(handler.handle_event(event).await.is_ok());
    }


}
