use async_trait::async_trait;
use kukuri_lib::test_support::application::ports::cache::PostCache;
use kukuri_lib::test_support::application::ports::group_key_store::{
    GroupKeyEntry, GroupKeyRecord, GroupKeyStore,
};
use kukuri_lib::test_support::application::ports::join_request_store::{
    InviteUsageRecord, JoinRequestRecord, JoinRequestStore,
};
use kukuri_lib::test_support::application::ports::key_manager::{KeyManager, KeyPair};
use kukuri_lib::test_support::application::ports::repositories::{
    BookmarkRepository, FollowListSort, PostRepository, UserCursorPage, UserRepository,
};
use kukuri_lib::test_support::application::services::event_service::EventServiceTrait;
use kukuri_lib::test_support::application::services::{AccessControlService, JoinRequestInput, PostService};
use kukuri_lib::test_support::domain::entities::{Event, Post, User};
use kukuri_lib::test_support::domain::p2p::user_topic_id;
use kukuri_lib::test_support::domain::value_objects::{EncryptedPostPayload, EventId};
use kukuri_lib::test_support::infrastructure::crypto::DefaultSignatureService;
use kukuri_lib::test_support::infrastructure::database::connection_pool::ConnectionPool;
use kukuri_lib::test_support::infrastructure::database::sqlite_repository::SqliteRepository;
use kukuri_lib::test_support::shared::error::AppError;
use nostr_sdk::prelude::{Keys, ToBech32};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

#[derive(Clone)]
struct TestKeyManager {
    keypair: KeyPair,
}

impl TestKeyManager {
    fn new(keypair: KeyPair) -> Self {
        Self { keypair }
    }
}

#[async_trait]
impl KeyManager for TestKeyManager {
    async fn generate_keypair(&self) -> Result<KeyPair, AppError> {
        Err(AppError::NotImplemented("generate_keypair".into()))
    }

    async fn import_private_key(&self, _nsec: &str) -> Result<KeyPair, AppError> {
        Err(AppError::NotImplemented("import_private_key".into()))
    }

    async fn export_private_key(&self, _npub: &str) -> Result<String, AppError> {
        Err(AppError::NotImplemented("export_private_key".into()))
    }

    async fn get_public_key(&self, _npub: &str) -> Result<String, AppError> {
        Err(AppError::NotImplemented("get_public_key".into()))
    }

    async fn store_keypair(&self, _keypair: &KeyPair) -> Result<(), AppError> {
        Err(AppError::NotImplemented("store_keypair".into()))
    }

    async fn delete_keypair(&self, _npub: &str) -> Result<(), AppError> {
        Err(AppError::NotImplemented("delete_keypair".into()))
    }

    async fn list_npubs(&self) -> Result<Vec<String>, AppError> {
        Ok(vec![self.keypair.npub.clone()])
    }

    async fn current_keypair(&self) -> Result<KeyPair, AppError> {
        Ok(self.keypair.clone())
    }
}

#[derive(Clone, Default)]
struct TestGroupKeyStore {
    records: Arc<RwLock<Vec<GroupKeyRecord>>>,
}

#[async_trait]
impl GroupKeyStore for TestGroupKeyStore {
    async fn store_key(&self, record: GroupKeyRecord) -> Result<(), AppError> {
        let mut records = self.records.write().await;
        records.retain(|entry| {
            !(entry.topic_id == record.topic_id
                && entry.scope == record.scope
                && entry.epoch == record.epoch)
        });
        records.push(record);
        Ok(())
    }

    async fn get_key(
        &self,
        topic_id: &str,
        scope: &str,
        epoch: i64,
    ) -> Result<Option<GroupKeyRecord>, AppError> {
        let records = self.records.read().await;
        Ok(records
            .iter()
            .find(|entry| {
                entry.topic_id == topic_id && entry.scope == scope && entry.epoch == epoch
            })
            .cloned())
    }

