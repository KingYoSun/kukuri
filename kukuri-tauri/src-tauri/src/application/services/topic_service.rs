use crate::domain::entities::Topic;
use crate::shared::error::AppError;
use crate::infrastructure::database::TopicRepository;
use crate::infrastructure::p2p::GossipService;
use std::sync::Arc;

pub struct TopicService {
    repository: Arc<dyn TopicRepository>,
    gossip: Arc<dyn GossipService>,
}

impl TopicService {
    pub fn new(repository: Arc<dyn TopicRepository>, gossip: Arc<dyn GossipService>) -> Self {
        Self {
            repository,
            gossip,
        }
    }

    pub async fn create_topic(&self, name: String, description: Option<String>) -> Result<Topic, AppError> {
        let topic = Topic::new(name, description);
        self.repository.create_topic(&topic).await?;
        
        // Join gossip topic
        self.gossip.join_topic(&topic.id, vec![]).await?;
        
        Ok(topic)
    }


    pub async fn get_topic(&self, id: &str) -> Result<Option<Topic>, AppError> {
        self.repository.get_topic(id).await
    }


    pub async fn get_all_topics(&self) -> Result<Vec<Topic>, AppError> {
        self.repository.get_all_topics().await
    }


    pub async fn get_joined_topics(&self) -> Result<Vec<Topic>, AppError> {
        self.repository.get_joined_topics().await
    }


    pub async fn join_topic(&self, id: &str) -> Result<(), AppError> {
        self.repository.join_topic(id).await?;
        self.gossip.join_topic(id, vec![]).await?;
        Ok(())
    }


    pub async fn leave_topic(&self, id: &str) -> Result<(), AppError> {
        self.repository.leave_topic(id).await?;
        self.gossip.leave_topic(id).await?;
        Ok(())
    }


    pub async fn update_topic(&self, topic: &Topic) -> Result<(), AppError> {
        self.repository.update_topic(topic).await
    }

    pub async fn delete_topic(&self, id: &str) -> Result<(), AppError> {
        // Prevent deletion of public topic
        if id == "public" {
            return Err("Cannot delete public topic".into());
        }
        
        self.gossip.leave_topic(id).await?;
        self.repository.delete_topic(id).await
    }

    pub async fn get_topic_stats(&self, id: &str) -> Result<(u32, u32), AppError> {
        if let Some(topic) = self.repository.get_topic(id).await? {
            Ok((topic.member_count, topic.post_count))
        } else {
            Ok((0, 0))
        }
    }


    pub async fn ensure_public_topic(&self) -> Result<(), AppError> {
        if self.repository.get_topic("public").await?.is_none() {
            let public_topic = Topic::public_topic();
            self.repository.create_topic(&public_topic).await?;
            self.gossip.join_topic("public", vec![]).await?;
        }
        Ok(())
    }
}