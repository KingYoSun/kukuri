use crate::{
    application::services::TopicService,
    presentation::dto::{
        topic_dto::{
            CreateTopicRequest, GetTopicStatsRequest, JoinTopicRequest, TopicResponse,
            TopicStatsResponse, UpdateTopicRequest,
        },
        ApiResponse, Validate,
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

        let topic = self
            .topic_service
            .create_topic(&request.name, &request.description, request.image_url)
            .await?;

        Ok(TopicResponse {
            id: topic.id.to_string(),
            name: topic.name,
            description: topic.description,
            image_url: topic.image_url,
            member_count: 0,
            post_count: 0,
            is_joined: false,
            created_at: topic.created_at.timestamp(),
            updated_at: topic.updated_at.timestamp(),
        })
    }

    pub async fn get_all_topics(&self) -> Result<Vec<TopicResponse>, AppError> {
        let topics = self.topic_service.get_all_topics().await?;

        Ok(topics
            .into_iter()
            .map(|topic| TopicResponse {
                id: topic.id.to_string(),
                name: topic.name,
                description: topic.description,
                image_url: topic.image_url,
                member_count: 0,
                post_count: 0,
                is_joined: false,
                created_at: topic.created_at.timestamp(),
                updated_at: topic.updated_at.timestamp(),
            })
            .collect())
    }

    pub async fn join_topic(&self, request: JoinTopicRequest, user_pubkey: &str) -> Result<(), AppError> {
        request.validate()
            .map_err(|e| AppError::InvalidInput(e))?;

        self.topic_service
            .join_topic(&request.topic_id, user_pubkey)
            .await
    }

    pub async fn leave_topic(&self, request: JoinTopicRequest, user_pubkey: &str) -> Result<(), AppError> {
        request.validate()
            .map_err(|e| AppError::InvalidInput(e))?;

        self.topic_service
            .leave_topic(&request.topic_id, user_pubkey)
            .await
    }

    pub async fn get_topic_stats(&self, request: GetTopicStatsRequest) -> Result<TopicStatsResponse, AppError> {
        request.validate()
            .map_err(|e| AppError::InvalidInput(e))?;

        let stats = self.topic_service
            .get_topic_stats(&request.topic_id)
            .await?;

        Ok(TopicStatsResponse {
            topic_id: request.topic_id,
            member_count: stats.0,
            post_count: stats.1,
            active_users_24h: 0,
            trending_score: 0.0,
        })
    }
}