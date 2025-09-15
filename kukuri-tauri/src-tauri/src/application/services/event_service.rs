use crate::domain::entities::{Event, EventKind};
use crate::infrastructure::database::EventRepository;
use crate::infrastructure::crypto::SignatureService;
use crate::infrastructure::p2p::EventDistributor;
use crate::infrastructure::p2p::event_distributor::DistributionStrategy;
use crate::presentation::dto::event::NostrMetadataDto;
use crate::shared::error::AppError;
use crate::modules::event::manager::EventManager;
use std::sync::Arc;
use async_trait::async_trait;
use nostr_sdk::prelude::*;

/// Nostrイベントサービスのトレイト
#[async_trait]
pub trait EventServiceTrait: Send + Sync {
    /// Nostrクライアントを初期化
    async fn initialize(&self) -> Result<(), AppError>;
    
    /// テキストノートを投稿
    async fn publish_text_note(&self, content: &str) -> Result<EventId, AppError>;
    
    /// トピック投稿を作成
    async fn publish_topic_post(
        &self,
        topic_id: &str,
        content: &str,
        reply_to: Option<&str>,
    ) -> Result<EventId, AppError>;
    
    /// リアクションを送信
    async fn send_reaction(&self, event_id: &str, reaction: &str) -> Result<EventId, AppError>;
    
    /// メタデータを更新
    async fn update_metadata(&self, metadata: NostrMetadataDto) -> Result<EventId, AppError>;
    
    /// トピックをサブスクライブ
    async fn subscribe_to_topic(&self, topic_id: &str) -> Result<(), AppError>;
    
    /// ユーザーをサブスクライブ
    async fn subscribe_to_user(&self, pubkey: &str) -> Result<(), AppError>;
    
    /// Nostr公開鍵を取得
    async fn get_public_key(&self) -> Result<Option<String>, AppError>;
    
    /// イベントを削除
    async fn delete_events(
        &self,
        event_ids: Vec<String>,
        reason: Option<String>,
    ) -> Result<EventId, AppError>;
    
    /// Nostrクライアントを切断
    async fn disconnect(&self) -> Result<(), AppError>;

    /// 既定のP2P配信トピックを設定
    async fn set_default_p2p_topic(&self, topic_id: &str) -> Result<(), AppError>;
}

pub struct EventService {
    repository: Arc<dyn EventRepository>,
    signature_service: Arc<dyn SignatureService>,
    distributor: Arc<dyn EventDistributor>,
    event_manager: Option<Arc<EventManager>>,
}

impl EventService {
    pub fn new(
        repository: Arc<dyn EventRepository>,
        signature_service: Arc<dyn SignatureService>,
        distributor: Arc<dyn EventDistributor>,
    ) -> Self {
        Self {
            repository,
            signature_service,
            distributor,
            event_manager: None,
        }
    }
    
    /// EventManagerを設定する
    pub fn set_event_manager(&mut self, event_manager: Arc<EventManager>) {
        self.event_manager = Some(event_manager);
    }

    pub async fn create_event(&self, kind: u32, content: String, pubkey: String, private_key: &str) -> Result<Event, AppError> {
        let mut event = Event::new(kind, content, pubkey);
        
        // Sign the event
        self.signature_service.sign_event(&mut event, private_key).await?;
        
        // Save to database
        self.repository.create_event(&event).await?;
        
        // Distribute
        self.distributor.distribute(&event, DistributionStrategy::Hybrid).await?;
        
        Ok(event)
    }

    pub async fn process_received_event(&self, event: Event) -> Result<(), AppError> {
        // Verify signature
        if !self.signature_service.verify_event(&event).await? {
            return Err("Invalid event signature".into());
        }
        
        // Save to database
        self.repository.create_event(&event).await?;
        
        // Process based on event kind
        match EventKind::from_u32(event.kind) {
            Some(EventKind::TextNote) => {
                // TODO: Convert to Post and save
            }
            Some(EventKind::Metadata) => {
                // TODO: Update user metadata
            }
            Some(EventKind::Reaction) => {
                // TODO: Process reaction
            }
            Some(EventKind::Repost) => {
                // TODO: Process repost
            }
            _ => {
                // Unknown or unhandled event kind
            }
        }
        
        Ok(())
    }

