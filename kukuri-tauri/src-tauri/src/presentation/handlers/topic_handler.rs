use crate::{
    application::services::TopicService,
    presentation::dto::{
        topic_dto::{
            CreateTopicRequest, GetTopicStatsRequest, JoinTopicRequest,
            TopicResponse, TopicStatsResponse,
        },
        Validate,
    },
    shared::error::AppError,
};
use std::sync::Arc;

pub struct TopicHandler {
    topic_service: Arc<TopicService>,
}

impl TopicHandler {
    pub fn new(topic_service: Arc<TopicService>) -> Self {
        Self { topic_service }
    }

    pub async fn create_topic(&self, request: CreateTopicRequest) -> Result<TopicResponse, AppError> {
        request.validate()
            .map_err(|e| AppError::InvalidInput(e))?;
        
        let topic = self.topic_service.create_topic(request.name, Some(request.description)).await?;
        
        Ok(TopicResponse {
            id: topic.id.to_string(),
            name: topic.name,
            description: topic.description.unwrap_or_default(),
            image_url: topic.image_url,
            member_count: topic.member_count,
            post_count: topic.post_count,
            is_joined: topic.is_joined,
            created_at: topic.created_at.timestamp(),
            updated_at: topic.updated_at.timestamp(),
        })
    }
    
    pub async fn get_topic(&self, id: &str) -> Result<Option<TopicResponse>, AppError> {
        let topic = self.topic_service.get_topic(id).await?;
        
        Ok(topic.map(|t| TopicResponse {
            id: t.id.to_string(),
            name: t.name,
            description: t.description.unwrap_or_default(),
            image_url: t.image_url,
            member_count: t.member_count,
            post_count: t.post_count,
            is_joined: t.is_joined,
            created_at: t.created_at.timestamp(),
            updated_at: t.updated_at.timestamp(),
        }))
    }
    
    pub async fn get_all_topics(&self) -> Result<Vec<TopicResponse>, AppError> {
        let topics = self.topic_service.get_all_topics().await?;
        
        Ok(topics.into_iter().map(|t| TopicResponse {
            id: t.id.to_string(),
            name: t.name,
            description: t.description.unwrap_or_default(),
            image_url: t.image_url,
            member_count: t.member_count,
            post_count: t.post_count,
            is_joined: t.is_joined,
            created_at: t.created_at.timestamp(),
            updated_at: t.updated_at.timestamp(),
        }).collect())
    }
    
    pub async fn get_joined_topics(&self) -> Result<Vec<TopicResponse>, AppError> {
        let topics = self.topic_service.get_joined_topics().await?;
        
        Ok(topics.into_iter().map(|t| TopicResponse {
            id: t.id.to_string(),
            name: t.name,
            description: t.description.unwrap_or_default(),
            image_url: t.image_url,
            member_count: t.member_count,
            post_count: t.post_count,
            is_joined: true,
            created_at: t.created_at.timestamp(),
            updated_at: t.updated_at.timestamp(),
        }).collect())
    }
    
    pub async fn join_topic(&self, request: JoinTopicRequest, user_pubkey: &str) -> Result<(), AppError> {
        request.validate()
            .map_err(|e| AppError::InvalidInput(e))?;
        
        // user_pubkeyはログ用に保持（将来の実装用）
        let _ = user_pubkey;
        
        self.topic_service.join_topic(&request.topic_id).await?;
        Ok(())
    }
    
    pub async fn leave_topic(&self, request: JoinTopicRequest, user_pubkey: &str) -> Result<(), AppError> {
        request.validate()
            .map_err(|e| AppError::InvalidInput(e))?;
        
        // user_pubkeyはログ用に保持（将来の実装用）
        let _ = user_pubkey;
        
        self.topic_service.leave_topic(&request.topic_id).await?;
        Ok(())
    }
    
    pub async fn get_topic_stats(&self, request: GetTopicStatsRequest) -> Result<TopicStatsResponse, AppError> {
        request.validate()
            .map_err(|e| AppError::InvalidInput(e))?;
        
        // 統計情報の仮実装
        Ok(TopicStatsResponse {
            topic_id: request.topic_id,
            member_count: 0,
            post_count: 0,
            active_users_24h: 0,
            trending_score: 0.0,
        })
    }
}