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
            .create_topic(request.name, Some(request.description))
            .await?;

        Ok(TopicResponse {
            id: topic.id.to_string(),
            name: topic.name,
            description: topic.description.unwrap_or_default(),
            image_url: topic.image_url,
            member_count: 0,
            post_count: 0,
            is_joined: false,
            created_at: topic.created_at.timestamp_millis(),
            updated_at: topic.updated_at.timestamp_millis(),
        })
    }

    pub async fn get_all_topics(&self) -> Result<Vec<TopicResponse>, AppError> {
        let topics = self.topic_service.get_all_topics().await?;

        Ok(topics
            .into_iter()
            .map(|topic| TopicResponse {
                id: topic.id.to_string(),
                name: topic.name,
                description: topic.description.unwrap_or_default(),
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
            .join_topic(&request.topic_id)
            .await?;
        Ok(())
    }

    pub async fn leave_topic(&self, request: JoinTopicRequest, user_pubkey: &str) -> Result<(), AppError> {
        request.validate()
            .map_err(|e| AppError::InvalidInput(e))?;

        self.topic_service
            .leave_topic(&request.topic_id)
            .await?;
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_topic_request_validation() {
        // 空の名前はエラー
        let invalid_request = CreateTopicRequest {
            name: "".to_string(),
            description: "Test description".to_string(),
        };
        assert!(invalid_request.validate().is_err());

        // 有効なリクエスト
        let valid_request = CreateTopicRequest {
            name: "test-topic".to_string(),
            description: "Test description".to_string(),
        };
        assert!(valid_request.validate().is_ok());

        // 長すぎる名前はエラー（100文字以上）
        let long_name = "a".repeat(101);
        let invalid_long_request = CreateTopicRequest {
            name: long_name,
            description: "Test description".to_string(),
        };
        assert!(invalid_long_request.validate().is_err());
    }

    #[test]
    fn test_join_topic_request_validation() {
        // 空のトピックIDはエラー
        let invalid_request = JoinTopicRequest {
            topic_id: "".to_string(),
        };
        assert!(invalid_request.validate().is_err());

        // 有効なリクエスト
        let valid_request = JoinTopicRequest {
            topic_id: "topic123".to_string(),
        };
        assert!(valid_request.validate().is_ok());
    }

    #[test]
    fn test_get_topic_stats_request_validation() {
        // 空のトピックIDはエラー
        let invalid_request = GetTopicStatsRequest {
            topic_id: "".to_string(),
        };
        assert!(invalid_request.validate().is_err());

        // 有効なリクエスト
        let valid_request = GetTopicStatsRequest {
            topic_id: "topic123".to_string(),
        };
        assert!(valid_request.validate().is_ok());
    }

    #[test]
    fn test_topic_response_creation() {
        use crate::domain::entities::Topic;
        use chrono::Utc;

        let topic = Topic {
            id: "topic123".to_string(),
            name: "Test Topic".to_string(),
            description: Some("A test topic".to_string()),
            image_url: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let response = TopicResponse {
            id: topic.id.clone(),
            name: topic.name.clone(),
            description: topic.description.clone().unwrap_or_default(),
            image_url: topic.image_url.clone(),
            member_count: 10,
            post_count: 50,
            is_joined: true,
            created_at: topic.created_at.timestamp_millis(),
            updated_at: topic.updated_at.timestamp_millis(),
        };

        assert_eq!(response.id, "topic123");
        assert_eq!(response.name, "Test Topic");
        assert_eq!(response.description, "A test topic");
        assert_eq!(response.member_count, 10);
        assert_eq!(response.post_count, 50);
        assert!(response.is_joined);
    }
}