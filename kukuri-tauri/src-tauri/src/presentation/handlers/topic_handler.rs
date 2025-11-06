use crate::{
    application::services::TopicService,
    presentation::dto::{
        Validate,
        topic_dto::{
            CreateTopicRequest, DeleteTopicRequest, GetTopicStatsRequest, JoinTopicRequest,
            ListTrendingTopicsRequest, ListTrendingTopicsResponse, TopicResponse,
            TopicStatsResponse, TrendingTopicDto, UpdateTopicRequest,
        },
    },
    shared::error::AppError,
};
use chrono::Utc;
use std::sync::Arc;

pub struct TopicHandler {
    topic_service: Arc<TopicService>,
}

impl TopicHandler {
    pub fn new(topic_service: Arc<TopicService>) -> Self {
        Self { topic_service }
    }

    pub async fn create_topic(
        &self,
        request: CreateTopicRequest,
    ) -> Result<TopicResponse, AppError> {
        request.validate().map_err(AppError::InvalidInput)?;

        let topic = self
            .topic_service
            .create_topic(request.name, Some(request.description))
            .await?;

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

    pub async fn update_topic(
        &self,
        request: UpdateTopicRequest,
    ) -> Result<TopicResponse, AppError> {
        request.validate().map_err(AppError::InvalidInput)?;

        let mut topic = self
            .topic_service
            .get_topic(&request.id)
            .await?
            .ok_or_else(|| AppError::NotFound("Topic not found".to_string()))?;

        if let Some(name) = request.name {
            topic.name = name;
        }
        if let Some(description) = request.description {
            topic.description = Some(description);
        }
        if let Some(image_url) = request.image_url {
            topic.image_url = if image_url.is_empty() {
                None
            } else {
                Some(image_url)
            };
        }
        topic.updated_at = Utc::now();

        self.topic_service.update_topic(&topic).await?;

        Ok(TopicResponse {
            id: topic.id.clone(),
            name: topic.name.clone(),
            description: topic.description.clone().unwrap_or_default(),
            image_url: topic.image_url.clone(),
            member_count: topic.member_count,
            post_count: topic.post_count,
            is_joined: topic.is_joined,
            created_at: topic.created_at.timestamp(),
            updated_at: topic.updated_at.timestamp(),
        })
    }

    pub async fn delete_topic(&self, request: DeleteTopicRequest) -> Result<(), AppError> {
        request.validate().map_err(AppError::InvalidInput)?;

        self.topic_service.delete_topic(&request.id).await?;
        Ok(())
    }

    pub async fn get_all_topics(&self) -> Result<Vec<TopicResponse>, AppError> {
        let topics = self.topic_service.get_all_topics().await?;

        Ok(topics
            .into_iter()
            .map(|t| TopicResponse {
                id: t.id.to_string(),
                name: t.name,
                description: t.description.unwrap_or_default(),
                image_url: t.image_url,
                member_count: t.member_count,
                post_count: t.post_count,
                is_joined: t.is_joined,
                created_at: t.created_at.timestamp(),
                updated_at: t.updated_at.timestamp(),
            })
            .collect())
    }

    pub async fn get_joined_topics(
        &self,
        user_pubkey: &str,
    ) -> Result<Vec<TopicResponse>, AppError> {
        let topics = self.topic_service.get_joined_topics(user_pubkey).await?;

        Ok(topics
            .into_iter()
            .map(|t| TopicResponse {
                id: t.id.to_string(),
                name: t.name,
                description: t.description.unwrap_or_default(),
                image_url: t.image_url,
                member_count: t.member_count,
                post_count: t.post_count,
                is_joined: true,
                created_at: t.created_at.timestamp(),
                updated_at: t.updated_at.timestamp(),
            })
            .collect())
    }

    pub async fn join_topic(
        &self,
        request: JoinTopicRequest,
        user_pubkey: &str,
    ) -> Result<(), AppError> {
        request.validate().map_err(AppError::InvalidInput)?;

        self.topic_service
            .join_topic(&request.topic_id, user_pubkey)
            .await?;
        Ok(())
    }

    pub async fn leave_topic(
        &self,
        request: JoinTopicRequest,
        user_pubkey: &str,
    ) -> Result<(), AppError> {
        request.validate().map_err(AppError::InvalidInput)?;

        self.topic_service
            .leave_topic(&request.topic_id, user_pubkey)
            .await?;
        Ok(())
    }

    pub async fn get_topic_stats(
        &self,
        request: GetTopicStatsRequest,
    ) -> Result<TopicStatsResponse, AppError> {
        request.validate().map_err(AppError::InvalidInput)?;

        let (member_count, post_count) = self
            .topic_service
            .get_topic_stats(&request.topic_id)
            .await?;

        let active_users_24h = member_count.min(post_count);
        let trending_score = if member_count == 0 && post_count == 0 {
            0.0
        } else {
            (post_count as f64 * 0.6) + (member_count as f64 * 0.4)
        };

        Ok(TopicStatsResponse {
            topic_id: request.topic_id,
            member_count,
            post_count,
            active_users_24h,
            trending_score,
        })
    }

    pub async fn list_trending_topics(
        &self,
        request: ListTrendingTopicsRequest,
    ) -> Result<ListTrendingTopicsResponse, AppError> {
        request.validate().map_err(AppError::InvalidInput)?;

        let limit = request.limit.unwrap_or(10).clamp(1, 100) as usize;
        let entries = self.topic_service.list_trending_topics(limit).await?;

        let topics: Vec<TrendingTopicDto> = entries
            .into_iter()
            .enumerate()
            .map(|(index, entry)| TrendingTopicDto {
                topic_id: entry.topic.id.clone(),
                name: entry.topic.name.clone(),
                description: entry.topic.description.clone(),
                member_count: entry.topic.member_count,
                post_count: entry.topic.post_count,
                trending_score: entry.trending_score,
                rank: (index as u32) + 1,
                score_change: None,
            })
            .collect();

        Ok(ListTrendingTopicsResponse {
            generated_at: Utc::now().timestamp_millis(),
            topics,
        })
    }
}
