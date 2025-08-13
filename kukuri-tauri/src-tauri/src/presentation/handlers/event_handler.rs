use crate::application::services::event_service::EventServiceTrait;
use crate::presentation::dto::event::{
    NostrMetadataDto, PublishTextNoteRequest, PublishTopicPostRequest,
    SendReactionRequest, UpdateMetadataRequest, DeleteEventsRequest,
    EventResponse, SubscribeRequest
};
use crate::presentation::dto::Validate;
use crate::shared::error::AppError;
use serde_json::json;
use std::sync::Arc;

pub struct EventHandler {
    event_service: Arc<dyn EventServiceTrait>,
}

impl EventHandler {
    pub fn new(event_service: Arc<dyn EventServiceTrait>) -> Self {
        Self { event_service }
    }

    /// Nostrクライアントを初期化
    pub async fn initialize_nostr(&self) -> Result<serde_json::Value, AppError> {
        self.event_service.initialize().await?;
        Ok(json!({ "success": true }))
    }

    /// テキストノートを投稿
    pub async fn publish_text_note(
        &self,
        request: PublishTextNoteRequest,
    ) -> Result<EventResponse, AppError> {
        request.validate()?;
        
        let event_id = self.event_service
            .publish_text_note(&request.content)
            .await?;
        
        Ok(EventResponse {
            event_id: event_id.to_string(),
            success: true,
            message: Some("Text note published successfully".to_string()),
        })
    }

    /// トピック投稿を作成
    pub async fn publish_topic_post(
        &self,
        request: PublishTopicPostRequest,
    ) -> Result<EventResponse, AppError> {
        request.validate()?;
        
        let event_id = self.event_service
            .publish_topic_post(
                &request.topic_id,
                &request.content,
                request.reply_to.as_deref(),
            )
            .await?;
        
        Ok(EventResponse {
            event_id: event_id.to_string(),
            success: true,
            message: Some("Topic post published successfully".to_string()),
        })
    }

    /// リアクションを送信
    pub async fn send_reaction(
        &self,
        request: SendReactionRequest,
    ) -> Result<EventResponse, AppError> {
        request.validate()?;
        
        let event_id = self.event_service
            .send_reaction(&request.event_id, &request.reaction)
            .await?;
        
        Ok(EventResponse {
            event_id: event_id.to_string(),
            success: true,
            message: Some("Reaction sent successfully".to_string()),
        })
    }

    /// メタデータを更新
    pub async fn update_metadata(
        &self,
        request: UpdateMetadataRequest,
    ) -> Result<EventResponse, AppError> {
        request.validate()?;
        
        let metadata = NostrMetadataDto {
            name: request.metadata.name,
            display_name: request.metadata.display_name,
            about: request.metadata.about,
            picture: request.metadata.picture,
            banner: request.metadata.banner,
            nip05: request.metadata.nip05,
            lud16: request.metadata.lud16,
            website: request.metadata.website,
        };
        
        let event_id = self.event_service
            .update_metadata(metadata)
            .await?;
        
        Ok(EventResponse {
            event_id: event_id.to_string(),
            success: true,
            message: Some("Metadata updated successfully".to_string()),
        })
    }

    /// トピックをサブスクライブ
    pub async fn subscribe_to_topic(
        &self,
        request: SubscribeRequest,
    ) -> Result<serde_json::Value, AppError> {
        request.validate()?;
        
        self.event_service
            .subscribe_to_topic(&request.topic_id)
            .await?;
        
        Ok(json!({ "success": true }))
    }

    /// ユーザーをサブスクライブ
    pub async fn subscribe_to_user(
        &self,
        pubkey: String,
    ) -> Result<serde_json::Value, AppError> {
        if pubkey.is_empty() {
            return Err(AppError::ValidationError("Public key is required".to_string()));
        }
        
        self.event_service
            .subscribe_to_user(&pubkey)
            .await?;
        
        Ok(json!({ "success": true }))
    }

    /// Nostr公開鍵を取得
    pub async fn get_nostr_pubkey(&self) -> Result<serde_json::Value, AppError> {
        let pubkey = self.event_service.get_public_key().await?;
        
        Ok(json!({ 
            "pubkey": pubkey
        }))
    }

    /// イベントを削除
    pub async fn delete_events(
        &self,
        request: DeleteEventsRequest,
    ) -> Result<EventResponse, AppError> {
        request.validate()?;
        
        let event_id = self.event_service
            .delete_events(request.event_ids, request.reason)
            .await?;
        
        Ok(EventResponse {
            event_id: event_id.to_string(),
            success: true,
            message: Some("Events deleted successfully".to_string()),
        })
    }

    /// Nostrクライアントを切断
    pub async fn disconnect_nostr(&self) -> Result<serde_json::Value, AppError> {
        self.event_service.disconnect().await?;
        
        Ok(json!({ "success": true }))
    }
}