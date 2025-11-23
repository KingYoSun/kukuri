use super::{TopicService, UserService};
use crate::application::ports::auth_lifecycle::{
    AuthAccountContext, AuthLifecycleEvent, AuthLifecyclePort, AuthLifecycleStage,
};
use crate::domain::constants::{DEFAULT_PUBLIC_TOPIC_ID, LEGACY_PUBLIC_TOPIC_ID};
use crate::domain::entities::User;
use crate::shared::error::AppError;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::debug;

pub struct DefaultAuthLifecycle {
    user_service: Arc<UserService>,
    topic_service: Arc<TopicService>,
    default_topics: Vec<String>,
}

impl DefaultAuthLifecycle {
    pub fn new(user_service: Arc<UserService>, topic_service: Arc<TopicService>) -> Self {
        Self {
            user_service,
            topic_service,
            default_topics: vec![DEFAULT_PUBLIC_TOPIC_ID.to_string()],
        }
    }

    pub fn with_default_topics(mut self, topics: Vec<String>) -> Self {
        if topics.is_empty() {
            return self;
        }
        self.default_topics = topics
            .into_iter()
            .map(|topic| {
                if topic == LEGACY_PUBLIC_TOPIC_ID {
                    DEFAULT_PUBLIC_TOPIC_ID.to_string()
                } else {
                    topic
                }
            })
            .collect();
        self
    }

    async fn ensure_topics_ready(&self) -> Result<(), AppError> {
        self.topic_service.ensure_public_topic().await
    }

    async fn join_default_topics(&self, user_pubkey: &str) {
        for topic in &self.default_topics {
            if let Err(err) = self.topic_service.join_topic(topic, user_pubkey).await {
                debug!(
                    topic = %topic,
                    pubkey = %user_pubkey,
                    error = %err,
                    "failed to join default topic (ignored)"
                );
            }
        }
    }

    async fn provision_account(&self, account: AuthAccountContext) -> Result<User, AppError> {
        self.ensure_topics_ready().await?;
        let user = self
            .user_service
            .create_user(account.npub.clone(), account.public_key.clone())
            .await?;
        self.join_default_topics(&account.public_key).await;
        Ok(user)
    }

    async fn handle_login(&self, account: AuthAccountContext) -> Result<User, AppError> {
        self.ensure_topics_ready().await?;
        let user = match self.user_service.get_user(&account.npub).await? {
            Some(existing) => existing,
            None => {
                self.user_service
                    .create_user(account.npub.clone(), account.public_key.clone())
                    .await?
            }
        };
        self.join_default_topics(&account.public_key).await;
        Ok(user)
    }
}

#[async_trait]
impl AuthLifecyclePort for DefaultAuthLifecycle {
    async fn handle(&self, event: AuthLifecycleEvent) -> Result<User, AppError> {
        match event.stage {
            AuthLifecycleStage::AccountCreated => self.provision_account(event.account).await,
            AuthLifecycleStage::Login => self.handle_login(event.account).await,
        }
    }

    async fn get_user(&self, npub: &str) -> Result<Option<User>, AppError> {
        self.user_service.get_user(npub).await
    }
}
