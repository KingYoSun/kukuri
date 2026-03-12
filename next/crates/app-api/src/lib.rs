use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use futures_util::StreamExt;
use next_blob_service::{BlobService, MemoryBlobService, StoredBlob};
use next_core::{
    CanonicalPostHeader, EventId, GossipHint, PayloadRef, TopicId, build_text_note,
    generate_keys, timeline_sort_key,
};
use next_docs_sync::{DocOp, DocQuery, DocsSync, MemoryDocsSync, author_replica_id, stable_key, topic_replica_id};
use next_store::{BlobCacheStatus, EventProjectionRow, Page, ProjectionStore, Store, TimelineCursor};
use next_transport::{HintTransport, PeerSnapshot, TopicPeerSnapshot, Transport};
use nostr_sdk::prelude::Keys;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostView {
    pub id: String,
    pub author_pubkey: String,
    pub author_npub: String,
    pub note_id: String,
    pub content: String,
    pub created_at: i64,
    pub reply_to: Option<String>,
    pub root_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineView {
    pub items: Vec<PostView>,
    pub next_cursor: Option<TimelineCursor>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncStatus {
    pub connected: bool,
    pub last_sync_ts: Option<i64>,
    pub peer_count: usize,
    pub pending_events: usize,
    pub status_detail: String,
    pub last_error: Option<String>,
    pub configured_peers: Vec<String>,
    pub subscribed_topics: Vec<String>,
    pub topic_diagnostics: Vec<TopicSyncStatus>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicSyncStatus {
    pub topic: String,
    pub joined: bool,
    pub peer_count: usize,
    pub connected_peers: Vec<String>,
    pub configured_peer_ids: Vec<String>,
    pub missing_peer_ids: Vec<String>,
    pub last_received_at: Option<i64>,
    pub status_detail: String,
    pub last_error: Option<String>,
}

pub struct AppService {
    store: Arc<dyn Store>,
    projection_store: Arc<dyn ProjectionStore>,
    transport: Arc<dyn Transport>,
    hint_transport: Arc<dyn HintTransport>,
    docs_sync: Arc<dyn DocsSync>,
    blob_service: Arc<dyn BlobService>,
    keys: Arc<Keys>,
    subscriptions: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    last_sync_ts: Arc<Mutex<Option<i64>>>,
}

impl AppService {
    pub fn new<S, T>(store: Arc<S>, transport: Arc<T>) -> Self
    where
        S: Store + ProjectionStore + 'static,
        T: Transport + HintTransport + 'static,
    {
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        Self::new_with_services(
            store.clone() as Arc<dyn Store>,
            store as Arc<dyn ProjectionStore>,
            transport.clone(),
            transport as Arc<dyn HintTransport>,
            docs_sync,
            blob_service,
            generate_keys(),
        )
    }

    pub fn new_with_services(
        store: Arc<dyn Store>,
        projection_store: Arc<dyn ProjectionStore>,
        transport: Arc<dyn Transport>,
        hint_transport: Arc<dyn HintTransport>,
        docs_sync: Arc<dyn DocsSync>,
        blob_service: Arc<dyn BlobService>,
        keys: Keys,
    ) -> Self {
        Self {
            store,
            transport,
            projection_store,
            hint_transport,
            docs_sync,
            blob_service,
            keys: Arc::new(keys),
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
            last_sync_ts: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn create_post(
        &self,
        topic_id: &str,
        content: &str,
        reply_to: Option<&str>,
    ) -> Result<String> {
        self.ensure_topic_subscription(topic_id).await?;
        let topic = TopicId::new(topic_id);
        let parent = if let Some(reply_to) = reply_to {
            self.store.get_event(&EventId::from(reply_to)).await?
        } else {
            None
        };
        let event = build_text_note(self.keys.as_ref(), &topic, content, parent.as_ref())?;
        let stored_blob = self
            .blob_service
            .put_blob(content.as_bytes().to_vec(), "text/plain")
            .await?;
        self.ingest_event(event.clone(), Some(stored_blob.clone())).await?;
        self.hint_transport
            .publish_hint(
                &topic,
                GossipHint::TopicIndexUpdated {
                    topic_id: topic.clone(),
                    event_ids: vec![event.id.clone()],
                },
            )
            .await?;
        self.transport.publish(&topic, event.clone()).await?;
        Ok(event.id.0)
    }

    pub async fn list_timeline(
        &self,
        topic_id: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<TimelineView> {
        self.ensure_topic_subscription(topic_id).await?;
        let mut page = ProjectionStore::list_topic_timeline(
            self.projection_store.as_ref(),
            topic_id,
            cursor.clone(),
            limit,
        )
        .await?;
        if page.items.is_empty() {
            self.hydrate_topic_projection(topic_id).await?;
            page = ProjectionStore::list_topic_timeline(
                self.projection_store.as_ref(),
                topic_id,
                cursor,
                limit,
            )
            .await?;
        }
        Ok(self.page_to_view(page))
    }

    pub async fn list_thread(
        &self,
        topic_id: &str,
        thread_id: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<TimelineView> {
        self.ensure_topic_subscription(topic_id).await?;
        let mut page = ProjectionStore::list_thread(
            self.projection_store.as_ref(),
            topic_id,
            &EventId::from(thread_id),
            cursor.clone(),
            limit,
        )
        .await?;
        if page.items.is_empty() {
            self.hydrate_topic_projection(topic_id).await?;
            page = ProjectionStore::list_thread(
                self.projection_store.as_ref(),
                topic_id,
                &EventId::from(thread_id),
                cursor,
                limit,
            )
            .await?;
        }
        Ok(self.page_to_view(page))
    }

    pub async fn get_sync_status(&self) -> Result<SyncStatus> {
        let PeerSnapshot {
            connected,
            peer_count,
            connected_peers: _,
            configured_peers,
            subscribed_topics,
            pending_events,
            status_detail,
            last_error,
            topic_diagnostics,
        } = self.transport.peers().await?;

        Ok(SyncStatus {
            connected,
            last_sync_ts: *self.last_sync_ts.lock().await,
            peer_count,
            pending_events,
            status_detail,
            last_error,
            configured_peers,
            subscribed_topics,
            topic_diagnostics: topic_diagnostics
                .into_iter()
                .map(
                    |TopicPeerSnapshot {
                         topic,
                         joined,
                         peer_count,
                         connected_peers,
                         configured_peer_ids,
                         missing_peer_ids,
                         last_received_at,
                         status_detail,
                         last_error,
                     }| TopicSyncStatus {
                        topic,
                        joined,
                        peer_count,
                        connected_peers,
                        configured_peer_ids,
                        missing_peer_ids,
                        last_received_at,
                        status_detail,
                        last_error,
                    },
                )
                .collect(),
        })
    }

    pub async fn import_peer_ticket(&self, ticket: &str) -> Result<()> {
        self.transport.import_ticket(ticket).await?;
        self.docs_sync.import_peer_ticket(ticket).await?;
        self.blob_service.import_peer_ticket(ticket).await?;
        let existing_topics = self
            .subscriptions
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for topic in existing_topics {
            self.restart_topic_subscription(topic.as_str()).await?;
        }
        Ok(())
    }

    pub async fn unsubscribe_topic(&self, topic_id: &str) -> Result<()> {
        if let Some(handle) = self.subscriptions.lock().await.remove(topic_id) {
            handle.abort();
        }
        self.transport.unsubscribe(&TopicId::new(topic_id)).await
    }

    pub async fn peer_ticket(&self) -> Result<Option<String>> {
        self.transport.export_ticket().await
    }

    async fn ensure_topic_subscription(&self, topic_id: &str) -> Result<()> {
        if self.subscriptions.lock().await.contains_key(topic_id) {
            return Ok(());
        }

        self.spawn_topic_subscription(topic_id).await
    }

    async fn restart_topic_subscription(&self, topic_id: &str) -> Result<()> {
        if let Some(handle) = self.subscriptions.lock().await.remove(topic_id) {
            handle.abort();
        }
        self.spawn_topic_subscription(topic_id).await
    }

    async fn spawn_topic_subscription(&self, topic_id: &str) -> Result<()> {
        let store = Arc::clone(&self.store);
        let projection_store = Arc::clone(&self.projection_store);
        let docs_sync = Arc::clone(&self.docs_sync);
        let blob_service = Arc::clone(&self.blob_service);
        let last_sync = Arc::clone(&self.last_sync_ts);
        let mut stream = self.transport.subscribe(&TopicId::new(topic_id)).await?;
        let topic_key = topic_id.to_string();

        let handle = tokio::spawn(async move {
            while let Some(envelope) = stream.next().await {
                if store.put_event(envelope.event.clone()).await.is_ok()
                    && ingest_remote_event(
                        docs_sync.as_ref(),
                        blob_service.as_ref(),
                        projection_store.as_ref(),
                        envelope.event.clone(),
                    )
                    .await
                    .is_ok()
                {
                    *last_sync.lock().await = Some(envelope.received_at);
                }
            }
        });

        self.subscriptions.lock().await.insert(topic_key, handle);
        Ok(())
    }

    async fn ingest_event(&self, event: next_core::Event, stored_blob: Option<StoredBlob>) -> Result<()> {
        self.store.put_event(event.clone()).await?;
        let blob = match stored_blob {
            Some(blob) => blob,
            None => self
                .blob_service
                .put_blob(event.content.as_bytes().to_vec(), "text/plain")
                .await?,
        };
        let header = event.to_canonical_header(PayloadRef::BlobText {
            hash: blob.hash.clone(),
            mime: blob.mime.clone(),
            bytes: blob.bytes,
        });
        persist_header(
            self.docs_sync.as_ref(),
            header.clone(),
            event.pubkey.as_str(),
        )
        .await?;
        ProjectionStore::put_projection_row(
            self.projection_store.as_ref(),
            projection_row_from_header(&header, Some(event.content.clone())),
        )
        .await?;
        ProjectionStore::mark_blob_status(
            self.projection_store.as_ref(),
            &blob.hash,
            BlobCacheStatus::Available,
        )
        .await?;
        *self.last_sync_ts.lock().await = Some(Utc::now().timestamp_millis());
        Ok(())
    }

    async fn hydrate_topic_projection(&self, topic_id: &str) -> Result<()> {
        let replica = topic_replica_id(topic_id);
        let records = self
            .docs_sync
            .query_replica(&replica, DocQuery::Prefix("post/".into()))
            .await?;
        for record in records {
            let header: CanonicalPostHeader = serde_json::from_slice(&record.value)?;
            let content = match &header.payload_ref {
                PayloadRef::InlineText { text } => Some(text.clone()),
                PayloadRef::BlobText { hash, .. } => self
                    .blob_service
                    .fetch_blob(hash)
                    .await?
                    .map(|bytes| String::from_utf8_lossy(&bytes).to_string()),
            };
            ProjectionStore::put_projection_row(
                self.projection_store.as_ref(),
                projection_row_from_header(&header, content),
            )
            .await?;
        }
        Ok(())
    }

    fn page_to_view(&self, page: Page<EventProjectionRow>) -> TimelineView {
        TimelineView {
            items: page
                .items
                .into_iter()
                .map(|event| {
                    PostView {
                        id: event.event_id.0.clone(),
                        author_pubkey: event.author_pubkey.clone(),
                        author_npub: event.author_pubkey.clone(),
                        note_id: event.event_id.0.clone(),
                        content: event.content.unwrap_or_else(|| "[blob pending]".to_string()),
                        created_at: event.created_at,
                        reply_to: event.reply_to.map(|id| id.0),
                        root_id: event.root_id.map(|id| id.0),
                    }
                })
                .collect(),
            next_cursor: page.next_cursor,
        }
    }
}

async fn persist_header(
    docs_sync: &dyn DocsSync,
    header: CanonicalPostHeader,
    author_pubkey: &str,
) -> Result<()> {
    let topic_replica = topic_replica_id(header.topic_id.as_str());
    let author_replica = author_replica_id(author_pubkey);
    let sort_key = timeline_sort_key(header.created_at, &header.event_id);
    let header_json = serde_json::to_value(&header)?;
    docs_sync.open_replica(&topic_replica).await?;
    docs_sync.open_replica(&author_replica).await?;
    docs_sync
        .apply_doc_op(
            &topic_replica,
            DocOp::SetJson {
                key: stable_key("post", &format!("{}/header", header.event_id.as_str())),
                value: header_json.clone(),
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            &topic_replica,
            DocOp::SetJson {
                key: stable_key("timeline", &format!("{sort_key}/{}", header.event_id.as_str())),
                value: serde_json::json!({
                    "event_id": header.event_id,
                    "created_at": header.created_at,
                }),
            },
        )
        .await?;
    let root_id = header.root.clone().unwrap_or_else(|| header.event_id.clone());
    docs_sync
        .apply_doc_op(
            &topic_replica,
            DocOp::SetJson {
                key: stable_key(
                    "thread",
                    &format!("{}/{sort_key}/{}", root_id.as_str(), header.event_id.as_str()),
                ),
                value: serde_json::json!({
                    "event_id": header.event_id,
                    "root_id": root_id,
                    "reply_to": header.reply_to,
                }),
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            &author_replica,
            DocOp::SetJson {
                key: stable_key("posts", &format!("{sort_key}/{}", header.event_id.as_str())),
                value: serde_json::json!({
                    "event_id": header.event_id,
                    "topic_id": header.topic_id,
                }),
            },
        )
        .await?;
    Ok(())
}

fn projection_row_from_header(header: &CanonicalPostHeader, content: Option<String>) -> EventProjectionRow {
    let source_blob_hash = match &header.payload_ref {
        PayloadRef::BlobText { hash, .. } => Some(hash.clone()),
        PayloadRef::InlineText { .. } => None,
    };
    EventProjectionRow {
        event_id: header.event_id.clone(),
        topic_id: header.topic_id.as_str().to_string(),
        author_pubkey: header.author.as_str().to_string(),
        created_at: header.created_at,
        root_id: header.root.clone(),
        reply_to: header.reply_to.clone(),
        payload_ref: header.payload_ref.clone(),
        content,
        source_replica_id: topic_replica_id(header.topic_id.as_str()),
        source_key: stable_key("post", &format!("{}/header", header.event_id.as_str())),
        source_event_id: header.event_id.clone(),
        source_blob_hash,
        derived_at: Utc::now().timestamp_millis(),
        projection_version: 1,
    }
}

async fn ingest_remote_event(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    event: next_core::Event,
) -> Result<()> {
    let blob = blob_service
        .put_blob(event.content.as_bytes().to_vec(), "text/plain")
        .await?;
    let header = event.to_canonical_header(PayloadRef::BlobText {
        hash: blob.hash.clone(),
        mime: blob.mime.clone(),
        bytes: blob.bytes,
    });
    persist_header(docs_sync, header.clone(), event.pubkey.as_str()).await?;
    projection_store
        .put_projection_row(projection_row_from_header(&header, Some(event.content.clone())))
        .await?;
    projection_store
        .mark_blob_status(&blob.hash, BlobCacheStatus::Available)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use next_store::MemoryStore;
    use next_transport::{FakeNetwork, FakeTransport, IrohGossipTransport};
    use tokio::time::{Duration, sleep, timeout};

    #[tokio::test]
    async fn create_post_and_list_timeline() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
        let app = AppService::new(store, transport);

        let event_id = app
            .create_post("kukuri:topic:api", "hello app", None)
            .await
            .expect("create post");
        let timeline = app
            .list_timeline("kukuri:topic:api", None, 10)
            .await
            .expect("timeline");

        assert_eq!(timeline.items.len(), 1);
        assert_eq!(timeline.items[0].id, event_id);
        assert_eq!(timeline.items[0].content, "hello app");
    }

    #[tokio::test]
    async fn tracking_multiple_topics_updates_sync_status() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
        let app = AppService::new(store, transport);

        let _ = app
            .list_timeline("kukuri:topic:one", None, 10)
            .await
            .expect("timeline one");
        let _ = app
            .list_timeline("kukuri:topic:two", None, 10)
            .await
            .expect("timeline two");
        let status = app.get_sync_status().await.expect("sync status");

        assert!(
            status
                .subscribed_topics
                .iter()
                .any(|topic| topic == "kukuri:topic:one")
        );
        assert!(
            status
                .subscribed_topics
                .iter()
                .any(|topic| topic == "kukuri:topic:two")
        );
        assert!(
            status
                .topic_diagnostics
                .iter()
                .any(|topic| topic.topic == "kukuri:topic:one")
        );
        assert!(
            status
                .topic_diagnostics
                .iter()
                .any(|topic| topic.topic == "kukuri:topic:two")
        );
        assert_eq!(status.status_detail, "No peer tickets imported");
        assert!(
            status
                .topic_diagnostics
                .iter()
                .all(|topic| !topic.status_detail.is_empty())
        );
        assert!(
            status
                .topic_diagnostics
                .iter()
                .all(|topic| topic.last_error.is_none())
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn invalid_ticket_updates_sync_status_error_reason() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(
            IrohGossipTransport::bind_local()
                .await
                .expect("transport should bind"),
        );
        let app = AppService::new(store, transport);

        let error = app
            .import_peer_ticket("not-a-ticket")
            .await
            .expect_err("invalid ticket should fail");
        let status = app.get_sync_status().await.expect("sync status");

        assert!(error.to_string().contains("failed to import peer ticket"));
        assert!(
            status
                .last_error
                .as_deref()
                .is_some_and(|message| message.contains("failed to import peer ticket"))
        );
    }

    #[tokio::test]
    async fn unsubscribe_topic_removes_subscription_from_sync_status() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
        let app = AppService::new(store, transport);

        let _ = app
            .list_timeline("kukuri:topic:one", None, 10)
            .await
            .expect("timeline one");
        let _ = app
            .list_timeline("kukuri:topic:two", None, 10)
            .await
            .expect("timeline two");
        app.unsubscribe_topic("kukuri:topic:two")
            .await
            .expect("unsubscribe topic");
        let status = app.get_sync_status().await.expect("sync status");

        assert!(
            status
                .subscribed_topics
                .iter()
                .any(|topic| topic == "kukuri:topic:one")
        );
        assert!(
            !status
                .subscribed_topics
                .iter()
                .any(|topic| topic == "kukuri:topic:two")
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn iroh_transport_syncs_post_between_apps() {
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let transport_a = Arc::new(
            IrohGossipTransport::bind_local()
                .await
                .expect("transport a should bind"),
        );
        let transport_b = Arc::new(
            IrohGossipTransport::bind_local()
                .await
                .expect("transport b should bind"),
        );
        let app_a = AppService::new(store_a, transport_a.clone());
        let app_b = AppService::new(store_b, transport_b.clone());

        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = app_b
            .peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        app_a
            .import_peer_ticket(&ticket_b)
            .await
            .expect("import b into a");
        app_b
            .import_peer_ticket(&ticket_a)
            .await
            .expect("import a into b");

        let topic = "kukuri:topic:app-api-iroh";
        let _ = app_b
            .list_timeline(topic, None, 20)
            .await
            .expect("app b should subscribe to topic");

        let event_id = app_a
            .create_post(topic, "hello over iroh transport", None)
            .await
            .expect("app a should create post");

        let received = timeout(Duration::from_secs(10), async {
            loop {
                let timeline = app_b
                    .list_timeline(topic, None, 20)
                    .await
                    .expect("timeline should load");
                if let Some(post) = timeline.items.iter().find(|post| post.id == event_id) {
                    return post.clone();
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("timeline sync timeout");

        assert_eq!(received.content, "hello over iroh transport");
        let status_b = app_b.get_sync_status().await.expect("sync status b");
        assert!(status_b.last_sync_ts.is_some());
        assert!(
            status_b
                .subscribed_topics
                .iter()
                .any(|value| value == topic)
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn import_peer_ticket_rebuilds_existing_topic_subscription() {
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let transport_a = Arc::new(
            IrohGossipTransport::bind_local()
                .await
                .expect("transport a should bind"),
        );
        let transport_b = Arc::new(
            IrohGossipTransport::bind_local()
                .await
                .expect("transport b should bind"),
        );
        let app_a = AppService::new(store_a, transport_a);
        let app_b = AppService::new(store_b, transport_b);
        let topic = "kukuri:topic:rebind-after-import";

        let _ = app_a
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe a before import");
        let _ = app_b
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe b before import");

        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = app_b
            .peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        app_a
            .import_peer_ticket(&ticket_b)
            .await
            .expect("import b into a");
        app_b
            .import_peer_ticket(&ticket_a)
            .await
            .expect("import a into b");

        let event_id = app_a
            .create_post(topic, "hello after import", None)
            .await
            .expect("create post");
        let received = timeout(Duration::from_secs(10), async {
            loop {
                let timeline = app_b
                    .list_timeline(topic, None, 20)
                    .await
                    .expect("timeline should load");
                if let Some(post) = timeline.items.iter().find(|post| post.id == event_id) {
                    return post.clone();
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("timeline sync timeout");

        assert_eq!(received.content, "hello after import");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn iroh_transport_syncs_reply_into_thread() {
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let transport_a = Arc::new(
            IrohGossipTransport::bind_local()
                .await
                .expect("transport a should bind"),
        );
        let transport_b = Arc::new(
            IrohGossipTransport::bind_local()
                .await
                .expect("transport b should bind"),
        );
        let app_a = AppService::new(store_a, transport_a);
        let app_b = AppService::new(store_b, transport_b);
        let topic = "kukuri:topic:reply-thread";

        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = app_b
            .peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        app_a
            .import_peer_ticket(&ticket_b)
            .await
            .expect("import b into a");
        app_b
            .import_peer_ticket(&ticket_a)
            .await
            .expect("import a into b");

        let _ = app_b
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe b timeline");
        let root_id = app_a
            .create_post(topic, "root over iroh", None)
            .await
            .expect("create root");

        timeout(Duration::from_secs(10), async {
            loop {
                let timeline = app_b
                    .list_timeline(topic, None, 20)
                    .await
                    .expect("timeline b");
                if timeline.items.iter().any(|post| post.id == root_id) {
                    return;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("root propagation timeout");

        let reply_id = app_b
            .create_post(topic, "reply over iroh", Some(root_id.as_str()))
            .await
            .expect("create reply");
        let thread = timeout(Duration::from_secs(10), async {
            loop {
                let thread = app_a
                    .list_thread(topic, root_id.as_str(), None, 20)
                    .await
                    .expect("thread a");
                if thread.items.iter().any(|post| post.id == reply_id) {
                    return thread;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("reply propagation timeout");

        assert_eq!(thread.items.len(), 2);
        let reply = thread
            .items
            .iter()
            .find(|post| post.id == reply_id)
            .expect("reply in thread");
        assert_eq!(reply.reply_to.as_deref(), Some(root_id.as_str()));
        assert_eq!(reply.root_id.as_deref(), Some(root_id.as_str()));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn iroh_transport_syncs_multiple_topics_bidirectionally() {
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let transport_a = Arc::new(
            IrohGossipTransport::bind_local()
                .await
                .expect("transport a should bind"),
        );
        let transport_b = Arc::new(
            IrohGossipTransport::bind_local()
                .await
                .expect("transport b should bind"),
        );
        let app_a = AppService::new(store_a, transport_a);
        let app_b = AppService::new(store_b, transport_b);
        let topic_one = "kukuri:topic:one";
        let topic_two = "kukuri:topic:two";

        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = app_b
            .peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        app_a
            .import_peer_ticket(&ticket_b)
            .await
            .expect("import b into a");
        app_b
            .import_peer_ticket(&ticket_a)
            .await
            .expect("import a into b");

        let _ = app_a
            .list_timeline(topic_one, None, 20)
            .await
            .expect("subscribe a topic one");
        let _ = app_a
            .list_timeline(topic_two, None, 20)
            .await
            .expect("subscribe a topic two");
        let _ = app_b
            .list_timeline(topic_one, None, 20)
            .await
            .expect("subscribe b topic one");
        let _ = app_b
            .list_timeline(topic_two, None, 20)
            .await
            .expect("subscribe b topic two");

        let id_one = app_a
            .create_post(topic_one, "topic one from a", None)
            .await
            .expect("post one");
        let id_two = app_b
            .create_post(topic_two, "topic two from b", None)
            .await
            .expect("post two");

        timeout(Duration::from_secs(10), async {
            loop {
                let timeline_b = app_b
                    .list_timeline(topic_one, None, 20)
                    .await
                    .expect("timeline b");
                let timeline_a = app_a
                    .list_timeline(topic_two, None, 20)
                    .await
                    .expect("timeline a");
                let has_one = timeline_b.items.iter().any(|post| post.id == id_one);
                let has_two = timeline_a.items.iter().any(|post| post.id == id_two);
                if has_one && has_two {
                    return;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("multi topic propagation timeout");
    }
}
