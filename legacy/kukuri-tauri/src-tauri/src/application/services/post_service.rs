use crate::application::ports::cache::PostCache;
use crate::application::ports::group_key_store::GroupKeyStore;
use crate::application::ports::repositories::{BookmarkRepository, PostFeedCursor, PostRepository};
use crate::application::services::event_service::EventServiceTrait;
use crate::domain::entities::{Post, User};
use crate::domain::value_objects::{EncryptedPostPayload, EventId, PublicKey};
use crate::shared::{AppError, ValidationFailureKind};
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::Utc;
use nostr_sdk::prelude::nip44;
use nostr_sdk::prelude::nip44::v2::ConversationKey;
use std::collections::HashSet;
use std::sync::Arc;
use tracing::warn;
use uuid::Uuid;

const ENCRYPTED_PLACEHOLDER: &str = "[Encrypted post]";
const PRIVATE_SCOPES: [&str; 3] = ["friend", "friend_plus", "invite"];

pub struct PostService {
    repository: Arc<dyn PostRepository>,
    bookmark_repository: Arc<dyn BookmarkRepository>,
    event_service: Arc<dyn EventServiceTrait>,
    cache: Arc<dyn PostCache>,
    group_key_store: Arc<dyn GroupKeyStore>,
}

#[derive(Debug, Clone)]
pub struct FollowingFeedPage {
    pub items: Vec<Post>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
    pub server_time: i64,
}

#[derive(Debug, Clone)]
pub struct TopicTimelineEntry {
    pub thread_uuid: String,
    pub parent_post: Post,
    pub first_reply: Option<Post>,
    pub reply_count: u32,
    pub last_activity_at: i64,
}

impl PostService {
    fn normalize_thread_uuid(thread_uuid: &str) -> Result<String, AppError> {
        let normalized = thread_uuid.trim();
        if normalized.is_empty() {
            return Err(AppError::validation(
                ValidationFailureKind::Generic,
                "thread_uuid is required",
            ));
        }

        Uuid::parse_str(normalized)
            .map(|id| id.to_string())
            .map_err(|err| {
                AppError::validation(
                    ValidationFailureKind::Generic,
                    format!("Invalid thread_uuid: {err}"),
                )
            })
    }

    fn build_thread_namespace(topic_id: &str, thread_uuid: &str) -> String {
        format!("{topic_id}/threads/{thread_uuid}")
    }

    fn normalize_scope(scope: Option<String>) -> Result<Option<String>, AppError> {
        let scope = scope
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let Some(value) = scope.as_deref() else {
            return Ok(None);
        };
        if value == "public" {
            return Ok(None);
        }
        if PRIVATE_SCOPES.contains(&value) {
            return Ok(Some(value.to_string()));
        }
        Err(AppError::validation(
            ValidationFailureKind::Generic,
            format!("Invalid scope: {value}"),
        ))
    }

    fn normalize_publish_reply_event_id(candidate: &str) -> Option<String> {
        let normalized = candidate.trim();
        if normalized.is_empty() {
            return None;
        }
        EventId::from_hex(normalized)
            .ok()
            .map(|_| normalized.to_string())
    }

    async fn resolve_publish_reply_to(
        &self,
        reply_to: Option<&str>,
    ) -> Result<Option<String>, AppError> {
        let Some(reply_to) = reply_to else {
            return Ok(None);
        };
        let reply_to = reply_to.trim();
        if reply_to.is_empty() {
            return Ok(None);
        }

        if let Some(event_id) = Self::normalize_publish_reply_event_id(reply_to) {
            return Ok(Some(event_id));
        }

        let synced_event_id = self.repository.get_sync_event_id(reply_to).await?;
        Ok(synced_event_id
            .as_deref()
            .and_then(Self::normalize_publish_reply_event_id))
    }

    fn conversation_key_from_b64(key_b64: &str) -> Result<ConversationKey, AppError> {
        let bytes = STANDARD
            .decode(key_b64)
            .map_err(|err| AppError::Crypto(format!("Invalid group key: {err}")))?;
        ConversationKey::from_slice(&bytes)
            .map_err(|err| AppError::Crypto(format!("Invalid group key: {err}")))
    }

    fn encrypt_with_group_key(key_b64: &str, plaintext: &str) -> Result<String, AppError> {
        let conversation_key = Self::conversation_key_from_b64(key_b64)?;
        let payload = nip44::v2::encrypt_to_bytes(&conversation_key, plaintext.as_bytes())
            .map_err(|err| AppError::Crypto(format!("Encrypt failed: {err}")))?;
        Ok(STANDARD.encode(payload))
    }

    fn decrypt_with_group_key(key_b64: &str, payload_b64: &str) -> Result<String, AppError> {
        let conversation_key = Self::conversation_key_from_b64(key_b64)?;
        let payload = STANDARD
            .decode(payload_b64)
            .map_err(|err| AppError::Crypto(format!("Invalid payload: {err}")))?;
        let decrypted = nip44::v2::decrypt_to_bytes(&conversation_key, &payload)
            .map_err(|err| AppError::Crypto(format!("Decrypt failed: {err}")))?;
        String::from_utf8(decrypted)
            .map_err(|err| AppError::Crypto(format!("Decrypt failed: {err}")))
    }

    async fn encrypt_post_content(
        &self,
        content: &str,
        topic_id: &str,
        scope: &str,
    ) -> Result<(String, i64), AppError> {
        let record = self
            .group_key_store
            .get_latest_key(topic_id, scope)
            .await?
            .ok_or_else(|| {
                AppError::validation(
                    ValidationFailureKind::Generic,
                    format!("Group key not found for {topic_id}:{scope}"),
                )
            })?;
        let payload_b64 = Self::encrypt_with_group_key(&record.key_b64, content)?;
        let payload = EncryptedPostPayload::new(
            topic_id.to_string(),
            scope.to_string(),
            record.epoch,
            payload_b64,
        );
        let json = serde_json::to_string(&payload)
            .map_err(|err| AppError::SerializationError(err.to_string()))?;
        Ok((json, record.epoch))
    }

