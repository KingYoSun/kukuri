use async_trait::async_trait;
use kukuri_lib::application::ports::key_manager::KeyPair;
use kukuri_lib::application::services::event_service::EventService;
use kukuri_lib::application::services::subscription_state::SubscriptionStateStore;
use kukuri_lib::domain::value_objects::subscription::{
    SubscriptionRecord, SubscriptionStatus, SubscriptionTarget,
};
use kukuri_lib::domain::entities::Event;
use kukuri_lib::domain::value_objects::EventId;
use kukuri_lib::infrastructure::crypto::SignatureService;
use kukuri_lib::infrastructure::database::EventRepository;
use kukuri_lib::infrastructure::event::{
    EventManagerHandle, EventManagerSubscriptionInvoker, LegacyEventManagerGateway,
};
use kukuri_lib::infrastructure::p2p::GossipService;
use kukuri_lib::infrastructure::p2p::event_distributor::{
    DistributionStrategy, EventDistributor,
};
use kukuri_lib::presentation::dto::event::NostrMetadataDto;
use kukuri_lib::shared::error::AppError;
use nostr_sdk::prelude::{EventId as NostrEventId, Keys, Metadata, PublicKey};
use nostr_sdk::Timestamp;
use serde_json::Value;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

fn repeating_hex(ch: char) -> String {
    std::iter::repeat(ch).take(64).collect()
}

fn to_event_id(hex: &str) -> EventId {
    EventId::from_hex(hex).expect("valid hex event id")
}

#[tokio::test]
async fn publish_text_note_routes_through_gateway() {
    let (service, manager, _) = build_service().await;
    let event_id = service
        .publish_text_note("phase5-text-note")
        .await
        .expect("publish text note");

    assert_eq!(
        event_id.to_hex(),
        manager.last_event_id().await.expect("event id recorded")
    );
    let notes = manager.text_notes().await;
    assert_eq!(notes, vec!["phase5-text-note".to_string()]);
}

#[tokio::test]
async fn publish_topic_post_converts_topic_and_reply() {
    let (service, manager, _) = build_service().await;
    let reply_id = repeating_hex('b');
    service
        .publish_topic_post("public", "phase5 topic body", Some(&reply_id))
        .await
        .expect("publish topic post");

    let posts = manager.topic_posts().await;
    assert_eq!(posts.len(), 1);
    let post = &posts[0];
    assert_eq!(post.topic_id, "public");
    assert_eq!(post.content, "phase5 topic body");
    assert_eq!(post.reply_to_hex.as_deref(), Some(&reply_id));
}

#[tokio::test]
async fn send_reaction_uses_gateway_and_passes_parameters() {
    let (service, manager, _) = build_service().await;
    let target = repeating_hex('c');
    service
        .send_reaction(&target, ":+1:")
        .await
        .expect("send reaction");

    let reactions = manager.reactions().await;
    assert_eq!(reactions, vec![(target, ":+1:".to_string())]);
}

#[tokio::test]
async fn update_metadata_flows_through_conversion() {
    let (service, manager, _) = build_service().await;
    let dto = NostrMetadataDto {
        name: Some("Alice".into()),
        display_name: Some("Alice / Phase5".into()),
        about: Some("updating metadata".into()),
        picture: Some("https://example.com/p.png".into()),
        banner: None,
        nip05: None,
        lud16: Some("alice@getalby.com".into()),
        website: Some("https://kukuri.app".into()),
    };

    service
        .update_metadata(dto.clone())
        .await
        .expect("update metadata succeeds");

    let metadata = manager.metadata().await;
    assert_eq!(metadata.len(), 1);
    let serialized: Value =
        serde_json::to_value(&metadata[0]).expect("metadata serializable to json");
    assert_eq!(serialized["name"], dto.name.unwrap().into());
    assert_eq!(serialized["display_name"], dto.display_name.unwrap().into());
    assert_eq!(serialized["about"], dto.about.unwrap().into());
    assert_eq!(serialized["picture"], dto.picture.unwrap().into());
    assert_eq!(serialized["lud16"], dto.lud16.unwrap().into());
    assert_eq!(serialized["website"], dto.website.unwrap().into());
}

#[tokio::test]
async fn delete_events_invokes_gateway_and_repository_cleanup() {
    let (service, manager, repo) = build_service().await;
    let targets = vec![repeating_hex('d'), repeating_hex('e')];
    service
        .delete_events(targets.clone(), Some("cleanup".into()))
        .await
        .expect("delete events");

    let deletions = manager.deletions().await;
    assert_eq!(deletions.len(), 1);
    assert_eq!(deletions[0].targets, targets);
    assert_eq!(deletions[0].reason.as_deref(), Some("cleanup"));

    let deleted_ids = repo.deleted_ids().await;
    assert_eq!(deleted_ids, targets);
}

