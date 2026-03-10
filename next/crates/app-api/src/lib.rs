use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use futures_util::StreamExt;
use next_core::{EventId, TopicId, build_text_note, generate_keys};
use next_store::{Page, Store, TimelineCursor};
use next_transport::{PeerSnapshot, Transport};
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
    pub subscribed_topics: Vec<String>,
}

pub struct AppService {
    store: Arc<dyn Store>,
    transport: Arc<dyn Transport>,
    keys: Arc<Keys>,
    subscriptions: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    last_sync_ts: Arc<Mutex<Option<i64>>>,
}

impl AppService {
    pub fn new(store: Arc<dyn Store>, transport: Arc<dyn Transport>) -> Self {
        Self {
            store,
            transport,
            keys: Arc::new(generate_keys()),
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
        self.store.put_event(event.clone()).await?;
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
        let page = self
            .store
            .list_topic_timeline(topic_id, cursor, limit)
            .await?;
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
        let page = self
            .store
            .list_thread(topic_id, &EventId::from(thread_id), cursor, limit)
            .await?;
        Ok(self.page_to_view(page))
    }

    pub async fn get_sync_status(&self) -> Result<SyncStatus> {
        let PeerSnapshot {
            connected,
            peer_count,
            connected_peers: _,
            subscribed_topics,
            pending_events,
        } = self.transport.peers().await?;

        Ok(SyncStatus {
            connected,
            last_sync_ts: *self.last_sync_ts.lock().await,
            peer_count,
            pending_events,
            subscribed_topics,
        })
    }

    pub async fn import_peer_ticket(&self, ticket: &str) -> Result<()> {
        self.transport.import_ticket(ticket).await
    }

    pub async fn peer_ticket(&self) -> Result<Option<String>> {
        self.transport.export_ticket().await
    }

    async fn ensure_topic_subscription(&self, topic_id: &str) -> Result<()> {
        if self.subscriptions.lock().await.contains_key(topic_id) {
            return Ok(());
        }

        let store = Arc::clone(&self.store);
        let last_sync = Arc::clone(&self.last_sync_ts);
        let mut stream = self.transport.subscribe(&TopicId::new(topic_id)).await?;
        let topic_key = topic_id.to_string();

        let handle = tokio::spawn(async move {
            while let Some(envelope) = stream.next().await {
                if store.put_event(envelope.event).await.is_ok() {
                    *last_sync.lock().await = Some(envelope.received_at);
                }
            }
        });

        self.subscriptions.lock().await.insert(topic_key, handle);
        Ok(())
    }

    fn page_to_view(&self, page: Page<next_core::Event>) -> TimelineView {
        TimelineView {
            items: page
                .items
                .into_iter()
                .map(|event| {
                    let thread = event.thread_ref();
                    PostView {
                        id: event.id.0.clone(),
                        author_pubkey: event.pubkey.0.clone(),
                        author_npub: event
                            .author_npub()
                            .unwrap_or_else(|_| event.pubkey.0.clone()),
                        note_id: event.note_id().unwrap_or_else(|_| event.id.0.clone()),
                        content: event.content,
                        created_at: event.created_at,
                        reply_to: thread
                            .as_ref()
                            .and_then(|thread| thread.reply_to.as_ref().map(|id| id.0.clone())),
                        root_id: thread.map(|thread| thread.root.0),
                    }
                })
                .collect(),
            next_cursor: page.next_cursor,
        }
    }
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
}
