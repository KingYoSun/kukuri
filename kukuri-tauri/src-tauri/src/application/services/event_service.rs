use crate::domain::entities::{Event, EventKind};
use crate::infrastructure::database::EventRepository;
use crate::infrastructure::crypto::SignatureService;
use crate::infrastructure::p2p::EventDistributor;
use crate::infrastructure::p2p::event_distributor::DistributionStrategy;
use crate::presentation::dto::event::NostrMetadataDto;
use crate::shared::error::AppError;
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
}

pub struct EventService {
    repository: Arc<dyn EventRepository>,
    signature_service: Arc<dyn SignatureService>,
    distributor: Arc<dyn EventDistributor>,
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
        }
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
        // Nostrクライアントの初期化処理
        // 実際の初期化はEventManagerで行われている場合はチェックのみ
        Ok(())
    }
    
    async fn publish_text_note(&self, content: &str) -> Result<EventId, AppError> {
        // TODO: 実際のEventManagerを使用して実装
        // 仮の実装
        let event_id = EventId::from_hex("0000000000000000000000000000000000000000000000000000000000000001")
            .map_err(|e| AppError::NostrError(e.to_string()))?;
        Ok(event_id)
    }
    
    async fn publish_topic_post(
        &self,
        _topic_id: &str,
        _content: &str,
        _reply_to: Option<&str>,
    ) -> Result<EventId, AppError> {
        // TODO: 実際のEventManagerを使用して実装
        let event_id = EventId::from_hex("0000000000000000000000000000000000000000000000000000000000000002")
            .map_err(|e| AppError::NostrError(e.to_string()))?;
        Ok(event_id)
    }
    
    async fn send_reaction(&self, _event_id: &str, _reaction: &str) -> Result<EventId, AppError> {
        // TODO: 実際のEventManagerを使用して実装
        let event_id = EventId::from_hex("0000000000000000000000000000000000000000000000000000000000000003")
            .map_err(|e| AppError::NostrError(e.to_string()))?;
        Ok(event_id)
    }
    
    async fn update_metadata(&self, _metadata: NostrMetadataDto) -> Result<EventId, AppError> {
        // TODO: 実際のEventManagerを使用して実装
        let event_id = EventId::from_hex("0000000000000000000000000000000000000000000000000000000000000004")
            .map_err(|e| AppError::NostrError(e.to_string()))?;
        Ok(event_id)
    }
    
    async fn subscribe_to_topic(&self, _topic_id: &str) -> Result<(), AppError> {
        // TODO: 実際のEventManagerを使用して実装
        Ok(())
    }
    
    async fn subscribe_to_user(&self, _pubkey: &str) -> Result<(), AppError> {
        // TODO: 実際のEventManagerを使用して実装
        Ok(())
    }
    
    async fn get_public_key(&self) -> Result<Option<String>, AppError> {
        // TODO: 実際のEventManagerを使用して実装
        Ok(None)
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
        // TODO: 実際のEventManagerを使用して実装
        Ok(())
    }
}