async fn build_service(
) -> (
    EventService,
    Arc<RecordingEventManager>,
    Arc<RecordingEventRepository>,
) {
    let manager = Arc::new(RecordingEventManager::new());
    let repository = Arc::new(RecordingEventRepository::default());
    let manager_trait: Arc<dyn EventManagerHandle> = Arc::clone(&manager);
    let event_gateway = Arc::new(LegacyEventManagerGateway::new(Arc::clone(&manager_trait)));

    let mut service = EventService::new(
        Arc::clone(&repository) as Arc<dyn EventRepository>,
        Arc::new(NoopSignatureService),
        Arc::new(NoopEventDistributor),
        event_gateway,
        Arc::new(NoopSubscriptionStateStore),
    );
    service.set_subscription_invoker(Arc::new(EventManagerSubscriptionInvoker::new(
        Arc::clone(&manager_trait),
    )));

    (service, manager, repository)
}

#[derive(Clone)]
struct RecordingEventManager {
    text_notes: Arc<Mutex<Vec<String>>>,
    topic_posts: Arc<Mutex<Vec<TopicPostRecord>>>,
    reactions: Arc<Mutex<Vec<(String, String)>>>,
    metadata: Arc<Mutex<Vec<Metadata>>>,
    deletions: Arc<Mutex<Vec<DeletionRecord>>>,
    default_topics: Arc<Mutex<Vec<String>>>,
    public_key: PublicKey,
    counter: Arc<AtomicU32>,
    last_event_hex: Arc<Mutex<Option<String>>>,
}

impl RecordingEventManager {
    fn new() -> Self {
        let keys = Keys::generate();
        Self {
            text_notes: Arc::new(Mutex::new(Vec::new())),
            topic_posts: Arc::new(Mutex::new(Vec::new())),
            reactions: Arc::new(Mutex::new(Vec::new())),
            metadata: Arc::new(Mutex::new(Vec::new())),
            deletions: Arc::new(Mutex::new(Vec::new())),
            default_topics: Arc::new(Mutex::new(vec!["public".into()])),
            public_key: keys.public_key(),
            counter: Arc::new(AtomicU32::new(1)),
            last_event_hex: Arc::new(Mutex::new(None)),
        }
    }

    async fn record_event_id(&self, id: &NostrEventId) {
        let mut guard = self.last_event_hex.lock().await;
        *guard = Some(id.to_hex());
    }

    async fn last_event_id(&self) -> Option<String> {
        self.last_event_hex.lock().await.clone()
    }

    async fn text_notes(&self) -> Vec<String> {
        self.text_notes.lock().await.clone()
    }

    async fn topic_posts(&self) -> Vec<TopicPostRecord> {
        self.topic_posts.lock().await.clone()
    }

    async fn reactions(&self) -> Vec<(String, String)> {
        self.reactions.lock().await.clone()
    }

    async fn metadata(&self) -> Vec<Metadata> {
        self.metadata.lock().await.clone()
    }

    async fn deletions(&self) -> Vec<DeletionRecord> {
        self.deletions.lock().await.clone()
    }

