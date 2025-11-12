use crate::{
    application::services::TopicService,
    domain::entities::PendingTopic,
    presentation::dto::{
        Validate,
        offline::OfflineAction,
        topic_dto::{
            CreateTopicRequest, DeleteTopicRequest, EnqueueTopicCreationRequest,
            EnqueueTopicCreationResponse, GetTopicStatsRequest, JoinTopicRequest,
            ListTrendingTopicsRequest, ListTrendingTopicsResponse, MarkPendingTopicFailedRequest,
            MarkPendingTopicSyncedRequest, PendingTopicResponse, TopicResponse, TopicStatsResponse,
            TrendingTopicDto, UpdateTopicRequest,
        },
    },
    presentation::handlers::offline_handler::map_action_record,
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
        user_pubkey: &str,
    ) -> Result<TopicResponse, AppError> {
        request.validate().map_err(AppError::InvalidInput)?;

        let topic = self
            .topic_service
            .create_topic(request.name, Some(request.description), user_pubkey)
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

    pub async fn enqueue_topic_creation(
        &self,
        request: EnqueueTopicCreationRequest,
        user_pubkey: &str,
    ) -> Result<EnqueueTopicCreationResponse, AppError> {
        request.validate().map_err(AppError::InvalidInput)?;

        let result = self
            .topic_service
            .enqueue_topic_creation(user_pubkey, request.name, request.description)
            .await?;

        Ok(EnqueueTopicCreationResponse {
            pending_topic: map_pending_topic(result.pending_topic),
            offline_action: map_action_record(&result.offline_action)?,
        })
    }

    pub async fn list_pending_topics(
        &self,
        user_pubkey: &str,
    ) -> Result<Vec<PendingTopicResponse>, AppError> {
        let topics = self.topic_service.list_pending_topics(user_pubkey).await?;
        Ok(topics.into_iter().map(map_pending_topic).collect())
    }

    pub async fn mark_pending_topic_synced(
        &self,
        request: MarkPendingTopicSyncedRequest,
        user_pubkey: &str,
    ) -> Result<PendingTopicResponse, AppError> {
        request.validate().map_err(AppError::InvalidInput)?;

        let pending = self
            .topic_service
            .get_pending_topic(&request.pending_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Pending topic not found".to_string()))?;

        if pending.user_pubkey != user_pubkey {
            return Err(AppError::Unauthorized(
                "You cannot modify this pending topic".to_string(),
            ));
        }

        self.topic_service
            .mark_pending_topic_synced(&request.pending_id, &request.topic_id)
            .await?;

        let updated = self
            .topic_service
            .get_pending_topic(&request.pending_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Pending topic not found".to_string()))?;

        Ok(map_pending_topic(updated))
    }

    pub async fn mark_pending_topic_failed(
        &self,
        request: MarkPendingTopicFailedRequest,
        user_pubkey: &str,
    ) -> Result<PendingTopicResponse, AppError> {
        request.validate().map_err(AppError::InvalidInput)?;

        let pending = self
            .topic_service
            .get_pending_topic(&request.pending_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Pending topic not found".to_string()))?;

        if pending.user_pubkey != user_pubkey {
            return Err(AppError::Unauthorized(
                "You cannot modify this pending topic".to_string(),
            ));
        }

        self.topic_service
            .mark_pending_topic_failed(&request.pending_id, request.error_message.clone())
            .await?;

        let updated = self
            .topic_service
            .get_pending_topic(&request.pending_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Pending topic not found".to_string()))?;

        Ok(map_pending_topic(updated))
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
        let result = self.topic_service.list_trending_topics(limit).await?;

        let topics: Vec<TrendingTopicDto> = result
            .entries
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
            generated_at: result.generated_at,
            topics,
        })
    }
}

fn map_pending_topic(topic: PendingTopic) -> PendingTopicResponse {
    PendingTopicResponse {
        pending_id: topic.pending_id,
        name: topic.name,
        description: topic.description,
        status: topic.status.as_str().to_string(),
        offline_action_id: topic.offline_action_id,
        synced_topic_id: topic.synced_topic_id,
        error_message: topic.error_message,
        created_at: topic.created_at.timestamp(),
        updated_at: topic.updated_at.timestamp(),
    }
}