    async fn get_latest_key(
        &self,
        topic_id: &str,
        scope: &str,
    ) -> Result<Option<GroupKeyRecord>, AppError> {
        let records = self.records.read().await;
        Ok(records
            .iter()
            .filter(|entry| entry.topic_id == topic_id && entry.scope == scope)
            .max_by_key(|entry| entry.epoch)
            .cloned())
    }

    async fn list_keys(&self) -> Result<Vec<GroupKeyEntry>, AppError> {
        let records = self.records.read().await;
        Ok(records
            .iter()
            .map(|entry| GroupKeyEntry {
                topic_id: entry.topic_id.clone(),
                scope: entry.scope.clone(),
                epoch: entry.epoch,
                stored_at: entry.stored_at,
            })
            .collect())
    }
}

#[derive(Clone, Default)]
struct TestJoinRequestStore {
    records: Arc<RwLock<HashMap<String, HashMap<String, JoinRequestRecord>>>>,
    invite_usage: Arc<RwLock<HashMap<String, HashMap<String, InviteUsageRecord>>>>,
}

#[async_trait]
impl JoinRequestStore for TestJoinRequestStore {
    async fn upsert_request(
        &self,
        owner_pubkey: &str,
        record: JoinRequestRecord,
    ) -> Result<(), AppError> {
        let mut records = self.records.write().await;
        let owner = records.entry(owner_pubkey.to_string()).or_default();
        owner.insert(record.event.id.clone(), record);
        Ok(())
    }

    async fn list_requests(&self, owner_pubkey: &str) -> Result<Vec<JoinRequestRecord>, AppError> {
        let records = self.records.read().await;
        Ok(records
            .get(owner_pubkey)
            .map(|owner| owner.values().cloned().collect())
            .unwrap_or_default())
    }

    async fn get_request(
        &self,
        owner_pubkey: &str,
        event_id: &str,
    ) -> Result<Option<JoinRequestRecord>, AppError> {
        let records = self.records.read().await;
        Ok(records
            .get(owner_pubkey)
            .and_then(|owner| owner.get(event_id).cloned()))
    }

    async fn delete_request(&self, owner_pubkey: &str, event_id: &str) -> Result<(), AppError> {
        let mut records = self.records.write().await;
        if let Some(owner) = records.get_mut(owner_pubkey) {
            owner.remove(event_id);
        }
        Ok(())
    }

    async fn get_invite_usage(
        &self,
        owner_pubkey: &str,
        invite_event_id: &str,
    ) -> Result<Option<InviteUsageRecord>, AppError> {
        let records = self.invite_usage.read().await;
        Ok(records
            .get(owner_pubkey)
            .and_then(|owner| owner.get(invite_event_id).cloned()))
    }

    async fn upsert_invite_usage(
        &self,
        owner_pubkey: &str,
        record: InviteUsageRecord,
    ) -> Result<(), AppError> {
        let mut records = self.invite_usage.write().await;
        let owner = records.entry(owner_pubkey.to_string()).or_default();
        owner.insert(record.invite_event_id.clone(), record);
        Ok(())
    }
}

#[derive(Clone, Default)]
struct TestUserRepository {
    follows: Arc<RwLock<HashSet<(String, String)>>>,
}

impl TestUserRepository {
    async fn seed_follow(&self, follower: &str, followed: &str) {
        let mut follows = self.follows.write().await;
        follows.insert((follower.to_string(), followed.to_string()));
    }
}

#[async_trait]
impl UserRepository for TestUserRepository {
    async fn create_user(&self, _user: &User) -> Result<(), AppError> {
        Err(AppError::NotImplemented("create_user".into()))
    }

    async fn get_user(&self, _npub: &str) -> Result<Option<User>, AppError> {
        Err(AppError::NotImplemented("get_user".into()))
    }

    async fn get_user_by_pubkey(&self, _pubkey: &str) -> Result<Option<User>, AppError> {
        Err(AppError::NotImplemented("get_user_by_pubkey".into()))
    }

