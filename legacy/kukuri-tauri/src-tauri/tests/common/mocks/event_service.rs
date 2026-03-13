use async_trait::async_trait;
use mockall::mock;
use nostr_sdk::prelude::Timestamp;

use kukuri_lib::application::ports::subscription_invoker::SubscriptionInvoker;
use kukuri_lib::application::services::subscription_state::SubscriptionStateStore;
use kukuri_lib::domain::value_objects::subscription::{SubscriptionRecord, SubscriptionTarget};
use kukuri_lib::domain::entities::Event;
use kukuri_lib::infrastructure::crypto::SignatureService;
use kukuri_lib::infrastructure::database::EventRepository;
use kukuri_lib::infrastructure::p2p::{event_distributor::DistributionStrategy, EventDistributor};
use kukuri_lib::shared::error::AppError;

mock! {
    pub EventRepo {}

    #[async_trait]
    impl EventRepository for EventRepo {
        async fn create_event(&self, event: &Event) -> Result<(), AppError>;
        async fn get_event(&self, id: &str) -> Result<Option<Event>, AppError>;
        async fn get_events_by_kind(&self, kind: u32, limit: usize) -> Result<Vec<Event>, AppError>;
        async fn get_events_by_author(&self, pubkey: &str, limit: usize) -> Result<Vec<Event>, AppError>;
        async fn delete_event(&self, id: &str) -> Result<(), AppError>;
        async fn get_unsync_events(&self) -> Result<Vec<Event>, AppError>;
        async fn mark_event_synced(&self, id: &str) -> Result<(), AppError>;
        async fn add_event_topic(&self, event_id: &str, topic_id: &str) -> Result<(), AppError>;
        async fn get_event_topics(&self, event_id: &str) -> Result<Vec<String>, AppError>;
    }
}

mock! {
    pub SignatureServ {}

    #[async_trait]
    impl SignatureService for SignatureServ {
        async fn sign_event(&self, event: &mut Event, private_key: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
        async fn verify_event(&self, event: &Event) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;
        async fn sign_message(&self, message: &str, private_key: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;
        async fn verify_message(&self, message: &str, signature: &str, public_key: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;
    }
}

mock! {
    pub EventDist {}

    #[async_trait]
    impl EventDistributor for EventDist {
        async fn distribute(&self, event: &Event, strategy: DistributionStrategy) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
        async fn receive(&self) -> Result<Option<Event>, Box<dyn std::error::Error + Send + Sync>>;
        async fn set_strategy(&self, strategy: DistributionStrategy);
        async fn get_pending_events(&self) -> Result<Vec<Event>, Box<dyn std::error::Error + Send + Sync>>;
        async fn retry_failed(&self) -> Result<u32, Box<dyn std::error::Error + Send + Sync>>;
    }
}

mock! {
    pub SubscriptionStateMock {}

    #[async_trait]
    impl SubscriptionStateStore for SubscriptionStateMock {
        async fn record_request(&self, target: SubscriptionTarget) -> Result<SubscriptionRecord, AppError>;
        async fn mark_subscribed(&self, target: &SubscriptionTarget, synced_at: i64) -> Result<(), AppError>;
        async fn mark_failure(&self, target: &SubscriptionTarget, error: &str) -> Result<(), AppError>;
        async fn mark_all_need_resync(&self) -> Result<(), AppError>;
        async fn list_for_restore(&self) -> Result<Vec<SubscriptionRecord>, AppError>;
        async fn list_all(&self) -> Result<Vec<SubscriptionRecord>, AppError>;
    }
}

mock! {
    pub SubscriptionInvokerMock {}

    #[async_trait]
    impl SubscriptionInvoker for SubscriptionInvokerMock {
        async fn subscribe_topic(&self, topic_id: &str, since: Option<Timestamp>) -> Result<(), AppError>;
        async fn subscribe_user(&self, pubkey: &str, since: Option<Timestamp>) -> Result<(), AppError>;
    }
}