    fn next_event_id(&self) -> NostrEventId {
        let next = self.counter.fetch_add(1, Ordering::Relaxed);
        let mut bytes = [0u8; 32];
        bytes[..4].copy_from_slice(&next.to_be_bytes());
        NostrEventId::from_slice(&bytes).expect("event id from counter")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TopicPostRecord {
    topic_id: String,
    content: String,
    reply_to_hex: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DeletionRecord {
    targets: Vec<String>,
    reason: Option<String>,
}

#[async_trait]
impl EventManagerHandle for RecordingEventManager {
    async fn set_gossip_service(&self, _gossip: Arc<dyn GossipService>) {}

    async fn set_event_repository(&self, _repo: Arc<dyn EventRepository>) {}

    async fn set_default_p2p_topic_id(&self, topic_id: &str) {
        let mut guard = self.default_topics.lock().await;
        guard.clear();
        guard.push(topic_id.to_string());
    }

    async fn set_default_p2p_topics(&self, topics: Vec<String>) {
        let mut guard = self.default_topics.lock().await;
        *guard = topics;
    }

    async fn list_default_p2p_topics(&self) -> Vec<String> {
        self.default_topics.lock().await.clone()
    }

    async fn handle_p2p_event(&self, _event: nostr_sdk::Event) -> anyhow::Result<()> {
        Ok(())
    }

    async fn publish_text_note(&self, content: &str) -> anyhow::Result<NostrEventId> {
        let event_id = self.next_event_id();
        self.record_event_id(&event_id).await;
        self.text_notes.lock().await.push(content.to_string());
        Ok(event_id)
    }

    async fn publish_topic_post(
        &self,
        topic_id: &str,
        content: &str,
        reply_to: Option<NostrEventId>,
    ) -> anyhow::Result<NostrEventId> {
        let event_id = self.next_event_id();
        self.record_event_id(&event_id).await;
        self.topic_posts.lock().await.push(TopicPostRecord {
            topic_id: topic_id.to_string(),
            content: content.to_string(),
            reply_to_hex: reply_to.map(|id| id.to_hex()),
        });
        Ok(event_id)
    }

    async fn send_reaction(
        &self,
        target: &NostrEventId,
        reaction: &str,
    ) -> anyhow::Result<NostrEventId> {
        let event_id = self.next_event_id();
        self.record_event_id(&event_id).await;
        self.reactions
            .lock()
            .await
            .push((target.to_hex(), reaction.to_string()));
        Ok(event_id)
    }

    async fn update_metadata(&self, metadata: Metadata) -> anyhow::Result<NostrEventId> {
        let event_id = self.next_event_id();
        self.record_event_id(&event_id).await;
        self.metadata.lock().await.push(metadata);
        Ok(event_id)
    }

    async fn delete_events(
        &self,
        target_ids: Vec<NostrEventId>,
        reason: Option<String>,
    ) -> anyhow::Result<NostrEventId> {
        let event_id = self.next_event_id();
        self.record_event_id(&event_id).await;
        self.deletions.lock().await.push(DeletionRecord {
            targets: target_ids.into_iter().map(|id| id.to_hex()).collect(),
            reason,
        });
        Ok(event_id)
    }

    async fn disconnect(&self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn get_public_key(&self) -> Option<PublicKey> {
        Some(self.public_key)
    }

    async fn subscribe_to_topic(
        &self,
        _topic_id: &str,
        _since: Option<Timestamp>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn subscribe_to_user(
        &self,
        _pubkey: PublicKey,
        _since: Option<Timestamp>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn initialize_with_keypair(&self, _keypair: KeyPair) -> anyhow::Result<()> {
        Ok(())
    }
}

#[derive(Default)]
struct RecordingEventRepository {
    deleted: Mutex<Vec<String>>,
}

impl RecordingEventRepository {
    async fn deleted_ids(&self) -> Vec<String> {
        self.deleted.lock().await.clone()
    }
}

#[async_trait]
impl EventRepository for RecordingEventRepository {
    async fn create_event(&self, _event: &Event) -> Result<(), AppError> {
        Ok(())
    }

    async fn get_event(&self, _id: &str) -> Result<Option<Event>, AppError> {
        Ok(None)
    }

    async fn get_events_by_kind(
        &self,
        _kind: u32,
        _limit: usize,
    ) -> Result<Vec<Event>, AppError> {
        Ok(vec![])
    }

    async fn get_events_by_author(
        &self,
        _pubkey: &str,
        _limit: usize,
    ) -> Result<Vec<Event>, AppError> {
        Ok(vec![])
    }

    async fn delete_event(&self, id: &str) -> Result<(), AppError> {
        self.deleted.lock().await.push(id.to_string());
        Ok(())
    }

    async fn get_unsync_events(&self) -> Result<Vec<Event>, AppError> {
        Ok(vec![])
    }

    async fn mark_event_synced(&self, _id: &str) -> Result<(), AppError> {
        Ok(())
    }
}

struct NoopSignatureService;

#[async_trait]
impl SignatureService for NoopSignatureService {
    async fn sign_event(
        &self,
        _event: &mut Event,
        _private_key: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    async fn verify_event(
        &self,
        _event: &Event,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        Ok(true)
    }

    async fn sign_message(
        &self,
        message: &str,
        _private_key: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        Ok(message.to_string())
    }

    async fn verify_message(
        &self,
        _message: &str,
        _signature: &str,
        _public_key: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        Ok(true)
    }
}

struct NoopEventDistributor;

#[async_trait]
impl EventDistributor for NoopEventDistributor {
    async fn distribute(
        &self,
        _event: &Event,
        _strategy: DistributionStrategy,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    async fn receive(&self) -> Result<Option<Event>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(None)
    }

    async fn set_strategy(&self, _strategy: DistributionStrategy) {}

    async fn get_pending_events(
        &self,
    ) -> Result<Vec<Event>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(vec![])
    }

    async fn retry_failed(&self) -> Result<u32, Box<dyn std::error::Error + Send + Sync>> {
        Ok(0)
    }
}

struct NoopSubscriptionStateStore;

#[async_trait]
impl SubscriptionStateStore for NoopSubscriptionStateStore {
    async fn record_request(
        &self,
        target: SubscriptionTarget,
    ) -> Result<SubscriptionRecord, AppError> {
        Ok(SubscriptionRecord {
            target,
            status: SubscriptionStatus::Pending,
            last_synced_at: None,
            last_attempt_at: None,
            failure_count: 0,
            error_message: None,
        })
    }

    async fn mark_subscribed(
        &self,
        _target: &SubscriptionTarget,
        _synced_at: i64,
    ) -> Result<(), AppError> {
        Ok(())
    }

    async fn mark_failure(&self, _target: &SubscriptionTarget, _error: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn mark_all_need_resync(&self) -> Result<(), AppError> {
        Ok(())
    }

    async fn list_for_restore(&self) -> Result<Vec<SubscriptionRecord>, AppError> {
        Ok(vec![])
    }

    async fn list_all(&self) -> Result<Vec<SubscriptionRecord>, AppError> {
        Ok(vec![])
    }
}