    async fn prepare_post(&self, mut post: Post) -> Result<Post, AppError> {
        let Some(payload) = EncryptedPostPayload::try_parse(&post.content) else {
            return Ok(post);
        };
        post.is_encrypted = true;
        if post.scope.is_none() {
            post.scope = Some(payload.scope.clone());
        }
        if post.epoch.is_none() {
            post.epoch = Some(payload.epoch);
        }

        let record = self
            .group_key_store
            .get_key(&payload.topic, &payload.scope, payload.epoch)
            .await?;
        let Some(record) = record else {
            post.content = ENCRYPTED_PLACEHOLDER.to_string();
            return Ok(post);
        };

        match Self::decrypt_with_group_key(&record.key_b64, &payload.payload_b64) {
            Ok(content) => {
                post.content = content;
            }
            Err(_) => {
                post.content = ENCRYPTED_PLACEHOLDER.to_string();
            }
        }

        Ok(post)
    }

    async fn prepare_posts(&self, posts: Vec<Post>) -> Result<Vec<Post>, AppError> {
        let mut prepared = Vec::with_capacity(posts.len());
        for post in posts {
            prepared.push(self.prepare_post(post).await?);
        }
        Ok(prepared)
    }

    pub fn new(
        repository: Arc<dyn PostRepository>,
        bookmark_repository: Arc<dyn BookmarkRepository>,
        event_service: Arc<dyn EventServiceTrait>,
        cache: Arc<dyn PostCache>,
        group_key_store: Arc<dyn GroupKeyStore>,
    ) -> Self {
        Self {
            repository,
            bookmark_repository,
            event_service,
            cache,
            group_key_store,
        }
    }

    pub async fn create_post(
        &self,
        content: String,
        author: User,
        topic_id: String,
        thread_uuid: String,
        reply_to: Option<String>,
        scope: Option<String>,
    ) -> Result<Post, AppError> {
        let scope = Self::normalize_scope(scope)?;
        let thread_uuid = Self::normalize_thread_uuid(&thread_uuid)?;
        let mut post = Post::new(content.clone(), author, topic_id.clone());
        let thread_namespace = Self::build_thread_namespace(&topic_id, &thread_uuid);

        let (thread_root_event_id, thread_parent_event_id) = if let Some(reply_to) =
            reply_to.as_ref()
        {
            let parent_thread = self
                .repository
                .get_event_thread(&topic_id, reply_to)
                .await?
                .ok_or_else(|| {
                    AppError::validation(
                        ValidationFailureKind::Generic,
                        format!(
                            "Parent post thread metadata not found for topic={topic_id} event={reply_to}"
                        ),
                    )
                })?;
            if parent_thread.thread_uuid != thread_uuid {
                return Err(AppError::validation(
                    ValidationFailureKind::Generic,
                    format!(
                        "thread_uuid mismatch: parent thread_uuid={} request thread_uuid={thread_uuid}",
                        parent_thread.thread_uuid
                    ),
                ));
            }
            (parent_thread.root_event_id, Some(reply_to.clone()))
        } else {
            (post.id.clone(), None)
        };

        post.thread_namespace = Some(thread_namespace);
        post.thread_uuid = Some(thread_uuid);
        post.thread_root_event_id = Some(thread_root_event_id);
        post.thread_parent_event_id = thread_parent_event_id;

        if let Some(ref scope_value) = scope {
            let (encrypted_content, epoch) = self
                .encrypt_post_content(&content, &topic_id, scope_value)
                .await?;
            post.content = encrypted_content;
            post.scope = Some(scope_value.clone());
            post.epoch = Some(epoch);
            post.is_encrypted = true;
        }

        self.repository.create_post(&post).await?;
        let publish_reply_to = self
            .resolve_publish_reply_to(post.thread_parent_event_id.as_deref())
            .await?;

        match self
            .event_service
            .publish_topic_post(
                &topic_id,
                &post.content,
                publish_reply_to.as_deref(),
                post.scope.as_deref(),
                post.epoch,
            )
            .await
        {
            Ok(event_id) => {
                let event_hex = event_id.to_string();
                post.mark_as_synced(event_hex.clone());
                self.repository
                    .mark_post_synced(&post.id, &event_hex)
                    .await?;
            }
            Err(err) => {
                self.cache.add(post.clone()).await;
                return Err(err);
            }
        }

        let prepared = self.prepare_post(post.clone()).await?;
        self.cache.add(prepared.clone()).await;
        Ok(prepared)
    }

    pub async fn get_post(&self, id: &str) -> Result<Option<Post>, AppError> {
        // キャッシュから取得を試みる
        if let Some(post) = self.cache.get(id).await {
            return Ok(Some(self.prepare_post(post).await?));
        }

        // キャッシュにない場合はDBから取得
        let post = self.repository.get_post(id).await?;

        // キャッシュに保存
        if let Some(post) = post {
            let prepared = self.prepare_post(post).await?;
            self.cache.add(prepared.clone()).await;
            return Ok(Some(prepared));
        }

        Ok(None)
    }

    pub async fn get_posts_by_topic(
        &self,
        topic_id: &str,
        limit: usize,
    ) -> Result<Vec<Post>, AppError> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let cached_all = self.cache.get_by_topic(topic_id, usize::MAX).await;
        if cached_all.len() >= limit {
            return Ok(cached_all.into_iter().take(limit).collect());
        }

        let mut posts = self.repository.get_posts_by_topic(topic_id, limit).await?;

        if !cached_all.is_empty() {
            let mut seen: HashSet<String> = posts.iter().map(|post| post.id.clone()).collect();
            for cached in cached_all {
                if seen.insert(cached.id.clone()) {
                    posts.push(cached);
                }
            }
        }

        posts.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let prepared = self.prepare_posts(posts).await?;

        // キャッシュにも最新の取得結果を反映
        self.cache.set_topic_posts(topic_id, prepared.clone()).await;

