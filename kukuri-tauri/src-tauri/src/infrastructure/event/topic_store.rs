use crate::application::ports::event_topic_store::EventTopicStore;
use crate::infrastructure::database::EventRepository;
use crate::shared::error::AppError;
use async_trait::async_trait;
use std::sync::Arc;

pub struct RepositoryEventTopicStore {
    repository: Arc<dyn EventRepository>,
}

impl RepositoryEventTopicStore {
    pub fn new(repository: Arc<dyn EventRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl EventTopicStore for RepositoryEventTopicStore {
    async fn add_event_topic(&self, event_id: &str, topic_id: &str) -> Result<(), AppError> {
        self.repository.add_event_topic(event_id, topic_id).await
    }

    async fn get_event_topics(&self, event_id: &str) -> Result<Vec<String>, AppError> {
        self.repository.get_event_topics(event_id).await
    }
}
