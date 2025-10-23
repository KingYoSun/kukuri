use crate::domain::entities::Topic;
use crate::infrastructure::database::TopicRepository;
use crate::infrastructure::p2p::GossipService;
use crate::shared::error::AppError;
use std::sync::Arc;

pub struct TopicService {
    repository: Arc<dyn TopicRepository>,
    gossip: Arc<dyn GossipService>,
}

impl TopicService {
    pub fn new(repository: Arc<dyn TopicRepository>, gossip: Arc<dyn GossipService>) -> Self {
        Self { repository, gossip }
    }

    pub async fn create_topic(
        &self,
        name: String,
        description: Option<String>,
    ) -> Result<Topic, AppError> {
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

    pub async fn get_joined_topics(&self, user_pubkey: &str) -> Result<Vec<Topic>, AppError> {
        self.repository.get_joined_topics(user_pubkey).await
    }

    pub async fn join_topic(&self, id: &str, user_pubkey: &str) -> Result<(), AppError> {
        self.repository.join_topic(id, user_pubkey).await?;
        self.gossip.join_topic(id, vec![]).await?;
        Ok(())
    }

    pub async fn leave_topic(&self, id: &str, user_pubkey: &str) -> Result<(), AppError> {
        self.repository.leave_topic(id, user_pubkey).await?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::database::TopicRepository as InfraTopicRepository;
    use crate::infrastructure::p2p::GossipService;
    use async_trait::async_trait;
    use mockall::{mock, predicate::*};

    mock! {
        pub TopicRepo {}

        #[async_trait]
        impl InfraTopicRepository for TopicRepo {
            async fn create_topic(&self, topic: &Topic) -> Result<(), AppError>;
            async fn get_topic(&self, id: &str) -> Result<Option<Topic>, AppError>;
            async fn get_all_topics(&self) -> Result<Vec<Topic>, AppError>;
            async fn get_joined_topics(&self, user_pubkey: &str) -> Result<Vec<Topic>, AppError>;
            async fn update_topic(&self, topic: &Topic) -> Result<(), AppError>;
            async fn delete_topic(&self, id: &str) -> Result<(), AppError>;
            async fn join_topic(&self, topic_id: &str, user_pubkey: &str) -> Result<(), AppError>;
            async fn leave_topic(&self, topic_id: &str, user_pubkey: &str) -> Result<(), AppError>;
            async fn update_topic_stats(
                &self,
                topic_id: &str,
                member_count: u32,
                post_count: u32,
            ) -> Result<(), AppError>;
        }
    }

    mock! {
        pub GossipSvc {}

        #[async_trait]
        impl GossipService for GossipSvc {
            async fn join_topic(&self, topic: &str, initial_peers: Vec<String>) -> Result<(), AppError>;
            async fn leave_topic(&self, topic: &str) -> Result<(), AppError>;
            async fn broadcast(&self, topic: &str, event: &crate::domain::entities::Event) -> Result<(), AppError>;
            async fn subscribe(&self, topic: &str) -> Result<tokio::sync::mpsc::Receiver<crate::domain::entities::Event>, AppError>;
            async fn get_joined_topics(&self) -> Result<Vec<String>, AppError>;
            async fn get_topic_peers(&self, topic: &str) -> Result<Vec<String>, AppError>;
            async fn get_topic_stats(&self, topic: &str) -> Result<Option<crate::domain::p2p::TopicStats>, AppError>;
            async fn broadcast_message(&self, topic: &str, message: &[u8]) -> Result<(), AppError>;
        }
    }

    #[tokio::test]
    async fn test_join_topic_calls_repository_and_gossip() {
        let mut repo = MockTopicRepo::new();
        repo.expect_join_topic()
            .with(eq("tech"), eq("pubkey1"))
            .times(1)
            .returning(|_, _| Ok(()));
        let mut gossip = MockGossipSvc::new();
        gossip
            .expect_join_topic()
            .with(eq("tech"), eq(Vec::<String>::new()))
            .times(1)
            .returning(|_, _| Ok(()));

        let repo_arc: Arc<dyn InfraTopicRepository> = Arc::new(repo);
        let gossip_arc: Arc<dyn GossipService> = Arc::new(gossip);
        let service = TopicService::new(repo_arc, gossip_arc);

        let result = service.join_topic("tech", "pubkey1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_leave_topic_calls_repository_and_gossip() {
        let mut repo = MockTopicRepo::new();
        repo.expect_leave_topic()
            .with(eq("tech"), eq("pubkey1"))
            .times(1)
            .returning(|_, _| Ok(()));
        let mut gossip = MockGossipSvc::new();
        gossip
            .expect_leave_topic()
            .with(eq("tech"))
            .times(1)
            .returning(|_| Ok(()));

        let repo_arc: Arc<dyn InfraTopicRepository> = Arc::new(repo);
        let gossip_arc: Arc<dyn GossipService> = Arc::new(gossip);
        let service = TopicService::new(repo_arc, gossip_arc);

        let result = service.leave_topic("tech", "pubkey1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_joined_topics_passes_user_pubkey() {
        let mut repo = MockTopicRepo::new();
        repo.expect_get_joined_topics()
            .with(eq("pubkey1"))
            .times(1)
            .returning(|_| Ok(vec![]));
        let gossip = MockGossipSvc::new();

        let repo_arc: Arc<dyn InfraTopicRepository> = Arc::new(repo);
        let gossip_arc: Arc<dyn GossipService> = Arc::new(gossip);
        let service = TopicService::new(repo_arc, gossip_arc);
        let result = service.get_joined_topics("pubkey1").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
}