    async fn search_users(&self, _query: &str, _limit: usize) -> Result<Vec<User>, AppError> {
        Err(AppError::NotImplemented("search_users".into()))
    }

    async fn update_user(&self, _user: &User) -> Result<(), AppError> {
        Err(AppError::NotImplemented("update_user".into()))
    }

    async fn delete_user(&self, _npub: &str) -> Result<(), AppError> {
        Err(AppError::NotImplemented("delete_user".into()))
    }

    async fn get_followers_paginated(
        &self,
        _npub: &str,
        _cursor: Option<&str>,
        _limit: usize,
        _sort: FollowListSort,
        _search: Option<&str>,
    ) -> Result<UserCursorPage, AppError> {
        Err(AppError::NotImplemented("get_followers_paginated".into()))
    }

    async fn get_following_paginated(
        &self,
        _npub: &str,
        _cursor: Option<&str>,
        _limit: usize,
        _sort: FollowListSort,
        _search: Option<&str>,
    ) -> Result<UserCursorPage, AppError> {
        Err(AppError::NotImplemented("get_following_paginated".into()))
    }

    async fn add_follow_relation(
        &self,
        follower_pubkey: &str,
        followed_pubkey: &str,
    ) -> Result<bool, AppError> {
        let mut follows = self.follows.write().await;
        Ok(follows.insert((
            follower_pubkey.to_string(),
            followed_pubkey.to_string(),
        )))
    }

    async fn remove_follow_relation(
        &self,
        follower_pubkey: &str,
        followed_pubkey: &str,
    ) -> Result<bool, AppError> {
        let mut follows = self.follows.write().await;
        Ok(follows.remove(&(
            follower_pubkey.to_string(),
            followed_pubkey.to_string(),
        )))
    }

    async fn list_following_pubkeys(
        &self,
        follower_pubkey: &str,
    ) -> Result<Vec<String>, AppError> {
        let follows = self.follows.read().await;
        Ok(follows
            .iter()
            .filter_map(|(follower, followed)| {
                if follower == follower_pubkey {
                    Some(followed.clone())
                } else {
                    None
                }
            })
            .collect())
    }

    async fn list_follower_pubkeys(
        &self,
        followed_pubkey: &str,
    ) -> Result<Vec<String>, AppError> {
        let follows = self.follows.read().await;
        Ok(follows
            .iter()
            .filter_map(|(follower, followed)| {
                if followed == followed_pubkey {
                    Some(follower.clone())
                } else {
                    None
                }
            })
            .collect())
    }
}

#[derive(Clone, Default)]
struct TestGossipService {
    joined: Arc<RwLock<HashSet<String>>>,
    broadcasts: Arc<RwLock<Vec<(String, Event)>>>,
}

impl TestGossipService {
    async fn broadcasts(&self) -> Vec<(String, Event)> {
        self.broadcasts.read().await.clone()
    }
}

#[async_trait]
impl kukuri_lib::test_support::infrastructure::p2p::GossipService for TestGossipService {
    async fn join_topic(&self, topic: &str, _initial_peers: Vec<String>) -> Result<(), AppError> {
        self.joined.write().await.insert(topic.to_string());
        Ok(())
    }

    async fn leave_topic(&self, topic: &str) -> Result<(), AppError> {
        self.joined.write().await.remove(topic);
        Ok(())
    }

    async fn broadcast(&self, topic: &str, event: &Event) -> Result<(), AppError> {
        self.broadcasts
            .write()
            .await
            .push((topic.to_string(), event.clone()));
        Ok(())
    }

    async fn subscribe(
        &self,
        _topic: &str,
    ) -> Result<tokio::sync::mpsc::Receiver<Event>, AppError> {
        Err(AppError::NotImplemented("subscribe".into()))
    }

    async fn get_joined_topics(&self) -> Result<Vec<String>, AppError> {
        Ok(self.joined.read().await.iter().cloned().collect())
    }