        Ok(prepared.into_iter().take(limit).collect())
    }

    pub async fn get_thread_posts(
        &self,
        topic_id: &str,
        thread_uuid: &str,
        limit: usize,
    ) -> Result<Vec<Post>, AppError> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let thread_uuid = Self::normalize_thread_uuid(thread_uuid)?;
        let posts = self
            .repository
            .get_posts_by_thread(topic_id, &thread_uuid, limit)
            .await?;
        self.prepare_posts(posts).await
    }

    pub async fn get_topic_timeline(
        &self,
        topic_id: &str,
        limit: usize,
    ) -> Result<Vec<TopicTimelineEntry>, AppError> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let summaries = self.repository.get_topic_timeline(topic_id, limit).await?;
        let mut entries = Vec::with_capacity(summaries.len());

        for summary in summaries {
            let Some(parent_post) = self.repository.get_post(&summary.root_event_id).await? else {
                continue;
            };
            let parent_post = self.prepare_post(parent_post).await?;
            if parent_post.topic_id != topic_id {
                continue;
            }

            let first_reply = if let Some(first_reply_event_id) = summary.first_reply_event_id {
                if let Some(post) = self.repository.get_post(&first_reply_event_id).await? {
                    let prepared = self.prepare_post(post).await?;
                    if prepared.topic_id == topic_id {
                        Some(prepared)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            entries.push(TopicTimelineEntry {
                thread_uuid: summary.thread_uuid,
                parent_post,
                first_reply,
                reply_count: summary.reply_count,
                last_activity_at: summary.last_activity_at / 1000,
            });
        }

        Ok(entries)
    }

    pub async fn like_post(&self, post_id: &str) -> Result<(), AppError> {
        self.react_to_post(post_id, "+").await
    }

    pub async fn boost_post(&self, post_id: &str) -> Result<(), AppError> {
        self.event_service.boost_post(post_id).await?;

        if let Some(mut post) = self.repository.get_post(post_id).await? {
            post.increment_boosts();
            self.repository.update_post(&post).await?;
            self.cache.remove(post_id).await;
        }

        Ok(())
    }

    pub async fn delete_post(&self, id: &str) -> Result<(), AppError> {
        self.event_service
            .delete_events(vec![id.to_string()], Some("Post deleted".to_string()))
            .await?;
        self.repository.delete_post(id).await?;
        self.cache.remove(id).await;
        Ok(())
    }

    pub async fn get_posts_by_author(
        &self,
        author_pubkey: &str,
        limit: usize,
    ) -> Result<Vec<Post>, AppError> {
        let posts = self
            .repository
            .get_posts_by_author(author_pubkey, limit)
            .await?;
        self.prepare_posts(posts).await
    }

    pub async fn get_recent_posts(&self, limit: usize) -> Result<Vec<Post>, AppError> {
        let posts = self.repository.get_recent_posts(limit).await?;
        self.prepare_posts(posts).await
    }

    pub async fn list_following_feed(
        &self,
        follower_pubkey: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<FollowingFeedPage, AppError> {
        let limit = limit.clamp(1, 100);
        let parsed_cursor = cursor.and_then(PostFeedCursor::parse);
        let page = self
            .repository
            .list_following_feed(follower_pubkey, parsed_cursor, limit)
            .await?;
        let items = self.prepare_posts(page.items).await?;

        Ok(FollowingFeedPage {
            items,
            next_cursor: page.next_cursor,
            has_more: page.has_more,
            server_time: Utc::now().timestamp_millis(),
        })
    }

    pub async fn react_to_post(&self, post_id: &str, reaction: &str) -> Result<(), AppError> {
        self.event_service.send_reaction(post_id, reaction).await?;

        if reaction == "+"
            && let Some(mut post) = self.repository.get_post(post_id).await?
        {
            post.increment_likes();
            self.repository.update_post(&post).await?;
            self.cache.remove(post_id).await;
        }

        Ok(())
    }

    pub async fn bookmark_post(&self, post_id: &str, user_pubkey: &str) -> Result<(), AppError> {
        let event_id = EventId::from_hex(post_id)
            .map_err(|err| AppError::validation(ValidationFailureKind::Generic, err))?;
        let public_key = PublicKey::from_hex_str(user_pubkey)
            .map_err(|err| AppError::validation(ValidationFailureKind::Generic, err))?;

        self.bookmark_repository
            .create_bookmark(&public_key, &event_id)
            .await?;
        // キャッシュを無効化して次回取得時に最新状態を反映
        self.cache.remove(post_id).await;
        Ok(())
    }

    pub async fn unbookmark_post(&self, post_id: &str, user_pubkey: &str) -> Result<(), AppError> {
        let event_id = EventId::from_hex(post_id)
            .map_err(|err| AppError::validation(ValidationFailureKind::Generic, err))?;
        let public_key = PublicKey::from_hex_str(user_pubkey)
            .map_err(|err| AppError::validation(ValidationFailureKind::Generic, err))?;

        self.bookmark_repository
            .delete_bookmark(&public_key, &event_id)
            .await?;
        // キャッシュを無効化して次回取得時に最新状態を反映
        self.cache.remove(post_id).await;
        Ok(())
    }

    pub async fn get_bookmarked_post_ids(
        &self,
        user_pubkey: &str,
    ) -> Result<Vec<String>, AppError> {
        let public_key = PublicKey::from_hex_str(user_pubkey)
            .map_err(|err| AppError::validation(ValidationFailureKind::Generic, err))?;

        let bookmarks = self.bookmark_repository.list_bookmarks(&public_key).await?;

        Ok(bookmarks
            .into_iter()
            .map(|bookmark| bookmark.post_id().as_str().to_string())
            .collect())
    }

    pub async fn sync_pending_posts(&self) -> Result<u32, AppError> {
        let unsynced_posts = self.repository.get_unsync_posts().await?;
        let mut synced_count = 0;

        for mut post in unsynced_posts {
            let publish_reply_to = self
                .resolve_publish_reply_to(post.thread_parent_event_id.as_deref())
                .await?;
            match self
                .event_service
                .publish_topic_post(
                    &post.topic_id,
                    &post.content,
                    publish_reply_to.as_deref(),
                    post.scope.as_deref(),
                    post.epoch,
                )
                .await
            {
                Ok(event_id) => {
                    let event_hex = event_id.to_string();
                    post.mark_as_synced(event_hex.clone());
                    self.repository
                        .mark_post_synced(&post.id, &event_hex)
                        .await?;
                    synced_count += 1;
                }
                Err(err) => {
                    warn!("failed to sync post {post_id}: {err}", post_id = post.id);
                }
            }
        }

        Ok(synced_count)
    }
}

#[async_trait]
impl super::sync_service::SyncParticipant for PostService {
    async fn sync_pending(&self) -> Result<u32, AppError> {
        self.sync_pending_posts().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::cache::PostCache;
    use crate::application::ports::group_key_store::{GroupKeyEntry, GroupKeyRecord};
    use crate::application::ports::repositories::{
        BookmarkRepository, PostRepository, UserRepository,
    };
    use crate::application::services::SubscriptionRecord;
    use crate::infrastructure::cache::PostCacheService;
    use crate::infrastructure::database::Repository;
    use crate::infrastructure::database::{
        connection_pool::ConnectionPool, sqlite_repository::SqliteRepository,
    };
    use crate::presentation::dto::event::NostrMetadataDto;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Mutex;
    use tokio::time::sleep;

    struct TestGroupKeyStore {
        records: Mutex<Vec<GroupKeyRecord>>,
    }

    impl TestGroupKeyStore {
        fn new() -> Self {
            Self {
                records: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl GroupKeyStore for TestGroupKeyStore {
        async fn store_key(&self, record: GroupKeyRecord) -> Result<(), AppError> {
            let mut guard = self.records.lock().await;
            if let Some(existing) = guard.iter_mut().find(|entry| {
                entry.topic_id == record.topic_id
                    && entry.scope == record.scope
                    && entry.epoch == record.epoch
            }) {
                *existing = record;
            } else {
                guard.push(record);
            }
            Ok(())
        }

        async fn get_key(
            &self,
            topic_id: &str,
            scope: &str,
            epoch: i64,
        ) -> Result<Option<GroupKeyRecord>, AppError> {
            let guard = self.records.lock().await;
            Ok(guard
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
            let guard = self.records.lock().await;
            Ok(guard
                .iter()
                .filter(|entry| entry.topic_id == topic_id && entry.scope == scope)
                .max_by_key(|entry| entry.epoch)
                .cloned())
        }

        async fn list_keys(&self) -> Result<Vec<GroupKeyEntry>, AppError> {
            let guard = self.records.lock().await;
            Ok(guard
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

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct PublishTopicPostCall {
        topic_id: String,
        content: String,
        reply_to: Option<String>,
        scope: Option<String>,
        epoch: Option<i64>,
    }

    struct TestEventService {
        publish_topic_post_result: Mutex<Option<Result<EventId, AppError>>>,
        publish_topic_post_calls: Mutex<Vec<PublishTopicPostCall>>,
    }

    impl TestEventService {
        fn new() -> Self {
            Self {
                publish_topic_post_result: Mutex::new(None),
                publish_topic_post_calls: Mutex::new(Vec::new()),
            }
        }

        fn with_publish_result(result: Result<EventId, AppError>) -> Self {
            Self {
                publish_topic_post_result: Mutex::new(Some(result)),
                publish_topic_post_calls: Mutex::new(Vec::new()),
            }
        }

        async fn next_publish_result(&self) -> Result<EventId, AppError> {
            let mut guard = self.publish_topic_post_result.lock().await;
            guard.take().unwrap_or_else(|| Ok(EventId::generate()))
        }

        async fn push_publish_call(&self, call: PublishTopicPostCall) {
            let mut guard = self.publish_topic_post_calls.lock().await;
            guard.push(call);
        }

        async fn publish_calls(&self) -> Vec<PublishTopicPostCall> {
            let guard = self.publish_topic_post_calls.lock().await;
            guard.clone()
        }
    }

    impl Default for TestEventService {
        fn default() -> Self {
            Self::new()
        }
    }

    #[async_trait]
    impl EventServiceTrait for TestEventService {
        async fn initialize(&self) -> Result<(), AppError> {
            Ok(())
        }
        async fn publish_text_note(&self, _: &str) -> Result<EventId, AppError> {
            Ok(EventId::generate())
        }
        async fn publish_topic_post(
            &self,
            topic_id: &str,
            content: &str,
            reply_to: Option<&str>,
            scope: Option<&str>,
            epoch: Option<i64>,
        ) -> Result<EventId, AppError> {
            self.push_publish_call(PublishTopicPostCall {
                topic_id: topic_id.to_string(),
                content: content.to_string(),
                reply_to: reply_to.map(|value| value.to_string()),
                scope: scope.map(|value| value.to_string()),
                epoch,
            })
            .await;

            if let Some(reply_to) = reply_to
                && EventId::from_hex(reply_to).is_err()
            {
                return Err(AppError::validation(
                    ValidationFailureKind::Generic,
                    "Invalid event ID: reply_to must be 64-char hex",
                ));
            }
            self.next_publish_result().await
        }
        async fn send_reaction(&self, _: &str, _: &str) -> Result<EventId, AppError> {
            Ok(EventId::generate())
        }
        async fn update_metadata(&self, _: NostrMetadataDto) -> Result<EventId, AppError> {
            Ok(EventId::generate())
        }
        async fn subscribe_to_topic(&self, _: &str) -> Result<(), AppError> {
            Ok(())
        }
        async fn subscribe_to_user(&self, _: &str) -> Result<(), AppError> {
            Ok(())
        }
        async fn get_public_key(&self) -> Result<Option<String>, AppError> {
            Ok(None)
        }
        async fn boost_post(&self, _: &str) -> Result<EventId, AppError> {
            Ok(EventId::generate())
        }
        async fn delete_events(
            &self,
            _: Vec<String>,
            _: Option<String>,
        ) -> Result<EventId, AppError> {
            Ok(EventId::generate())
        }
        async fn disconnect(&self) -> Result<(), AppError> {
            Ok(())
        }
        async fn set_default_p2p_topic(&self, _: &str) -> Result<(), AppError> {
            Ok(())
        }
        async fn list_subscriptions(&self) -> Result<Vec<SubscriptionRecord>, AppError> {
            Ok(vec![])
        }
    }

    async fn setup_post_service_with_deps(
        event_service: Arc<dyn EventServiceTrait>,
    ) -> (PostService, Arc<SqliteRepository>, Arc<PostCacheService>) {
        let (service, repository, cache, _group_key_store) =
            setup_post_service_with_group_store(event_service).await;

        (service, repository, cache)
    }

    async fn setup_post_service_with_group_store(
        event_service: Arc<dyn EventServiceTrait>,
    ) -> (
        PostService,
        Arc<SqliteRepository>,
        Arc<PostCacheService>,
        Arc<TestGroupKeyStore>,
    ) {
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
        let cache = Arc::new(PostCacheService::new());
        let group_key_store = Arc::new(TestGroupKeyStore::new());
        let group_key_store_trait: Arc<dyn GroupKeyStore> = group_key_store.clone();

        let service = PostService::new(
            Arc::clone(&repository) as Arc<dyn PostRepository>,
            Arc::clone(&repository) as Arc<dyn BookmarkRepository>,
            event_service,
            Arc::clone(&cache) as Arc<dyn PostCache>,
            group_key_store_trait,
        );

        (service, repository, cache, group_key_store)
    }

    async fn setup_post_service() -> PostService {
        let event_service: Arc<dyn EventServiceTrait> = Arc::new(TestEventService::default());
        setup_post_service_with_deps(event_service).await.0
    }

    fn sample_user() -> User {
        User {
            npub: "npub1test".to_string(),
            pubkey: "test_pubkey".to_string(),
            profile: crate::domain::entities::user::UserProfile {
                display_name: "Test User".to_string(),
                bio: "Test bio".to_string(),
                avatar_url: None,
            },
            name: Some("Test User".to_string()),
            nip05: None,
            lud16: None,
            public_profile: true,
            show_online_status: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    const SAMPLE_PUBKEY: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn sample_thread_uuid(index: u32) -> String {
        format!("00000000-0000-7000-8000-{index:012x}")
    }

    #[tokio::test]
    async fn bookmark_flow_roundtrip() {
        let service = setup_post_service().await;
        let event_id = EventId::generate();
        let event_hex = event_id.to_hex();

        service
            .bookmark_post(&event_hex, SAMPLE_PUBKEY)
            .await
            .expect("bookmark should succeed");

        let bookmarked = service
            .get_bookmarked_post_ids(SAMPLE_PUBKEY)
            .await
            .expect("list bookmarks");
        assert_eq!(bookmarked, vec![event_hex.clone()]);

        service
            .unbookmark_post(&event_hex, SAMPLE_PUBKEY)
            .await
            .expect("unbookmark should succeed");

        let bookmarked = service
            .get_bookmarked_post_ids(SAMPLE_PUBKEY)
            .await
            .expect("list bookmarks after removal");
        assert!(bookmarked.is_empty());
    }

    #[tokio::test]
    async fn create_post_marks_synced_after_publish() {
        let expected_event = EventId::generate();
        let event_service: Arc<dyn EventServiceTrait> = Arc::new(
            TestEventService::with_publish_result(Ok(expected_event.clone())),
        );

        let (service, repository, cache) = setup_post_service_with_deps(event_service).await;
        let expected_event_hex = expected_event.to_string();

        let post = service
            .create_post(
                "hello world".into(),
                sample_user(),
                "topic-sync".into(),
                sample_thread_uuid(1),
                None,
                None,
            )
            .await
            .expect("post creation succeeds");

        assert!(post.is_synced);
        assert_eq!(post.event_id.as_deref(), Some(expected_event_hex.as_str()));

        let stored = repository
            .get_post(&post.id)
            .await
            .expect("db query succeeds")
            .expect("post present in db");
        assert_eq!(stored.id, post.id);

        let unsynced = repository
            .get_unsync_posts()
            .await
            .expect("unsynced query succeeds");
        assert!(unsynced.is_empty(), "all posts should be marked synced");

        let cached = cache
            .get(&post.id)
            .await
            .expect("post cached after creation");
        assert_eq!(cached.event_id.as_deref(), post.event_id.as_deref());
    }

    #[tokio::test]
    async fn create_private_post_encrypts_and_decrypts() {
        let event_service: Arc<dyn EventServiceTrait> = Arc::new(TestEventService::default());
        let (service, repository, _cache, group_key_store) =
            setup_post_service_with_group_store(event_service).await;

        let key_b64 = STANDARD.encode([7u8; 32]);
        let record = GroupKeyRecord {
            topic_id: "topic-private".to_string(),
            scope: "friend".to_string(),
            epoch: 2,
            key_b64,
            stored_at: Utc::now().timestamp(),
        };
        group_key_store
            .store_key(record.clone())
            .await
            .expect("store group key");

        let post = service
            .create_post(
                "secret message".into(),
                sample_user(),
                "topic-private".into(),
                sample_thread_uuid(2),
                None,
                Some("friend".into()),
            )
            .await
            .expect("create private post");

        assert_eq!(post.content, "secret message");
        assert!(post.is_encrypted);
        assert_eq!(post.scope.as_deref(), Some("friend"));
        assert_eq!(post.epoch, Some(record.epoch));

        let stored = repository
            .get_post(&post.id)
            .await
            .expect("db fetch")
            .expect("stored post");
        assert_ne!(stored.content, "secret message");
        let payload =
            EncryptedPostPayload::try_parse(&stored.content).expect("encrypted payload parse");
        assert_eq!(payload.scope, "friend");
        assert_eq!(payload.epoch, record.epoch);
    }

    #[tokio::test]
    async fn create_post_caches_on_publish_failure() {
        let event_service: Arc<dyn EventServiceTrait> = Arc::new(
            TestEventService::with_publish_result(Err(AppError::NostrError("failed".into()))),
        );
        let (service, repository, cache) = setup_post_service_with_deps(event_service).await;

        let err = service
            .create_post(
                "offline".into(),
                sample_user(),
                "topic-offline".into(),
                sample_thread_uuid(3),
                None,
                None,
            )
            .await
            .expect_err("publish failure propagates");
        assert!(matches!(err, AppError::NostrError(_)));

        let stored = repository
            .get_posts_by_topic("topic-offline", 10)
            .await
            .expect("query succeeds");
        assert_eq!(stored.len(), 1);
        let stored_post = &stored[0];
        assert!(!stored_post.is_synced);
        assert!(stored_post.event_id.is_none());

        let unsynced = repository
            .get_unsync_posts()
            .await
            .expect("unsynced query succeeds");
        assert_eq!(
            unsynced.len(),
            1,
            "failed publish should remain in unsynced queue"
        );

        let cached = cache
            .get(&stored_post.id)
            .await
            .expect("unsynced post cached for retry");
        assert_eq!(cached.id, stored_post.id);
        assert!(!cached.is_synced);
    }

    #[tokio::test]
    async fn list_following_feed_returns_posts_in_desc_order() {
        let event_service: Arc<dyn EventServiceTrait> = Arc::new(TestEventService::default());
        let (service, repository, _cache) = setup_post_service_with_deps(event_service).await;

        let follower_pubkey = "followerpub";
        let followed_pubkey = "followedpub";

        repository
            .add_follow_relation(follower_pubkey, followed_pubkey)
            .await
            .expect("follow relation");

        let mut author = sample_user();
        author.pubkey = followed_pubkey.to_string();
        author.npub = format!("npub_{followed_pubkey}");

        let first_post = service
            .create_post(
                "first".into(),
                author.clone(),
                "trend".into(),
                sample_thread_uuid(10),
                None,
                None,
            )
            .await
            .expect("create first post");
        sleep(Duration::from_millis(5)).await;
        let second_post = service
            .create_post(
                "second".into(),
                author.clone(),
                "trend".into(),
                sample_thread_uuid(11),
                None,
                None,
            )
            .await
            .expect("create second post");

        assert_ne!(first_post.id, second_post.id);

        let raw_page = repository
            .list_following_feed(follower_pubkey, None, 5)
            .await
            .expect("raw page");
        assert_eq!(raw_page.items.len(), 2);

        let page_one = service
            .list_following_feed(follower_pubkey, None, 1)
            .await
            .expect("page one");
        assert_eq!(page_one.items.len(), 1);
        let newest_id = page_one.items[0].id.clone();
        assert_eq!(newest_id, second_post.id);
        assert!(page_one.has_more);
        let next_cursor = page_one.next_cursor.clone();

        let page_two = service
            .list_following_feed(follower_pubkey, next_cursor.as_deref(), 1)
            .await
            .expect("page two");
        assert_eq!(page_two.items.len(), 1);
        assert_eq!(page_two.items[0].id, first_post.id);
        assert!(!page_two.has_more);
    }

    #[tokio::test]
    async fn get_posts_by_topic_prefers_cache_when_available() {
        let event_service: Arc<dyn EventServiceTrait> = Arc::new(TestEventService::default());
        let (service, repository, cache) = setup_post_service_with_deps(event_service).await;
        let topic_id = "topic-cache";

        let mut posts = Vec::new();
        for (idx, ts) in [10_i64, 20, 30].into_iter().enumerate() {
            let mut post = Post::new(format!("post-{idx}"), sample_user(), topic_id.to_string());
            post.created_at = Utc.timestamp_opt(ts, 0).unwrap();
            post.is_synced = true;
            posts.push(post);
        }

        for post in &posts {
            repository.create_post(post).await.expect("seed repository");
        }

        let initial = service
            .get_posts_by_topic(topic_id, 3)
            .await
            .expect("initial fetch");
        assert_eq!(initial.len(), 3);

        // DBから削除してもキャッシュから取得できることを確認
        for post in &initial {
            repository
                .delete_post(&post.id)
                .await
                .expect("delete seeded post");
        }

        let cached = service
            .get_posts_by_topic(topic_id, 2)
            .await
            .expect("fetch from cache");
        assert_eq!(cached.len(), 2);
        assert_eq!(cached[0].id, initial[0].id);
        assert_eq!(cached[1].id, initial[1].id);

        let cached_full = cache.get_by_topic(topic_id, 5).await;
        assert_eq!(cached_full.len(), 3);
    }

    #[tokio::test]
    async fn get_posts_by_topic_merges_cached_entries_not_in_db() {
        let event_service: Arc<dyn EventServiceTrait> = Arc::new(TestEventService::default());
        let (service, repository, cache) = setup_post_service_with_deps(event_service).await;
        let topic_id = "topic-cache-merge";

        let mut db_post = Post::new("db-post".into(), sample_user(), topic_id.to_string());
        db_post.created_at = Utc.timestamp_opt(50, 0).unwrap();
        db_post.is_synced = true;
        repository
            .create_post(&db_post)
            .await
            .expect("seed repository with db post");

        let mut cached_only = Post::new("cached-only".into(), sample_user(), topic_id.to_string());
        cached_only.created_at = Utc.timestamp_opt(200, 0).unwrap();
        cache.add(cached_only.clone()).await;

        let result = service
            .get_posts_by_topic(topic_id, 2)
            .await
            .expect("fetch posts should include cached-only entry");

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].content, "cached-only");
        assert_eq!(result[1].content, "db-post");

        let cached_snapshot = cache.get_by_topic(topic_id, 5).await;
        assert!(
            cached_snapshot.iter().any(|post| post.id == cached_only.id),
            "cached-only post should remain cached after merge"
        );
    }

    #[tokio::test]
    async fn create_reply_post_reuses_parent_thread_metadata() {
        let event_service: Arc<dyn EventServiceTrait> = Arc::new(TestEventService::default());
        let (service, repository, _cache) = setup_post_service_with_deps(event_service).await;
        let thread_uuid = sample_thread_uuid(30);

        let root = service
            .create_post(
                "root".into(),
                sample_user(),
                "topic-thread".into(),
                thread_uuid.clone(),
                None,
                None,
            )
            .await
            .expect("create root");

        let reply = service
            .create_post(
                "reply".into(),
                sample_user(),
                "topic-thread".into(),
                thread_uuid.clone(),
                Some(root.id.clone()),
                None,
            )
            .await
            .expect("create reply");

        assert_eq!(reply.thread_uuid.as_deref(), Some(thread_uuid.as_str()));
        assert_eq!(
            reply.thread_root_event_id.as_deref(),
            Some(root.id.as_str())
        );
        assert_eq!(
            reply.thread_parent_event_id.as_deref(),
            Some(root.id.as_str())
        );

        let relation = repository
            .get_event_thread("topic-thread", &reply.id)
            .await
            .expect("query event_thread")
            .expect("event_thread relation");
        assert_eq!(relation.thread_uuid, thread_uuid);
        assert_eq!(relation.root_event_id, root.id);
        assert_eq!(
            relation.parent_event_id.as_deref(),
            reply.thread_parent_event_id.as_deref()
        );
    }

    #[tokio::test]
    async fn create_reply_post_publishes_synced_parent_event_id() {
        let event_service_impl = Arc::new(TestEventService::default());
        let event_service: Arc<dyn EventServiceTrait> = event_service_impl.clone();
        let (service, repository, _cache) = setup_post_service_with_deps(event_service).await;
        let thread_uuid = sample_thread_uuid(31);

        let root = service
            .create_post(
                "root".into(),
                sample_user(),
                "topic-thread".into(),
                thread_uuid.clone(),
                None,
                None,
            )
            .await
            .expect("create root");
        let root_sync_event_id = repository
            .get_sync_event_id(&root.id)
            .await
            .expect("query parent sync event id")
            .expect("parent should have sync event id");

        service
            .create_post(
                "reply".into(),
                sample_user(),
                "topic-thread".into(),
                thread_uuid,
                Some(root.id),
                None,
            )
            .await
            .expect("create reply");

        let calls = event_service_impl.publish_calls().await;
        assert_eq!(calls.len(), 2);
        assert_eq!(
            calls[1].reply_to.as_deref(),
            Some(root_sync_event_id.as_str())
        );
    }

    #[tokio::test]
    async fn create_reply_post_with_unsynced_parent_does_not_publish_uuid_reply_to() {
        let event_service_impl = Arc::new(TestEventService::with_publish_result(Err(
            AppError::NostrError("root publish failed".into()),
        )));
        let event_service: Arc<dyn EventServiceTrait> = event_service_impl.clone();
        let (service, repository, _cache) = setup_post_service_with_deps(event_service).await;
        let thread_uuid = sample_thread_uuid(32);

        let _ = service
            .create_post(
                "root".into(),
                sample_user(),
                "topic-thread".into(),
                thread_uuid.clone(),
                None,
                None,
            )
            .await
            .expect_err("root publish should fail once");
        let unsynced_parent = repository
            .get_unsync_posts()
            .await
            .expect("load unsynced posts")
            .into_iter()
            .next()
            .expect("unsynced parent exists");

        service
            .create_post(
                "reply".into(),
                sample_user(),
                "topic-thread".into(),
                thread_uuid,
                Some(unsynced_parent.id),
                None,
            )
            .await
            .expect("reply create should fallback to publish without reply_to");

        let calls = event_service_impl.publish_calls().await;
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[1].reply_to, None);
    }

    #[tokio::test]
    async fn get_thread_posts_filters_by_topic_and_thread_uuid() {
        let event_service: Arc<dyn EventServiceTrait> = Arc::new(TestEventService::default());
        let (service, _repository, _cache) = setup_post_service_with_deps(event_service).await;
        let target_thread_uuid = sample_thread_uuid(40);

        let root = service
            .create_post(
                "thread-root".into(),
                sample_user(),
                "topic-thread-main".into(),
                target_thread_uuid.clone(),
                None,
                None,
            )
            .await
            .expect("create thread root");

        sleep(Duration::from_millis(5)).await;

        let reply = service
            .create_post(
                "thread-reply".into(),
                sample_user(),
                "topic-thread-main".into(),
                target_thread_uuid.clone(),
                Some(root.id.clone()),
                None,
            )
            .await
            .expect("create thread reply");

        service
            .create_post(
                "other-thread".into(),
                sample_user(),
                "topic-thread-main".into(),
                sample_thread_uuid(41),
                None,
                None,
            )
            .await
            .expect("create other thread");

        service
            .create_post(
                "other-topic".into(),
                sample_user(),
                "topic-thread-sub".into(),
                target_thread_uuid.clone(),
                None,
                None,
            )
            .await
            .expect("create other topic");

        let posts = service
            .get_thread_posts("topic-thread-main", &target_thread_uuid, 20)
            .await
            .expect("get thread posts");

        assert_eq!(posts.len(), 2);
        assert_eq!(posts[0].id, root.id);
        assert_eq!(posts[1].id, reply.id);
        assert!(
            posts
                .iter()
                .all(|post| post.thread_uuid.as_deref() == Some(target_thread_uuid.as_str()))
        );
        assert!(
            posts
                .iter()
                .all(|post| post.topic_id == "topic-thread-main")
        );
    }

    #[tokio::test]
    async fn get_topic_timeline_returns_parent_first_reply_counts_and_last_activity() {
        let event_service: Arc<dyn EventServiceTrait> = Arc::new(TestEventService::default());
        let (service, repository, _cache) = setup_post_service_with_deps(event_service).await;
        let topic_id = "topic-timeline-main";
        let thread_a = sample_thread_uuid(50);
        let thread_b = sample_thread_uuid(51);

        let mut root_a = Post::new("thread-a-root".into(), sample_user(), topic_id.to_string());
        root_a.created_at = Utc.timestamp_opt(100, 0).unwrap();
        root_a.thread_namespace = Some(format!("{topic_id}/threads/{thread_a}"));
        root_a.thread_uuid = Some(thread_a.clone());
        root_a.thread_root_event_id = Some(root_a.id.clone());
        root_a.thread_parent_event_id = None;
        root_a.is_synced = true;
        repository
            .create_post(&root_a)
            .await
            .expect("seed thread-a root");

        let mut reply_a1 = Post::new(
            "thread-a-reply-1".into(),
            sample_user(),
            topic_id.to_string(),
        );
        reply_a1.created_at = Utc.timestamp_opt(110, 0).unwrap();
        reply_a1.thread_namespace = Some(format!("{topic_id}/threads/{thread_a}"));
        reply_a1.thread_uuid = Some(thread_a.clone());
        reply_a1.thread_root_event_id = Some(root_a.id.clone());
        reply_a1.thread_parent_event_id = Some(root_a.id.clone());
        reply_a1.is_synced = true;
        repository
            .create_post(&reply_a1)
            .await
            .expect("seed thread-a first reply");

        let mut reply_a2 = Post::new(
            "thread-a-reply-2".into(),
            sample_user(),
            topic_id.to_string(),
        );
        reply_a2.created_at = Utc.timestamp_opt(120, 0).unwrap();
        reply_a2.thread_namespace = Some(format!("{topic_id}/threads/{thread_a}"));
        reply_a2.thread_uuid = Some(thread_a.clone());
        reply_a2.thread_root_event_id = Some(root_a.id.clone());
        reply_a2.thread_parent_event_id = Some(reply_a1.id.clone());
        reply_a2.is_synced = true;
        repository
            .create_post(&reply_a2)
            .await
            .expect("seed thread-a second reply");

        let mut root_b = Post::new("thread-b-root".into(), sample_user(), topic_id.to_string());
        root_b.created_at = Utc.timestamp_opt(105, 0).unwrap();
        root_b.thread_namespace = Some(format!("{topic_id}/threads/{thread_b}"));
        root_b.thread_uuid = Some(thread_b.clone());
        root_b.thread_root_event_id = Some(root_b.id.clone());
        root_b.thread_parent_event_id = None;
        root_b.is_synced = true;
        repository
            .create_post(&root_b)
            .await
            .expect("seed thread-b root");

        let other_topic = "topic-timeline-other";
        let mut other_root = Post::new(
            "other-topic-root".into(),
            sample_user(),
            other_topic.to_string(),
        );
        other_root.created_at = Utc.timestamp_opt(130, 0).unwrap();
        other_root.thread_namespace = Some(format!("{other_topic}/threads/{thread_a}"));
        other_root.thread_uuid = Some(thread_a.clone());
        other_root.thread_root_event_id = Some(other_root.id.clone());
        other_root.thread_parent_event_id = None;
        other_root.is_synced = true;
        repository
            .create_post(&other_root)
            .await
            .expect("seed other topic root");

        let entries = service
            .get_topic_timeline(topic_id, 20)
            .await
            .expect("get topic timeline");

        assert_eq!(entries.len(), 2);

        let first = &entries[0];
        assert_eq!(first.thread_uuid, thread_a);
        assert_eq!(first.parent_post.id, root_a.id);
        assert_eq!(
            first.first_reply.as_ref().map(|post| post.id.as_str()),
            Some(reply_a1.id.as_str())
        );
        assert_eq!(first.reply_count, 2);
        assert_eq!(first.last_activity_at, 120);

        let second = &entries[1];
        assert_eq!(second.thread_uuid, thread_b);
        assert_eq!(second.parent_post.id, root_b.id);
        assert!(second.first_reply.is_none());
        assert_eq!(second.reply_count, 0);
        assert_eq!(second.last_activity_at, 105);
    }

    #[tokio::test]
    async fn get_topic_timeline_excludes_topic_posts_without_thread_uuid() {
        let event_service: Arc<dyn EventServiceTrait> = Arc::new(TestEventService::default());
        let (service, repository, _cache) = setup_post_service_with_deps(event_service).await;
        let topic_id = "topic-timeline-fallback";
        let thread_uuid = sample_thread_uuid(52);

        let mut threaded_root =
            Post::new("threaded-root".into(), sample_user(), topic_id.to_string());
        threaded_root.created_at = Utc.timestamp_opt(100, 0).unwrap();
        threaded_root.thread_namespace = Some(format!("{topic_id}/threads/{thread_uuid}"));
        threaded_root.thread_uuid = Some(thread_uuid.clone());
        threaded_root.thread_root_event_id = Some(threaded_root.id.clone());
        threaded_root.thread_parent_event_id = None;
        threaded_root.is_synced = true;
        repository
            .create_post(&threaded_root)
            .await
            .expect("seed threaded root");

        let mut standalone = Post::new("standalone".into(), sample_user(), topic_id.to_string());
        standalone.created_at = Utc.timestamp_opt(130, 0).unwrap();
        standalone.is_synced = true;
        repository
            .create_post(&standalone)
            .await
            .expect("seed standalone post");

        let entries = service
            .get_topic_timeline(topic_id, 20)
            .await
            .expect("get topic timeline");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].parent_post.id, threaded_root.id);
        assert_eq!(entries[0].thread_uuid, thread_uuid);
        assert_ne!(entries[0].parent_post.id, standalone.id);
    }

    #[tokio::test]
    async fn delete_post_removes_from_cache() {
        let (service, _repository, cache) =
            setup_post_service_with_deps(Arc::new(TestEventService::default())).await;

        let post = service
            .create_post(
                "to delete".into(),
                sample_user(),
                "topic-del".into(),
                sample_thread_uuid(20),
                None,
                None,
            )
            .await
            .expect("post creation succeeds");

        assert!(
            cache.get(&post.id).await.is_some(),
            "post should be present in cache after creation"
        );

        service
            .delete_post(&post.id)
            .await
            .expect("delete_post should succeed");

        assert!(
            cache.get(&post.id).await.is_none(),
            "post should be evicted from cache after deletion"
        );
    }
}
