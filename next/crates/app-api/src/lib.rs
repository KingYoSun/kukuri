use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use futures_util::StreamExt;
use next_blob_service::{BlobService, MemoryBlobService, StoredBlob};
use next_core::{
    CanonicalPostHeader, EventId, GossipHint, PayloadRef, TopicId, build_text_note, generate_keys,
    timeline_sort_key,
};
use next_docs_sync::{
    DocOp, DocQuery, DocsSync, MemoryDocsSync, author_replica_id, stable_key, topic_replica_id,
};
use next_store::{
    BlobCacheStatus, EventProjectionRow, Page, ProjectionStore, Store, TimelineCursor,
};
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
            self.resolve_parent_event(&EventId::from(reply_to)).await?
        } else {
            None
        };
        let event = build_text_note(self.keys.as_ref(), &topic, content, parent.as_ref())?;
        let stored_blob = self
            .blob_service
            .put_blob(content.as_bytes().to_vec(), "text/plain")
            .await?;
        self.ingest_event(event.clone(), Some(stored_blob.clone()))
            .await?;
        self.hint_transport
            .publish_hint(
                &topic,
                GossipHint::TopicIndexUpdated {
                    topic_id: topic.clone(),
                    event_ids: vec![event.id.clone()],
                },
            )
            .await?;
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
        if page.items.is_empty() || projection_page_needs_hydration(&page) {
            if self.hydrate_topic_projection(topic_id).await? > 0 {
                *self.last_sync_ts.lock().await = Some(Utc::now().timestamp_millis());
            }
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
        if page.items.is_empty() || projection_page_needs_hydration(&page) {
            if self.hydrate_topic_projection(topic_id).await? > 0 {
                *self.last_sync_ts.lock().await = Some(Utc::now().timestamp_millis());
            }
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
        let subscribed_topics = normalize_topics(subscribed_topics);
        let topic_diagnostics = normalize_topic_diagnostics(topic_diagnostics);

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
                .map(|diagnostic| TopicSyncStatus {
                    topic: diagnostic.topic,
                    joined: diagnostic.joined,
                    peer_count: diagnostic.peer_count,
                    connected_peers: diagnostic.connected_peers,
                    configured_peer_ids: diagnostic.configured_peer_ids,
                    missing_peer_ids: diagnostic.missing_peer_ids,
                    last_received_at: diagnostic.last_received_at,
                    status_detail: diagnostic.status_detail,
                    last_error: diagnostic.last_error,
                })
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
        self.hint_transport
            .unsubscribe_hints(&TopicId::new(topic_id))
            .await?;
        self.transport.unsubscribe(&TopicId::new(topic_id)).await
    }

    pub async fn peer_ticket(&self) -> Result<Option<String>> {
        self.transport.export_ticket().await
    }

    pub async fn shutdown(&self) {
        let handles = {
            let mut subscriptions = self.subscriptions.lock().await;
            subscriptions
                .drain()
                .map(|(_, handle)| handle)
                .collect::<Vec<_>>()
        };
        for handle in handles {
            handle.abort();
            let _ = handle.await;
        }
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
        let projection_store = Arc::clone(&self.projection_store);
        let docs_sync = Arc::clone(&self.docs_sync);
        let blob_service = Arc::clone(&self.blob_service);
        let hint_transport = Arc::clone(&self.hint_transport);
        let last_sync = Arc::clone(&self.last_sync_ts);
        let topic_key = topic_id.to_string();
        let topic_replica = topic_replica_id(topic_id);
        docs_sync.open_replica(&topic_replica).await?;
        let mut doc_stream = docs_sync.subscribe_replica(&topic_replica).await?;
        let mut hint_stream = hint_transport
            .subscribe_hints(&TopicId::new(topic_id))
            .await?;
        let topic = topic_id.to_string();
        let handle = tokio::spawn(async move {
            let _ = hydrate_topic_projection_with_services(
                docs_sync.as_ref(),
                blob_service.as_ref(),
                projection_store.as_ref(),
                topic.as_str(),
            )
            .await;
            loop {
                tokio::select! {
                    Some(event) = doc_stream.next() => {
                        if event.is_ok()
                            && let Ok(count) = hydrate_topic_projection_with_services(
                                docs_sync.as_ref(),
                                blob_service.as_ref(),
                                projection_store.as_ref(),
                                topic.as_str(),
                            ).await
                            && count > 0
                        {
                            *last_sync.lock().await = Some(Utc::now().timestamp_millis());
                        }
                    }
                    Some(event) = hint_stream.next() => {
                        if hint_targets_topic(&event.hint, topic.as_str())
                            && let Ok(count) = hydrate_topic_projection_with_services(
                                docs_sync.as_ref(),
                                blob_service.as_ref(),
                                projection_store.as_ref(),
                                topic.as_str(),
                            ).await
                            && count > 0
                        {
                            *last_sync.lock().await = Some(Utc::now().timestamp_millis());
                        }
                    }
                    else => break,
                }
            }
        });

        self.subscriptions.lock().await.insert(topic_key, handle);
        Ok(())
    }

    async fn ingest_event(
        &self,
        event: next_core::Event,
        stored_blob: Option<StoredBlob>,
    ) -> Result<()> {
        self.store.put_event(event.clone()).await?;
        let blob = match stored_blob {
            Some(blob) => blob,
            None => {
                self.blob_service
                    .put_blob(event.content.as_bytes().to_vec(), "text/plain")
                    .await?
            }
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

    async fn resolve_parent_event(&self, event_id: &EventId) -> Result<Option<next_core::Event>> {
        if let Some(event) = self.store.get_event(event_id).await? {
            return Ok(Some(event));
        }

        let Some(projection) =
            ProjectionStore::get_event_projection(self.projection_store.as_ref(), event_id).await?
        else {
            return Ok(None);
        };

        let mut tags = vec![
            vec!["t".into(), projection.topic_id.clone()],
            vec!["topic".into(), projection.topic_id.clone()],
        ];
        if let Some(root_id) = projection.root_id.clone() {
            tags.push(vec![
                "e".into(),
                root_id.0.clone(),
                String::new(),
                "root".into(),
            ]);
        }
        if let Some(reply_to) = projection.reply_to.clone() {
            tags.push(vec![
                "e".into(),
                reply_to.0.clone(),
                String::new(),
                "reply".into(),
            ]);
        }

        Ok(Some(next_core::Event {
            id: projection.event_id,
            pubkey: projection.author_pubkey.into(),
            created_at: projection.created_at,
            kind: 1,
            tags,
            content: projection.content.unwrap_or_default(),
            sig: String::new(),
        }))
    }

    async fn hydrate_topic_projection(&self, topic_id: &str) -> Result<usize> {
        hydrate_topic_projection_with_services(
            self.docs_sync.as_ref(),
            self.blob_service.as_ref(),
            self.projection_store.as_ref(),
            topic_id,
        )
        .await
    }

    fn page_to_view(&self, page: Page<EventProjectionRow>) -> TimelineView {
        TimelineView {
            items: page
                .items
                .into_iter()
                .map(|event| PostView {
                    id: event.event_id.0.clone(),
                    author_pubkey: event.author_pubkey.clone(),
                    author_npub: event.author_pubkey.clone(),
                    note_id: event.event_id.0.clone(),
                    content: event
                        .content
                        .unwrap_or_else(|| "[blob pending]".to_string()),
                    created_at: event.created_at,
                    reply_to: event.reply_to.map(|id| id.0),
                    root_id: event.root_id.map(|id| id.0),
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
                key: stable_key(
                    "timeline",
                    &format!("{sort_key}/{}", header.event_id.as_str()),
                ),
                value: serde_json::json!({
                    "event_id": header.event_id,
                    "created_at": header.created_at,
                }),
            },
        )
        .await?;
    let root_id = header
        .root
        .clone()
        .unwrap_or_else(|| header.event_id.clone());
    docs_sync
        .apply_doc_op(
            &topic_replica,
            DocOp::SetJson {
                key: stable_key(
                    "thread",
                    &format!(
                        "{}/{sort_key}/{}",
                        root_id.as_str(),
                        header.event_id.as_str()
                    ),
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

fn projection_row_from_header(
    header: &CanonicalPostHeader,
    content: Option<String>,
) -> EventProjectionRow {
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

async fn hydrate_topic_projection_with_services(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
) -> Result<usize> {
    let replica = topic_replica_id(topic_id);
    let records = docs_sync
        .query_replica(&replica, DocQuery::Prefix("post/".into()))
        .await?;
    let mut hydrated = 0usize;
    for record in records {
        let header: CanonicalPostHeader = serde_json::from_slice(&record.value)?;
        let content = match &header.payload_ref {
            PayloadRef::InlineText { text } => Some(text.clone()),
            PayloadRef::BlobText { hash, .. } => blob_service
                .fetch_blob(hash)
                .await?
                .map(|bytes| String::from_utf8_lossy(&bytes).to_string()),
        };
        projection_store
            .put_projection_row(projection_row_from_header(&header, content))
            .await?;
        hydrated += 1;
    }
    Ok(hydrated)
}

fn hint_targets_topic(hint: &GossipHint, topic: &str) -> bool {
    match hint {
        GossipHint::TopicIndexUpdated { topic_id, .. }
        | GossipHint::Presence { topic_id, .. }
        | GossipHint::Typing { topic_id, .. }
        | GossipHint::LiveSignal { topic_id, .. } => topic_id.as_str() == topic,
        GossipHint::ThreadUpdated { .. } | GossipHint::ProfileUpdated { .. } => true,
    }
}

fn projection_page_needs_hydration(page: &Page<EventProjectionRow>) -> bool {
    page.items.iter().any(|item| item.content.is_none())
}

fn normalize_topic_name(topic: String) -> String {
    topic
        .strip_prefix("hint/")
        .map_or(topic.clone(), ToOwned::to_owned)
}

fn normalize_topics(topics: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();
    for topic in topics {
        let topic = normalize_topic_name(topic);
        if seen.insert(topic.clone()) {
            normalized.push(topic);
        }
    }
    normalized
}

fn normalize_topic_diagnostics(diagnostics: Vec<TopicPeerSnapshot>) -> Vec<TopicPeerSnapshot> {
    let mut merged = BTreeMap::<String, TopicPeerSnapshot>::new();
    for diagnostic in diagnostics {
        let topic = normalize_topic_name(diagnostic.topic);
        let entry = merged
            .entry(topic.clone())
            .or_insert_with(|| TopicPeerSnapshot {
                topic: topic.clone(),
                joined: false,
                peer_count: 0,
                connected_peers: Vec::new(),
                configured_peer_ids: Vec::new(),
                missing_peer_ids: Vec::new(),
                last_received_at: None,
                status_detail: diagnostic.status_detail.clone(),
                last_error: diagnostic.last_error.clone(),
            });
        entry.joined |= diagnostic.joined;
        entry.peer_count = entry.peer_count.max(diagnostic.peer_count);
        for peer in diagnostic.connected_peers {
            if !entry.connected_peers.contains(&peer) {
                entry.connected_peers.push(peer);
            }
        }
        for peer in diagnostic.configured_peer_ids {
            if !entry.configured_peer_ids.contains(&peer) {
                entry.configured_peer_ids.push(peer);
            }
        }
        for peer in diagnostic.missing_peer_ids {
            if !entry.missing_peer_ids.contains(&peer) {
                entry.missing_peer_ids.push(peer);
            }
        }
        entry.last_received_at = match (entry.last_received_at, diagnostic.last_received_at) {
            (Some(left), Some(right)) => Some(left.max(right)),
            (None, value) | (value, None) => value,
        };
        if entry.status_detail.starts_with("No peers configured")
            || entry.status_detail.starts_with("Waiting")
        {
            entry.status_detail = diagnostic.status_detail;
        }
        if entry.last_error.is_none() {
            entry.last_error = diagnostic.last_error;
        }
    }
    merged.into_values().collect()
}

impl Drop for AppService {
    fn drop(&mut self) {
        if let Ok(mut subscriptions) = self.subscriptions.try_lock() {
            for (_, handle) in subscriptions.drain() {
                handle.abort();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use next_blob_service::IrohBlobService;
    use next_docs_sync::IrohDocsNode;
    use next_docs_sync::IrohDocsSync;
    use next_store::MemoryStore;
    use next_transport::{
        EventEnvelope, EventStream, FakeNetwork, FakeTransport, HintEnvelope, HintStream,
        IrohGossipTransport,
    };
    use tempfile::tempdir;
    use tokio::sync::{Mutex as TokioMutex, broadcast};
    use tokio::time::{Duration, sleep, timeout};
    use tokio_stream::wrappers::BroadcastStream;

    #[derive(Clone)]
    struct StaticTransport {
        peers: Arc<TokioMutex<PeerSnapshot>>,
        events: Arc<TokioMutex<HashMap<String, broadcast::Sender<EventEnvelope>>>>,
        hints: Arc<TokioMutex<HashMap<String, broadcast::Sender<HintEnvelope>>>>,
        local_ticket: String,
    }

    impl StaticTransport {
        fn new(peers: PeerSnapshot) -> Self {
            Self {
                peers: Arc::new(TokioMutex::new(peers)),
                events: Arc::new(TokioMutex::new(HashMap::new())),
                hints: Arc::new(TokioMutex::new(HashMap::new())),
                local_ticket: "static-peer".into(),
            }
        }

        async fn event_sender(&self, topic: &TopicId) -> broadcast::Sender<EventEnvelope> {
            let mut guard = self.events.lock().await;
            guard
                .entry(topic.as_str().to_string())
                .or_insert_with(|| broadcast::channel(64).0)
                .clone()
        }

        async fn hint_sender(&self, topic: &TopicId) -> broadcast::Sender<HintEnvelope> {
            let mut guard = self.hints.lock().await;
            guard
                .entry(topic.as_str().to_string())
                .or_insert_with(|| broadcast::channel(64).0)
                .clone()
        }
    }

    #[async_trait]
    impl Transport for StaticTransport {
        async fn subscribe(&self, topic: &TopicId) -> Result<EventStream> {
            let sender = self.event_sender(topic).await;
            let stream = BroadcastStream::new(sender.subscribe())
                .filter_map(|item| async move { item.ok() });
            Ok(Box::pin(stream))
        }

        async fn unsubscribe(&self, _topic: &TopicId) -> Result<()> {
            Ok(())
        }

        async fn publish(&self, topic: &TopicId, event: next_core::Event) -> Result<()> {
            let sender = self.event_sender(topic).await;
            let _ = sender.send(EventEnvelope {
                event,
                received_at: Utc::now().timestamp_millis(),
                source_peer: "static".into(),
            });
            Ok(())
        }

        async fn peers(&self) -> Result<PeerSnapshot> {
            Ok(self.peers.lock().await.clone())
        }

        async fn export_ticket(&self) -> Result<Option<String>> {
            Ok(Some(self.local_ticket.clone()))
        }

        async fn import_ticket(&self, _ticket: &str) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl HintTransport for StaticTransport {
        async fn subscribe_hints(&self, topic: &TopicId) -> Result<HintStream> {
            let sender = self.hint_sender(topic).await;
            let stream = BroadcastStream::new(sender.subscribe())
                .filter_map(|item| async move { item.ok() });
            Ok(Box::pin(stream))
        }

        async fn unsubscribe_hints(&self, _topic: &TopicId) -> Result<()> {
            Ok(())
        }

        async fn publish_hint(&self, topic: &TopicId, hint: GossipHint) -> Result<()> {
            let sender = self.hint_sender(topic).await;
            let _ = sender.send(HintEnvelope {
                hint,
                received_at: Utc::now().timestamp_millis(),
                source_peer: "static".into(),
            });
            Ok(())
        }
    }

    struct TestIrohStack {
        _node: Arc<IrohDocsNode>,
        transport: Arc<IrohGossipTransport>,
        docs_sync: Arc<IrohDocsSync>,
        blob_service: Arc<IrohBlobService>,
    }

    impl TestIrohStack {
        async fn new(root: &std::path::Path) -> Self {
            let node = IrohDocsNode::persistent_with_config(
                root,
                next_transport::TransportNetworkConfig::loopback(),
            )
            .await
            .expect("iroh docs node");
            let transport = Arc::new(IrohGossipTransport::from_shared_parts(
                node.endpoint().clone(),
                node.gossip().clone(),
                node.discovery(),
                next_transport::TransportNetworkConfig::loopback(),
            ));
            let docs_sync = Arc::new(IrohDocsSync::new(node.clone()));
            let blob_service = Arc::new(IrohBlobService::new(node.clone()));
            Self {
                _node: node,
                transport,
                docs_sync,
                blob_service,
            }
        }
    }

    fn app_with_iroh_services(store: Arc<MemoryStore>, stack: &TestIrohStack) -> AppService {
        AppService::new_with_services(
            store.clone(),
            store,
            stack.transport.clone(),
            stack.transport.clone(),
            stack.docs_sync.clone(),
            stack.blob_service.clone(),
            generate_keys(),
        )
    }

    #[derive(Clone)]
    struct NoopHintTransport;

    #[async_trait]
    impl HintTransport for NoopHintTransport {
        async fn subscribe_hints(&self, _topic: &TopicId) -> Result<HintStream> {
            Ok(Box::pin(futures_util::stream::empty()))
        }

        async fn unsubscribe_hints(&self, _topic: &TopicId) -> Result<()> {
            Ok(())
        }

        async fn publish_hint(&self, _topic: &TopicId, _hint: GossipHint) -> Result<()> {
            Ok(())
        }
    }

    async fn assert_docs_sync_recovers_post_without_hints(topic: &str, content: &str) {
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = AppService::new_with_services(
            store_a.clone(),
            store_a,
            stack_a.transport.clone(),
            Arc::new(NoopHintTransport),
            stack_a.docs_sync.clone(),
            stack_a.blob_service.clone(),
            generate_keys(),
        );
        let app_b = AppService::new_with_services(
            store_b.clone(),
            store_b,
            stack_b.transport.clone(),
            Arc::new(NoopHintTransport),
            stack_b.docs_sync.clone(),
            stack_b.blob_service.clone(),
            generate_keys(),
        );

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
        app_a.import_peer_ticket(&ticket_b).await.expect("import b");
        app_b.import_peer_ticket(&ticket_a).await.expect("import a");

        let event_id = app_a
            .create_post(topic, content, None)
            .await
            .expect("create post");

        let received = timeout(Duration::from_secs(10), async {
            loop {
                let timeline = app_b
                    .list_timeline(topic, None, 20)
                    .await
                    .expect("timeline");
                if let Some(post) = timeline.items.iter().find(|post| post.id == event_id) {
                    return post.clone();
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("missing gossip timeout");

        assert_eq!(received.content, content);
    }

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

    #[tokio::test]
    async fn list_timeline_rehydrates_placeholder_from_blob_store() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let keys = generate_keys();
        let topic = TopicId::new("kukuri:topic:hydrate");
        let event = build_text_note(&keys, &topic, "hello after blob fetch", None).expect("event");
        let stored_blob = blob_service
            .put_blob(b"hello after blob fetch".to_vec(), "text/plain")
            .await
            .expect("put blob");
        let header = event.to_canonical_header(PayloadRef::BlobText {
            hash: stored_blob.hash.clone(),
            mime: stored_blob.mime.clone(),
            bytes: stored_blob.bytes,
        });
        persist_header(docs_sync.as_ref(), header.clone(), event.pubkey.as_str())
            .await
            .expect("persist header");
        ProjectionStore::put_projection_row(
            store.as_ref(),
            projection_row_from_header(&header, None),
        )
        .await
        .expect("put placeholder projection");

        let app = AppService::new_with_services(
            store.clone(),
            store,
            transport.clone(),
            transport,
            docs_sync,
            blob_service,
            keys,
        );

        let timeline = app
            .list_timeline(topic.as_str(), None, 20)
            .await
            .expect("timeline");

        assert_eq!(timeline.items.len(), 1);
        assert_eq!(timeline.items[0].content, "hello after blob fetch");
    }

    #[tokio::test]
    async fn on_demand_hydration_updates_last_sync_ts() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let keys = generate_keys();
        let topic = TopicId::new("kukuri:topic:on-demand-sync-ts");
        let event = build_text_note(&keys, &topic, "hydrate updates sync ts", None).expect("event");
        let stored_blob = blob_service
            .put_blob(b"hydrate updates sync ts".to_vec(), "text/plain")
            .await
            .expect("put blob");
        let header = event.to_canonical_header(PayloadRef::BlobText {
            hash: stored_blob.hash,
            mime: stored_blob.mime,
            bytes: stored_blob.bytes,
        });
        persist_header(docs_sync.as_ref(), header, event.pubkey.as_str())
            .await
            .expect("persist header");

        let app = AppService::new_with_services(
            store.clone(),
            store,
            transport.clone(),
            transport,
            docs_sync,
            blob_service,
            keys,
        );

        assert!(
            app.get_sync_status()
                .await
                .expect("status")
                .last_sync_ts
                .is_none()
        );

        let timeline = app
            .list_timeline(topic.as_str(), None, 20)
            .await
            .expect("timeline");
        assert_eq!(timeline.items.len(), 1);

        assert!(
            app.get_sync_status()
                .await
                .expect("status")
                .last_sync_ts
                .is_some()
        );
    }

    #[tokio::test]
    async fn sync_status_normalizes_hint_topic_names() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot {
            connected: true,
            peer_count: 1,
            connected_peers: vec!["peer-a".into()],
            configured_peers: vec!["peer-a".into()],
            subscribed_topics: vec!["hint/kukuri:topic:demo".into()],
            pending_events: 0,
            status_detail: "Connected".into(),
            last_error: None,
            topic_diagnostics: vec![TopicPeerSnapshot {
                topic: "hint/kukuri:topic:demo".into(),
                joined: true,
                peer_count: 1,
                connected_peers: vec!["peer-a".into()],
                configured_peer_ids: vec!["peer-a".into()],
                missing_peer_ids: Vec::new(),
                last_received_at: Some(1),
                status_detail: "Connected".into(),
                last_error: None,
            }],
        }));
        let app = AppService::new(store, transport);

        let status = app.get_sync_status().await.expect("sync status");

        assert_eq!(status.subscribed_topics, vec!["kukuri:topic:demo"]);
        assert_eq!(status.topic_diagnostics.len(), 1);
        assert_eq!(status.topic_diagnostics[0].topic, "kukuri:topic:demo");
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn missing_gossip_but_docs_sync_recovers_post() {
        assert_docs_sync_recovers_post_without_hints("kukuri:topic:missing-gossip", "docs recover")
            .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn gossip_loss_does_not_lose_durable_post() {
        assert_docs_sync_recovers_post_without_hints(
            "kukuri:topic:gossip-loss",
            "durable docs payload",
        )
        .await;
    }

    #[tokio::test]
    async fn thread_open_triggers_lazy_blob_fetch() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let keys = generate_keys();
        let topic = TopicId::new("kukuri:topic:thread-lazy");
        let root = build_text_note(&keys, &topic, "root body", None).expect("root");
        let reply = build_text_note(&keys, &topic, "reply body", Some(&root)).expect("reply");

        for event in [root.clone(), reply.clone()] {
            let blob = blob_service
                .put_blob(event.content.as_bytes().to_vec(), "text/plain")
                .await
                .expect("put blob");
            let header = event.to_canonical_header(PayloadRef::BlobText {
                hash: blob.hash,
                mime: blob.mime,
                bytes: blob.bytes,
            });
            persist_header(docs_sync.as_ref(), header.clone(), event.pubkey.as_str())
                .await
                .expect("persist header");
            ProjectionStore::put_projection_row(
                store.as_ref(),
                projection_row_from_header(&header, None),
            )
            .await
            .expect("placeholder row");
        }

        let app = AppService::new_with_services(
            store.clone(),
            store,
            transport.clone(),
            transport,
            docs_sync,
            blob_service,
            generate_keys(),
        );

        let thread = app
            .list_thread(topic.as_str(), root.id.as_str(), None, 20)
            .await
            .expect("thread");

        assert_eq!(thread.items.len(), 2);
        assert!(thread.items.iter().any(|post| post.content == "root body"));
        assert!(thread.items.iter().any(|post| post.content == "reply body"));
    }

    #[tokio::test]
    async fn image_post_visible_before_full_blob_download() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let keys = generate_keys();
        let topic = TopicId::new("kukuri:topic:image");
        let event = build_text_note(&keys, &topic, "", None).expect("event");
        let mut header = event.to_canonical_header(PayloadRef::BlobText {
            hash: next_core::BlobHash::new("f".repeat(64)),
            mime: "text/plain".into(),
            bytes: 0,
        });
        header.attachments = vec![next_core::AssetRef {
            hash: next_core::BlobHash::new("a".repeat(64)),
            mime: "image/png".into(),
            bytes: 1024,
            role: next_core::AssetRole::ImageOriginal,
        }];
        persist_header(docs_sync.as_ref(), header.clone(), event.pubkey.as_str())
            .await
            .expect("persist header");

        let app = AppService::new_with_services(
            store.clone(),
            store.clone(),
            transport.clone(),
            transport,
            docs_sync,
            blob_service.clone(),
            generate_keys(),
        );

        let timeline = app
            .list_timeline(topic.as_str(), None, 20)
            .await
            .expect("timeline");
        assert_eq!(timeline.items.len(), 1);
        assert_eq!(timeline.items[0].content, "[blob pending]");

        blob_service
            .put_blob(b"caption".to_vec(), "text/plain")
            .await
            .expect("put body blob");
    }

    #[tokio::test]
    async fn new_writes_use_blob_text_payload_refs() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
        let app = AppService::new(store.clone(), transport);
        let topic = "kukuri:topic:blobtext";

        let event_id = app
            .create_post(topic, "blob text only", None)
            .await
            .expect("create post");
        let projection =
            ProjectionStore::get_event_projection(store.as_ref(), &EventId::from(event_id))
                .await
                .expect("projection")
                .expect("projection row");

        assert!(matches!(
            projection.payload_ref,
            PayloadRef::BlobText { .. }
        ));
        assert!(!matches!(
            projection.payload_ref,
            PayloadRef::InlineText { .. }
        ));
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
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("post-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("post-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b, &stack_b);

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
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("rebind-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("rebind-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b, &stack_b);
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
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("reply-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("reply-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b, &stack_b);
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
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("multi-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("multi-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b, &stack_b);
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
