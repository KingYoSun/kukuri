use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::domain::entities as domain;
use crate::infrastructure::p2p::GossipService;
use crate::modules::p2p::TopicStats;
use crate::shared::error::AppError;

pub struct TestGossipService {
    joined: Arc<RwLock<HashSet<String>>>,
    broadcasts: Arc<RwLock<Vec<(String, domain::Event)>>>,
}

impl TestGossipService {
    pub fn new() -> Self {
        Self {
            joined: Arc::new(RwLock::new(HashSet::new())),
            broadcasts: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn joined_topics(&self) -> HashSet<String> {
        self.joined.read().await.clone()
    }

    pub async fn broadcasted_topics(&self) -> Vec<String> {
        self.broadcasts
            .read()
            .await
            .iter()
            .map(|(t, _)| t.clone())
            .collect()
    }
}

impl Default for TestGossipService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl GossipService for TestGossipService {
    async fn join_topic(&self, topic: &str, _initial_peers: Vec<String>) -> Result<(), AppError> {
        let mut j = self.joined.write().await;
        j.insert(topic.to_string());
        Ok(())
    }

    async fn leave_topic(&self, _topic: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn broadcast(&self, topic: &str, event: &domain::Event) -> Result<(), AppError> {
        let mut b = self.broadcasts.write().await;
        b.push((topic.to_string(), event.clone()));
        Ok(())
    }

    async fn subscribe(
        &self,
        _topic: &str,
    ) -> Result<tokio::sync::mpsc::Receiver<domain::Event>, AppError> {
        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        Ok(rx)
    }

    async fn get_joined_topics(&self) -> Result<Vec<String>, AppError> {
        Ok(vec![])
    }

    async fn get_topic_peers(&self, _topic: &str) -> Result<Vec<String>, AppError> {
        Ok(vec![])
    }

    async fn get_topic_stats(&self, _topic: &str) -> Result<Option<TopicStats>, AppError> {
        Ok(None)
    }

    async fn broadcast_message(&self, _topic: &str, _message: &[u8]) -> Result<(), AppError> {
        Ok(())
    }
}
