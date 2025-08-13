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
}