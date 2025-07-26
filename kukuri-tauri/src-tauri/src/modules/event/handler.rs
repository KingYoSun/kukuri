
use nostr_sdk::prelude::*;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, error};

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
    use std::sync::atomic::{AtomicBool, Ordering};

    #[tokio::test]
    async fn test_event_handler_creation() {
        let handler = EventHandler::new();
        assert!(handler.event_callbacks.read().await.is_empty());
    }

    #[test]
    fn test_event_verification() {
        let handler = EventHandler::new();
        let keys = Keys::generate();
        let event = EventBuilder::text_note("Test message")
            .sign_with_keys(&keys)
            .unwrap();
        
        assert!(handler.verify_event(&event).unwrap());
    }

    #[test]
    fn test_event_verification_invalid() {
        let handler = EventHandler::new();
        let keys = Keys::generate();
        let mut event = EventBuilder::text_note("Test message")
            .sign_with_keys(&keys)
            .unwrap();
        
        // イベントを改竄して検証が失敗することを確認
        event.content = "Modified content".to_string();
        
        assert!(!handler.verify_event(&event).unwrap());
    }

    #[tokio::test]
    async fn test_add_callback() {
        let handler = EventHandler::new();
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();
        
        // コールバックを追加
        handler.add_callback(move |_event| {
            called_clone.store(true, Ordering::Relaxed);
        }).await;
        
        // コールバックが追加されたことを確認
        assert_eq!(handler.event_callbacks.read().await.len(), 1);
    }

    #[tokio::test]
    async fn test_handle_event_callbacks() {
        let handler = EventHandler::new();
        let counter = Arc::new(RwLock::new(0));
        let counter_clone = counter.clone();
        
        // コールバックを追加
        handler.add_callback(move |_event| {
            let counter = counter_clone.clone();
            tokio::spawn(async move {
                let mut count = counter.write().await;
                *count += 1;
            });
        }).await;
        
        // イベントを作成して処理
        let keys = Keys::generate();
        let event = EventBuilder::text_note("Test message")
            .sign_with_keys(&keys)
            .unwrap();
        
        handler.handle_event(event).await.unwrap();
        
        // 少し待機してからカウンターを確認
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        assert_eq!(*counter.read().await, 1);
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
        
        let metadata = Metadata::new()
            .name("Test User")
            .about("Test about");
        
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

    #[tokio::test]
    async fn test_handle_multiple_callbacks() {
        let handler = EventHandler::new();
        let counter1 = Arc::new(AtomicBool::new(false));
        let counter2 = Arc::new(AtomicBool::new(false));
        
        let counter1_clone = counter1.clone();
        let counter2_clone = counter2.clone();
        
        // 複数のコールバックを追加
        handler.add_callback(move |_| {
            counter1_clone.store(true, Ordering::Relaxed);
        }).await;
        
        handler.add_callback(move |_| {
            counter2_clone.store(true, Ordering::Relaxed);
        }).await;
        
        // イベントを処理
        let keys = Keys::generate();
        let event = EventBuilder::text_note("Test")
            .sign_with_keys(&keys)
            .unwrap();
        
        handler.handle_event(event).await.unwrap();
        
        // 両方のコールバックが呼ばれたことを確認
        assert!(counter1.load(Ordering::Relaxed));
        assert!(counter2.load(Ordering::Relaxed));
    }
}