    pub async fn get_event(&self, id: &str) -> Result<Option<Event>, AppError> {
        self.repository.get_event(id).await
    }

    pub async fn get_events_by_kind(&self, kind: u32, limit: usize) -> Result<Vec<Event>, AppError> {
        self.repository.get_events_by_kind(kind, limit).await
    }

    pub async fn get_events_by_author(&self, pubkey: &str, limit: usize) -> Result<Vec<Event>, AppError> {
        self.repository.get_events_by_author(pubkey, limit).await
    }

    pub async fn delete_event(&self, id: &str, pubkey: String, private_key: &str) -> Result<(), AppError> {
        // Create deletion event (Kind 5)
        let mut deletion_event = Event::new(EventKind::EventDeletion.as_u32(), String::new(), pubkey);
        deletion_event.add_e_tag(id.to_string());
        
        self.signature_service.sign_event(&mut deletion_event, private_key).await?;
        self.repository.create_event(&deletion_event).await?;
        self.distributor.distribute(&deletion_event, DistributionStrategy::Hybrid).await?;
        
        // Mark original event as deleted in database
        self.repository.delete_event(id).await
    }

    pub async fn sync_pending_events(&self) -> Result<u32, AppError> {
        let unsync_events = self.repository.get_unsync_events().await?;
        let mut synced_count = 0;
        
        for event in unsync_events {
            self.distributor.distribute(&event, DistributionStrategy::Hybrid).await?;
            self.repository.mark_event_synced(&event.id).await?;
            synced_count += 1;
        }
        
        Ok(synced_count)
    }
}

#[async_trait]
impl EventServiceTrait for EventService {
    async fn initialize(&self) -> Result<(), AppError> {
        // EventManagerが設定されていることを確認
        if self.event_manager.is_none() {
            return Err(AppError::ConfigurationError("EventManager not set".to_string()));
        }
        // 実際の初期化はEventManagerで既に行われているため、ここでは確認のみ
        Ok(())
    }
    
    async fn publish_text_note(&self, content: &str) -> Result<EventId, AppError> {
        let event_manager = self.event_manager.as_ref()
            .ok_or_else(|| AppError::ConfigurationError("EventManager not set".to_string()))?;
        
        event_manager.publish_text_note(content)
            .await
            .map_err(|e| AppError::NostrError(e.to_string()))
    }
    
    async fn publish_topic_post(
        &self,
        topic_id: &str,
        content: &str,
        reply_to: Option<&str>,
    ) -> Result<EventId, AppError> {
        let event_manager = self.event_manager.as_ref()
            .ok_or_else(|| AppError::ConfigurationError("EventManager not set".to_string()))?;
        
        let reply_to_id = if let Some(reply_id) = reply_to {
            Some(EventId::from_hex(reply_id).map_err(|e| AppError::NostrError(e.to_string()))?)
        } else {
            None
        };
        
        event_manager.publish_topic_post(topic_id, content, reply_to_id)
            .await
            .map_err(|e| AppError::NostrError(e.to_string()))
    }
    
    async fn send_reaction(&self, event_id: &str, reaction: &str) -> Result<EventId, AppError> {
        let event_manager = self.event_manager.as_ref()
            .ok_or_else(|| AppError::ConfigurationError("EventManager not set".to_string()))?;
        
        let event_id = EventId::from_hex(event_id)
            .map_err(|e| AppError::NostrError(e.to_string()))?;
        
        event_manager.send_reaction(&event_id, reaction)
            .await
            .map_err(|e| AppError::NostrError(e.to_string()))
    }
    