    async fn get_topic_peers(&self, _topic: &str) -> Result<Vec<String>, AppError> {
        Err(AppError::NotImplemented("get_topic_peers".into()))
    }

    async fn get_topic_stats(
        &self,
        _topic: &str,
    ) -> Result<Option<kukuri_lib::test_support::domain::p2p::TopicStats>, AppError> {
        Ok(None)
    }

    async fn broadcast_message(&self, _topic: &str, _message: &[u8]) -> Result<(), AppError> {
        Ok(())
    }
}

#[derive(Default)]
struct TestPostCache {
    posts: Mutex<HashMap<String, Post>>,
}

#[async_trait]
impl PostCache for TestPostCache {
    async fn add(&self, post: Post) {
        self.posts.lock().await.insert(post.id.clone(), post);
    }

    async fn get(&self, id: &str) -> Option<Post> {
        self.posts.lock().await.get(id).cloned()
    }

    async fn remove(&self, id: &str) -> Option<Post> {
        self.posts.lock().await.remove(id)
    }

    async fn get_by_topic(&self, topic_id: &str, limit: usize) -> Vec<Post> {
        let posts = self.posts.lock().await;
        let mut filtered: Vec<Post> = posts
            .values()
            .filter(|post| post.topic_id == topic_id)
            .cloned()
            .collect();
        filtered.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        if limit == usize::MAX {
            return filtered;
        }
        filtered.into_iter().take(limit).collect()
    }

    async fn set_topic_posts(&self, topic_id: &str, posts: Vec<Post>) {
        let mut guard = self.posts.lock().await;
        guard.retain(|_, post| post.topic_id != topic_id);
        for post in posts {
            guard.insert(post.id.clone(), post);
        }
    }

    async fn invalidate_topic(&self, topic_id: &str) {
        let mut guard = self.posts.lock().await;
        guard.retain(|_, post| post.topic_id != topic_id);
    }
}

#[derive(Default)]
struct TestEventService;

#[async_trait]
impl EventServiceTrait for TestEventService {
    async fn initialize(&self) -> Result<(), AppError> {
        Ok(())
    }

    async fn publish_text_note(&self, _content: &str) -> Result<EventId, AppError> {
        Ok(EventId::generate())
    }

    async fn publish_topic_post(
        &self,
        _topic_id: &str,
        _content: &str,
        _reply_to: Option<&str>,
        _scope: Option<&str>,
        _epoch: Option<i64>,
    ) -> Result<EventId, AppError> {
        Ok(EventId::generate())
    }

    async fn send_reaction(&self, _event_id: &str, _reaction: &str) -> Result<EventId, AppError> {
        Ok(EventId::generate())
    }

    async fn update_metadata(
        &self,
        _metadata: kukuri_lib::test_support::presentation::dto::event::NostrMetadataDto,
    ) -> Result<EventId, AppError> {
        Ok(EventId::generate())
    }

