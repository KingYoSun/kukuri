use async_trait::async_trait;
use kukuri_lib::application::ports::cache::PostCache;
use kukuri_lib::application::ports::repositories::{
    BookmarkRepository, PostRepository, UserRepository,
};
use kukuri_lib::application::services::event_service::EventServiceTrait;
use kukuri_lib::application::services::post_service::PostService;
use kukuri_lib::application::services::user_service::UserService;
use kukuri_lib::application::services::SubscriptionRecord;
use kukuri_lib::domain::value_objects::EventId;
use kukuri_lib::infrastructure::cache::PostCacheService;
use kukuri_lib::infrastructure::database::connection_pool::ConnectionPool;
use kukuri_lib::infrastructure::database::sqlite_repository::SqliteRepository;
use kukuri_lib::presentation::dto::event::NostrMetadataDto;
use kukuri_lib::shared::error::AppError;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Default)]
struct RecordingEventService {
    deleted_event_ids: Mutex<Vec<String>>,
}

impl RecordingEventService {
    async fn collected_ids(&self) -> Vec<String> {
        self.deleted_event_ids.lock().await.clone()
    }
}

#[async_trait]
impl EventServiceTrait for RecordingEventService {
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
    ) -> Result<EventId, AppError> {
        Ok(EventId::generate())
    }

    async fn send_reaction(&self, _event_id: &str, _reaction: &str) -> Result<EventId, AppError> {
        Ok(EventId::generate())
    }

    async fn update_metadata(&self, _metadata: NostrMetadataDto) -> Result<EventId, AppError> {
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
        event_ids: Vec<String>,
        _reason: Option<String>,
    ) -> Result<EventId, AppError> {
        let mut guard = self.deleted_event_ids.lock().await;
        guard.extend(event_ids);
        Ok(EventId::generate())
    }

    async fn disconnect(&self) -> Result<(), AppError> {
        Ok(())
    }

    async fn set_default_p2p_topic(&self, _topic_id: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn list_subscriptions(&self) -> Result<Vec<SubscriptionRecord>, AppError> {
        Ok(vec![])
    }
}

const FOLLOWER_NPUB: &str =
    "npub1postdeleteflowfollower00000000000000000000000000000000000000000000000000000";
const FOLLOWER_PUBKEY: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
const AUTHOR_NPUB: &str =
    "npub1postdeleteflowauthor0000000000000000000000000000000000000000000000000000000";
const AUTHOR_PUBKEY: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[tokio::test]
async fn delete_post_is_removed_from_following_and_topic_feeds() {
    let (post_service, cache, event_service, repository) = setup_post_service().await;

    let user_service = UserService::new(Arc::clone(&repository) as Arc<dyn UserRepository>);

    let follower = user_service
        .create_user(FOLLOWER_NPUB.to_string(), FOLLOWER_PUBKEY.to_string())
        .await
        .expect("create follower");
    let author = user_service
        .create_user(AUTHOR_NPUB.to_string(), AUTHOR_PUBKEY.to_string())
        .await
        .expect("create author");

    user_service
        .follow_user(&follower.npub, &author.npub)
        .await
        .expect("establish follow relationship");

    let topic_id = "post-delete-flow-topic";
    let first_post = post_service
        .create_post("first post".into(), author.clone(), topic_id.to_string())
        .await
        .expect("create first post");
    let second_post = post_service
        .create_post("second post".into(), author.clone(), topic_id.to_string())
        .await
        .expect("create second post");

    let initial_feed = post_service
        .list_following_feed(&follower.pubkey, None, 10)
        .await
        .expect("initial feed");
    assert_eq!(initial_feed.items.len(), 2);
    assert_eq!(initial_feed.items[0].id, second_post.id);
    assert_eq!(initial_feed.items[1].id, first_post.id);

    let topic_posts = post_service
        .get_posts_by_topic(topic_id, 10)
        .await
        .expect("topic posts before deletion");
    assert_eq!(topic_posts.len(), 2);

    post_service
        .delete_post(&first_post.id)
        .await
        .expect("delete post");

    let updated_feed = post_service
        .list_following_feed(&follower.pubkey, None, 10)
        .await
        .expect("feed after delete");
    assert_eq!(updated_feed.items.len(), 1);
    assert_eq!(updated_feed.items[0].id, second_post.id);
    assert!(
        updated_feed
            .items
            .iter()
            .all(|post| post.id != first_post.id),
        "deleted post should not appear in following feed"
    );

    let refreshed_topic_posts = post_service
        .get_posts_by_topic(topic_id, 10)
        .await
        .expect("topic posts after deletion");
    assert_eq!(refreshed_topic_posts.len(), 1);
    assert_eq!(refreshed_topic_posts[0].id, second_post.id);

    assert!(
        cache.get(&first_post.id).await.is_none(),
        "cache entry should be cleared when the post is deleted"
    );

    let deleted_ids = event_service.collected_ids().await;
    assert!(
        deleted_ids.contains(&first_post.id),
        "delete_post should call EventService::delete_events with the removed post id"
    );
}

async fn setup_post_service(
) -> (
    Arc<PostService>,
    Arc<PostCacheService>,
    Arc<RecordingEventService>,
    Arc<SqliteRepository>,
) {
    let pool = ConnectionPool::new("sqlite::memory:?cache=shared")
        .await
        .expect("create pool");

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
    .expect("create bookmarks table");

    let repository = Arc::new(SqliteRepository::new(pool));
    repository
        .initialize()
        .await
        .expect("initialize schema");

    let cache = Arc::new(PostCacheService::new());
    let event_service = Arc::new(RecordingEventService::default());

    let post_service = Arc::new(PostService::new(
        Arc::clone(&repository) as Arc<dyn PostRepository>,
        Arc::clone(&repository) as Arc<dyn BookmarkRepository>,
        Arc::clone(&event_service) as Arc<dyn EventServiceTrait>,
        Arc::clone(&cache) as Arc<dyn PostCache>,
    ));

    (post_service, cache, event_service, repository)
}