    async fn update_metadata(&self, metadata: NostrMetadataDto) -> Result<EventId, AppError> {
        let event_manager = self.event_manager.as_ref()
            .ok_or_else(|| AppError::ConfigurationError("EventManager not set".to_string()))?;
        
        // DTOからnostr_sdkのMetadataに変換
        let mut nostr_metadata = Metadata::new();
        if let Some(name) = metadata.name {
            nostr_metadata = nostr_metadata.name(name);
        }
        if let Some(display_name) = metadata.display_name {
            nostr_metadata = nostr_metadata.display_name(display_name);
        }
        if let Some(about) = metadata.about {
            nostr_metadata = nostr_metadata.about(about);
        }
        if let Some(picture) = metadata.picture {
            if let Ok(pic_url) = picture.parse() {
                nostr_metadata = nostr_metadata.picture(pic_url);
            }
        }
        if let Some(website) = metadata.website {
            nostr_metadata = nostr_metadata.website(website.parse().map_err(|_| AppError::ValidationError("Invalid website URL".to_string()))?);
        }
        
        event_manager.update_metadata(nostr_metadata)
            .await
            .map_err(|e| AppError::NostrError(e.to_string()))
    }
    
    async fn subscribe_to_topic(&self, topic_id: &str) -> Result<(), AppError> {
        let event_manager = self.event_manager.as_ref()
            .ok_or_else(|| AppError::ConfigurationError("EventManager not set".to_string()))?;
        
        event_manager.subscribe_to_topic(topic_id)
            .await
            .map_err(|e| AppError::NostrError(e.to_string()))
    }
    
    async fn subscribe_to_user(&self, pubkey: &str) -> Result<(), AppError> {
        let event_manager = self.event_manager.as_ref()
            .ok_or_else(|| AppError::ConfigurationError("EventManager not set".to_string()))?;
        
        let public_key = PublicKey::from_hex(pubkey)
            .map_err(|e| AppError::NostrError(e.to_string()))?;
        
        event_manager.subscribe_to_user(public_key)
            .await
            .map_err(|e| AppError::NostrError(e.to_string()))
    }
    
    async fn get_public_key(&self) -> Result<Option<String>, AppError> {
        let event_manager = self.event_manager.as_ref()
            .ok_or_else(|| AppError::ConfigurationError("EventManager not set".to_string()))?;
        
        let public_key = event_manager.get_public_key().await;
        Ok(public_key.map(|pk| pk.to_hex()))
    }
    
    async fn delete_events(
        &self,
        event_ids: Vec<String>,
        _reason: Option<String>,
    ) -> Result<EventId, AppError> {
        // TODO: 実際のEventManagerを使用して実装
        if event_ids.is_empty() {
            return Err(AppError::ValidationError("No event IDs provided".to_string()));
        }
        let event_id = EventId::from_hex("0000000000000000000000000000000000000000000000000000000000000005")
            .map_err(|e| AppError::NostrError(e.to_string()))?;
        Ok(event_id)
    }
    
    async fn disconnect(&self) -> Result<(), AppError> {
        let event_manager = self.event_manager.as_ref()
            .ok_or_else(|| AppError::ConfigurationError("EventManager not set".to_string()))?;
        
        event_manager.disconnect()
            .await
            .map_err(|e| AppError::NostrError(e.to_string()))
    }