    async fn subscribe_to_topic(&self, _topic_id: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn subscribe_to_user(&self, _pubkey: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn get_public_key(&self) -> Result<Option<String>, AppError> {
        Ok(None)
    }

    async fn boost_post(&self, _event_id: &str) -> Result<EventId, AppError> {
        Ok(EventId::generate())
    }

    async fn delete_events(
        &self,
        _event_ids: Vec<String>,
        _reason: Option<String>,
    ) -> Result<EventId, AppError> {
        Ok(EventId::generate())
    }

    async fn disconnect(&self) -> Result<(), AppError> {
        Ok(())
    }

    async fn set_default_p2p_topic(&self, _topic_id: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn list_subscriptions(
        &self,
    ) -> Result<Vec<kukuri_lib::test_support::application::services::SubscriptionRecord>, AppError>
    {
        Ok(vec![])
    }
}

fn make_keypair() -> KeyPair {
    let keys = Keys::generate();
    let public_key = keys.public_key().to_string();
    let private_key = keys.secret_key().display_secret().to_string();
    let npub = keys
        .public_key()
        .to_bech32()
        .unwrap_or_else(|_| public_key.clone());
    let nsec = format!("nsec1{private_key}");
    KeyPair {
        public_key,
        private_key,
        npub,
        nsec,
    }
}

async fn setup_post_service_with_group_store(
    group_key_store: Arc<dyn GroupKeyStore>,
    event_service: Arc<dyn EventServiceTrait>,
) -> (PostService, Arc<SqliteRepository>) {
    let pool = ConnectionPool::new("sqlite::memory:?cache=shared")
        .await
        .expect("failed to create pool");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS bookmarks (
            id TEXT PRIMARY KEY,
            user_pubkey TEXT NOT NULL,
            post_id TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            UNIQUE(user_pubkey, post_id)
        )
        "#,
    )
    .execute(pool.get_pool())
    .await
    .expect("failed to create bookmarks table");

    let repository = Arc::new(SqliteRepository::new(pool));
    repository
        .initialize()
        .await
        .expect("failed to initialize repository schema");

    let cache: Arc<dyn PostCache> = Arc::new(TestPostCache::default());

    let service = PostService::new(
        Arc::clone(&repository) as Arc<dyn PostRepository>,
        Arc::clone(&repository) as Arc<dyn BookmarkRepository>,
        event_service,
        cache,
        group_key_store,
    );

    (service, repository)
}

#[tokio::test]
async fn p2p_only_invite_join_key_envelope_encrypted_post_flow() {
    let inviter_keypair = make_keypair();
    let requester_keypair = make_keypair();

    let inviter_key_manager = Arc::new(TestKeyManager::new(inviter_keypair.clone()));
    let requester_key_manager = Arc::new(TestKeyManager::new(requester_keypair.clone()));

    let inviter_group_keys = Arc::new(TestGroupKeyStore::default());
    let requester_group_keys = Arc::new(TestGroupKeyStore::default());
    let inviter_join_requests = Arc::new(TestJoinRequestStore::default());
    let requester_join_requests = Arc::new(TestJoinRequestStore::default());
    let user_repository = Arc::new(TestUserRepository::default());

    let signature_service = Arc::new(DefaultSignatureService::new());
    let inviter_gossip = Arc::new(TestGossipService::default());
    let requester_gossip = Arc::new(TestGossipService::default());

    let inviter_service = AccessControlService::new(
        inviter_key_manager,
        Arc::clone(&inviter_group_keys) as Arc<dyn GroupKeyStore>,
        Arc::clone(&inviter_join_requests) as Arc<dyn JoinRequestStore>,
        Arc::clone(&user_repository) as Arc<dyn UserRepository>,
        Arc::clone(&signature_service),
        inviter_gossip.clone(),
    );
    let requester_service = AccessControlService::new(
        requester_key_manager,
        Arc::clone(&requester_group_keys) as Arc<dyn GroupKeyStore>,
        Arc::clone(&requester_join_requests) as Arc<dyn JoinRequestStore>,
        Arc::clone(&user_repository) as Arc<dyn UserRepository>,
        Arc::clone(&signature_service),
        requester_gossip.clone(),
    );

    let topic_id = "kukuri:topic-invite";
    let invite_json = inviter_service
        .issue_invite(topic_id, Some(900), Some(1), Some("p2p-e2e".into()))
        .await
        .expect("issue invite");

    let join_result = requester_service
        .request_join(JoinRequestInput {
            topic_id: None,
            scope: None,
            invite_event_json: Some(invite_json.clone()),
            target_pubkey: None,
            broadcast_to_topic: false,
        })
        .await
        .expect("request join");

    let inviter_topic = user_topic_id(&inviter_keypair.public_key);
    assert!(
        join_result.sent_topics.contains(&inviter_topic),
        "join request should target inviter topic"
    );

    let requester_broadcasts = requester_gossip.broadcasts().await;
    let join_event = requester_broadcasts
        .iter()
        .find(|(_, event)| event.kind == 39022)
        .map(|(_, event)| event.clone())
        .expect("join request event broadcasted");

    inviter_service
        .handle_incoming_event(&join_event)
        .await
        .expect("inviter handles join request");

    let pending = inviter_service
        .list_pending_join_requests()
        .await
        .expect("pending join requests");
    assert_eq!(pending.len(), 1);

    inviter_service
        .approve_join_request(&join_event.id)
        .await
        .expect("approve join request");
    let pending_after = inviter_service
        .list_pending_join_requests()
        .await
        .expect("pending cleared");
    assert!(pending_after.is_empty());

    let inviter_broadcasts = inviter_gossip.broadcasts().await;
    let key_envelope_event = inviter_broadcasts
        .iter()
        .find(|(_, event)| event.kind == 39020)
        .map(|(_, event)| event.clone())
        .expect("key envelope broadcasted");

    requester_service
        .handle_incoming_event(&key_envelope_event)
        .await
        .expect("requester stores key envelope");

    let stored_key = requester_group_keys
        .get_latest_key(topic_id, "invite")
        .await
        .expect("load key")
        .expect("invite key stored");
    assert_eq!(stored_key.scope, "invite");

    let event_service: Arc<dyn EventServiceTrait> = Arc::new(TestEventService::default());
    let (post_service, repository) = setup_post_service_with_group_store(
        Arc::clone(&requester_group_keys) as Arc<dyn GroupKeyStore>,
        event_service,
    )
    .await;

    let author = User::new(requester_keypair.npub.clone(), requester_keypair.public_key.clone());
    let created = post_service
        .create_post(
            "p2p invite encrypted post".into(),
            author,
            topic_id.to_string(),
            Some("invite".into()),
        )
        .await
        .expect("create encrypted post");

    assert!(created.is_encrypted);
    assert_eq!(created.scope.as_deref(), Some("invite"));
    assert_eq!(created.epoch, Some(stored_key.epoch));
    assert_eq!(created.content, "p2p invite encrypted post");

    let stored = repository
        .get_post(&created.id)
        .await
        .expect("fetch stored post")
        .expect("stored post exists");
    assert_ne!(stored.content, "p2p invite encrypted post");
    let payload =
        EncryptedPostPayload::try_parse(&stored.content).expect("encrypted payload parse");
    assert_eq!(payload.scope, "invite");
    assert_eq!(payload.epoch, stored_key.epoch);
}

#[tokio::test]
async fn p2p_only_friend_plus_join_key_envelope_encrypted_post_flow() {
    let inviter_keypair = make_keypair();
    let requester_keypair = make_keypair();
    let friend_keypair = make_keypair();

    let inviter_key_manager = Arc::new(TestKeyManager::new(inviter_keypair.clone()));
    let requester_key_manager = Arc::new(TestKeyManager::new(requester_keypair.clone()));

    let inviter_group_keys = Arc::new(TestGroupKeyStore::default());
    let requester_group_keys = Arc::new(TestGroupKeyStore::default());
    let inviter_join_requests = Arc::new(TestJoinRequestStore::default());
    let requester_join_requests = Arc::new(TestJoinRequestStore::default());
    let user_repository = Arc::new(TestUserRepository::default());

    user_repository
        .seed_follow(&inviter_keypair.public_key, &friend_keypair.public_key)
        .await;
    user_repository
        .seed_follow(&friend_keypair.public_key, &inviter_keypair.public_key)
        .await;
    user_repository
        .seed_follow(&friend_keypair.public_key, &requester_keypair.public_key)
        .await;
    user_repository
        .seed_follow(&requester_keypair.public_key, &friend_keypair.public_key)
        .await;

    let signature_service = Arc::new(DefaultSignatureService::new());
    let inviter_gossip = Arc::new(TestGossipService::default());
    let requester_gossip = Arc::new(TestGossipService::default());

    let inviter_service = AccessControlService::new(
        inviter_key_manager,
        Arc::clone(&inviter_group_keys) as Arc<dyn GroupKeyStore>,
        Arc::clone(&inviter_join_requests) as Arc<dyn JoinRequestStore>,
        Arc::clone(&user_repository) as Arc<dyn UserRepository>,
        Arc::clone(&signature_service),
        inviter_gossip.clone(),
    );
    let requester_service = AccessControlService::new(
        requester_key_manager,
        Arc::clone(&requester_group_keys) as Arc<dyn GroupKeyStore>,
        Arc::clone(&requester_join_requests) as Arc<dyn JoinRequestStore>,
        Arc::clone(&user_repository) as Arc<dyn UserRepository>,
        Arc::clone(&signature_service),
        requester_gossip.clone(),
    );

    let topic_id = "kukuri:topic-friend-plus";

    let join_result = requester_service
        .request_join(JoinRequestInput {
            topic_id: Some(topic_id.to_string()),
            scope: Some("friend_plus".into()),
            invite_event_json: None,
            target_pubkey: Some(inviter_keypair.public_key.clone()),
            broadcast_to_topic: false,
        })
        .await
        .expect("request join");

    let inviter_topic = user_topic_id(&inviter_keypair.public_key);
    assert!(
        join_result.sent_topics.contains(&inviter_topic),
        "join request should target inviter topic"
    );

    let requester_broadcasts = requester_gossip.broadcasts().await;
    let join_event = requester_broadcasts
        .iter()
        .find(|(_, event)| event.kind == 39022)
        .map(|(_, event)| event.clone())
        .expect("join request event broadcasted");

    inviter_service
        .handle_incoming_event(&join_event)
        .await
        .expect("inviter handles join request");

    let pending = inviter_service
        .list_pending_join_requests()
        .await
        .expect("pending join requests");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].scope, "friend_plus");

