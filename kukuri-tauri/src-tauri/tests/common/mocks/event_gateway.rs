use async_trait::async_trait;
use mockall::mock;

use kukuri_lib::application::ports::event_gateway::EventGateway;
use kukuri_lib::domain::entities::event_gateway::{DomainEvent, ProfileMetadata};
use kukuri_lib::domain::value_objects::event_gateway::{PublicKey, ReactionValue, TopicContent};
use kukuri_lib::domain::value_objects::{EventId, TopicId};
use kukuri_lib::shared::error::AppError;

mock! {
    pub EventGatewayPort {}

    #[async_trait]
    impl EventGateway for EventGatewayPort {
        async fn handle_incoming_event(&self, event: DomainEvent) -> Result<(), AppError>;
        async fn publish_text_note(&self, content: &str) -> Result<EventId, AppError>;
        async fn publish_topic_post(
            &self,
            topic_id: &TopicId,
            content: &TopicContent,
            reply_to: Option<&EventId>,
        ) -> Result<EventId, AppError>;
        async fn send_reaction(
            &self,
            target: &EventId,
            reaction: &ReactionValue,
        ) -> Result<EventId, AppError>;
        async fn update_profile_metadata(
            &self,
            metadata: &ProfileMetadata,
        ) -> Result<EventId, AppError>;
        async fn delete_events(
            &self,
            targets: &[EventId],
            reason: Option<&str>,
        ) -> Result<EventId, AppError>;
        async fn disconnect(&self) -> Result<(), AppError>;
        async fn get_public_key(&self) -> Result<Option<PublicKey>, AppError>;
        async fn set_default_topics(&self, topics: &[TopicId]) -> Result<(), AppError>;
        async fn list_default_topics(&self) -> Result<Vec<TopicId>, AppError>;
    }
}

pub type MockEventGateway = MockEventGatewayPort;