    async fn set_default_p2p_topic(&self, topic_id: &str) -> Result<(), AppError> {
        let event_manager = self.event_manager.as_ref()
            .ok_or_else(|| AppError::ConfigurationError("EventManager not set".to_string()))?;
        if topic_id.is_empty() {
            return Err(AppError::ValidationError("Topic ID is required".to_string()));
        }
        event_manager
            .set_default_p2p_topic_id(topic_id.to_string())
            .await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::database::EventRepository;
    use crate::infrastructure::crypto::SignatureService;
    use crate::infrastructure::p2p::{EventDistributor, event_distributor::DistributionStrategy};
    use async_trait::async_trait;
    use mockall::{mock, predicate::*};

    // EventRepositoryのモック
    mock! {
        pub EventRepo {}
        
        #[async_trait]
        impl EventRepository for EventRepo {
            async fn create_event(&self, event: &Event) -> Result<(), AppError>;
            async fn get_event(&self, id: &str) -> Result<Option<Event>, AppError>;
            async fn get_events_by_kind(&self, kind: u32, limit: usize) -> Result<Vec<Event>, AppError>;
            async fn get_events_by_author(&self, pubkey: &str, limit: usize) -> Result<Vec<Event>, AppError>;
            async fn delete_event(&self, id: &str) -> Result<(), AppError>;
            async fn get_unsync_events(&self) -> Result<Vec<Event>, AppError>;
            async fn mark_event_synced(&self, id: &str) -> Result<(), AppError>;
            async fn add_event_topic(&self, event_id: &str, topic_id: &str) -> Result<(), AppError>;
            async fn get_event_topics(&self, event_id: &str) -> Result<Vec<String>, AppError>;
        }
    }

    // SignatureServiceのモック
    mock! {
        pub SignatureServ {}
        
        #[async_trait]
        impl SignatureService for SignatureServ {
            async fn sign_event(&self, event: &mut Event, private_key: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
            async fn verify_event(&self, event: &Event) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;
            async fn sign_message(&self, message: &str, private_key: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;
            async fn verify_message(&self, message: &str, signature: &str, public_key: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;
        }
    }

    // EventDistributorのモック
    mock! {
        pub EventDist {}
        
        #[async_trait]
        impl EventDistributor for EventDist {
            async fn distribute(&self, event: &Event, strategy: DistributionStrategy) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
            async fn receive(&self) -> Result<Option<Event>, Box<dyn std::error::Error + Send + Sync>>;
            async fn set_strategy(&self, strategy: DistributionStrategy);
            async fn get_pending_events(&self) -> Result<Vec<Event>, Box<dyn std::error::Error + Send + Sync>>;
            async fn retry_failed(&self) -> Result<u32, Box<dyn std::error::Error + Send + Sync>>;
        }
    }

    fn create_test_event() -> Event {
        Event::new(1, "Test content".to_string(), "test_pubkey".to_string())
    }

    #[tokio::test]
    async fn test_create_event_success() {
        // モックの準備
        let mut mock_repo = MockEventRepo::new();
        mock_repo
            .expect_create_event()
            .times(1)
            .returning(|_| Ok(()));

        let mut mock_signature = MockSignatureServ::new();
        mock_signature
            .expect_sign_event()
            .times(1)
            .returning(|_, _| Ok(()));

        let mut mock_distributor = MockEventDist::new();
        mock_distributor
            .expect_distribute()
            .times(1)
            .returning(|_, _| Ok(()));

        // EventServiceを作成
        let service = EventService::new(
            Arc::new(mock_repo),
            Arc::new(mock_signature),
            Arc::new(mock_distributor),
        );

        // テスト実行
        let result = service.create_event(
            1,
            "Test content".to_string(),
            "test_pubkey".to_string(),
            "test_private_key",
        ).await;

        // 検証
        assert!(result.is_ok());
        let event = result.unwrap();
        assert_eq!(event.content, "Test content");
        assert_eq!(event.pubkey, "test_pubkey");
    }

    #[tokio::test]
    async fn test_process_received_event_valid_signature() {
        // モックの準備
        let mut mock_repo = MockEventRepo::new();
        mock_repo
            .expect_create_event()
            .times(1)
            .returning(|_| Ok(()));

        let mut mock_signature = MockSignatureServ::new();
        mock_signature
            .expect_verify_event()
            .times(1)
            .returning(|_| Ok(true));

        let mock_distributor = MockEventDist::new();

        let service = EventService::new(
            Arc::new(mock_repo),
            Arc::new(mock_signature),
            Arc::new(mock_distributor),
        );

        // テストイベント作成
        let event = create_test_event();

        // テスト実行
        let result = service.process_received_event(event).await;

        // 検証
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_process_received_event_invalid_signature() {
        // モックの準備
        let mock_repo = MockEventRepo::new();

        let mut mock_signature = MockSignatureServ::new();
        mock_signature
            .expect_verify_event()
            .times(1)
            .returning(|_| Ok(false));

        let mock_distributor = MockEventDist::new();

        let service = EventService::new(
            Arc::new(mock_repo),
            Arc::new(mock_signature),
            Arc::new(mock_distributor),
        );

        // テストイベント作成
        let event = create_test_event();

        // テスト実行
        let result = service.process_received_event(event).await;

        // 検証
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid event signature"));
    }

    #[tokio::test]
    async fn test_get_event() {
        // モックの準備
        let mut mock_repo = MockEventRepo::new();
        let test_event = create_test_event();
        let test_event_clone = test_event.clone();
        
        mock_repo
            .expect_get_event()
            .with(eq("test_id"))
            .times(1)
            .returning(move |_| Ok(Some(test_event_clone.clone())));

        let mock_signature = MockSignatureServ::new();
        let mock_distributor = MockEventDist::new();

        let service = EventService::new(
            Arc::new(mock_repo),
            Arc::new(mock_signature),
            Arc::new(mock_distributor),
        );

        // テスト実行
        let result = service.get_event("test_id").await;

        // 検証
        assert!(result.is_ok());
        let event_opt = result.unwrap();
        assert!(event_opt.is_some());
        let event = event_opt.unwrap();
        assert_eq!(event.content, "Test content");
    }

    #[tokio::test]
    async fn test_get_events_by_kind() {
        // モックの準備
        let mut mock_repo = MockEventRepo::new();
        let test_events = vec![create_test_event(), create_test_event()];
        let test_events_clone = test_events.clone();
        
        mock_repo
            .expect_get_events_by_kind()
            .with(eq(1u32), eq(10usize))
            .times(1)
            .returning(move |_, _| Ok(test_events_clone.clone()));

        let mock_signature = MockSignatureServ::new();
        let mock_distributor = MockEventDist::new();

        let service = EventService::new(
            Arc::new(mock_repo),
            Arc::new(mock_signature),
            Arc::new(mock_distributor),
        );

        // テスト実行
        let result = service.get_events_by_kind(1, 10).await;

        // 検証
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 2);
    }

    #[tokio::test]
    async fn test_get_events_by_author() {
        // モックの準備
        let mut mock_repo = MockEventRepo::new();
        let test_events = vec![create_test_event()];
        let test_events_clone = test_events.clone();
        
        mock_repo
            .expect_get_events_by_author()
            .with(eq("test_pubkey"), eq(5usize))
            .times(1)
            .returning(move |_, _| Ok(test_events_clone.clone()));

        let mock_signature = MockSignatureServ::new();
        let mock_distributor = MockEventDist::new();

        let service = EventService::new(
            Arc::new(mock_repo),
            Arc::new(mock_signature),
            Arc::new(mock_distributor),
        );

        // テスト実行
        let result = service.get_events_by_author("test_pubkey", 5).await;

        // 検証
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].pubkey, "test_pubkey");
    }

    #[tokio::test]
    async fn test_delete_event() {
        // モックの準備
        let mut mock_repo = MockEventRepo::new();
        mock_repo
            .expect_create_event()
            .times(1)
            .returning(|_| Ok(()));
        mock_repo
            .expect_delete_event()
            .with(eq("event_to_delete"))
            .times(1)
            .returning(|_| Ok(()));

        let mut mock_signature = MockSignatureServ::new();
        mock_signature
            .expect_sign_event()
            .times(1)
            .returning(|_, _| Ok(()));

        let mut mock_distributor = MockEventDist::new();
        mock_distributor
            .expect_distribute()
            .times(1)
            .returning(|_, _| Ok(()));

        let service = EventService::new(
            Arc::new(mock_repo),
            Arc::new(mock_signature),
            Arc::new(mock_distributor),
        );

        // テスト実行
        let result = service.delete_event("event_to_delete", "test_pubkey".to_string(), "test_private_key").await;

        // 検証
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_sync_pending_events() {
        // モックの準備
        let mut mock_repo = MockEventRepo::new();
        let test_events = vec![create_test_event(), create_test_event()];
        let test_events_clone = test_events.clone();
        
        mock_repo
            .expect_get_unsync_events()
            .times(1)
            .returning(move || Ok(test_events_clone.clone()));
        
        mock_repo
            .expect_mark_event_synced()
            .times(2)
            .returning(|_| Ok(()));

        let mock_signature = MockSignatureServ::new();
        
        let mut mock_distributor = MockEventDist::new();
        mock_distributor
            .expect_distribute()
            .times(2)
            .returning(|_, _| Ok(()));

        let service = EventService::new(
            Arc::new(mock_repo),
            Arc::new(mock_signature),
            Arc::new(mock_distributor),
        );

        // テスト実行
        let result = service.sync_pending_events().await;

        // 検証
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2); // 2つのイベントが同期された
    }
}