    inviter_service
        .approve_join_request(&join_event.id)
        .await
        .expect("approve join request");

    let inviter_broadcasts = inviter_gossip.broadcasts().await;
    let key_envelope_event = inviter_broadcasts
        .iter()
        .find(|(_, event)| event.kind == 39020)
        .map(|(_, event)| event.clone())
        .expect("key envelope broadcasted");

    requester_service
        .handle_incoming_event(&key_envelope_event)
        .await
        .expect("requester stores key envelope");

    let stored_key = requester_group_keys
        .get_latest_key(topic_id, "friend_plus")
        .await
        .expect("load key")
        .expect("friend_plus key stored");
    assert_eq!(stored_key.scope, "friend_plus");

    let event_service: Arc<dyn EventServiceTrait> = Arc::new(TestEventService::default());
    let (post_service, repository) = setup_post_service_with_group_store(
        Arc::clone(&requester_group_keys) as Arc<dyn GroupKeyStore>,
        event_service,
    )
    .await;

    let author = User::new(requester_keypair.npub.clone(), requester_keypair.public_key.clone());
    let created = post_service
        .create_post(
            "p2p friend_plus encrypted post".into(),
            author,
            topic_id.to_string(),
            Some("friend_plus".into()),
        )
        .await
        .expect("create encrypted post");

    assert!(created.is_encrypted);
    assert_eq!(created.scope.as_deref(), Some("friend_plus"));
    assert_eq!(created.epoch, Some(stored_key.epoch));
    assert_eq!(created.content, "p2p friend_plus encrypted post");

    let stored = repository
        .get_post(&created.id)
        .await
        .expect("fetch stored post")
        .expect("stored post exists");
    assert_ne!(stored.content, "p2p friend_plus encrypted post");
    let payload =
        EncryptedPostPayload::try_parse(&stored.content).expect("encrypted payload parse");
    assert_eq!(payload.scope, "friend_plus");
    assert_eq!(payload.epoch, stored_key.epoch);
}
