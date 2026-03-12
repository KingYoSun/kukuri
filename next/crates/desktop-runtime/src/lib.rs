mod identity;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use next_app_api::{AppService, SyncStatus, TimelineView};
use next_blob_service::IrohBlobService;
use next_docs_sync::{IrohDocsNode, IrohDocsSync};
use next_store::{SqliteStore, TimelineCursor};
use next_transport::{IrohGossipTransport, TransportNetworkConfig};
use serde::{Deserialize, Serialize};

use crate::identity::{IdentityStorageMode, load_or_create_keys};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePostRequest {
    pub topic: String,
    pub content: String,
    pub reply_to: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListTimelineRequest {
    pub topic: String,
    pub cursor: Option<TimelineCursor>,
    pub limit: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListThreadRequest {
    pub topic: String,
    pub thread_id: String,
    pub cursor: Option<TimelineCursor>,
    pub limit: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportPeerTicketRequest {
    pub ticket: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsubscribeTopicRequest {
    pub topic: String,
}

pub struct DesktopRuntime {
    app_service: AppService,
    db_path: PathBuf,
    _iroh_stack: SharedIrohStack,
}

struct SharedIrohStack {
    _node: Arc<IrohDocsNode>,
    transport: Arc<IrohGossipTransport>,
    docs_sync: Arc<IrohDocsSync>,
    blob_service: Arc<IrohBlobService>,
}

impl DesktopRuntime {
    pub async fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        Self::new_with_config_and_identity(
            db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::from_env(),
        )
        .await
    }

    pub async fn new_with_config(
        db_path: impl AsRef<Path>,
        network_config: TransportNetworkConfig,
    ) -> Result<Self> {
        Self::new_with_config_and_identity(db_path, network_config, IdentityStorageMode::from_env())
            .await
    }

    async fn new_with_config_and_identity(
        db_path: impl AsRef<Path>,
        network_config: TransportNetworkConfig,
        identity_mode: IdentityStorageMode,
    ) -> Result<Self> {
        let db_path = db_path.as_ref().to_path_buf();
        let docs_root = db_path.with_extension("iroh-data");
        let store = Arc::new(SqliteStore::connect_file(&db_path).await?);
        let iroh_stack = SharedIrohStack::new(&docs_root, network_config.clone()).await?;
        let keys = load_or_create_keys(&db_path, identity_mode)?;
        let app_service = AppService::new_with_services(
            store.clone(),
            store,
            iroh_stack.transport.clone(),
            iroh_stack.transport.clone(),
            iroh_stack.docs_sync.clone(),
            iroh_stack.blob_service.clone(),
            keys,
        );

        Ok(Self {
            app_service,
            db_path,
            _iroh_stack: iroh_stack,
        })
    }

    pub async fn from_env(db_path: impl AsRef<Path>) -> Result<Self> {
        Self::new_with_config(db_path, TransportNetworkConfig::from_env()?).await
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub async fn create_post(&self, request: CreatePostRequest) -> Result<String> {
        self.app_service
            .create_post(
                request.topic.as_str(),
                request.content.as_str(),
                request.reply_to.as_deref(),
            )
            .await
    }

    pub async fn list_timeline(&self, request: ListTimelineRequest) -> Result<TimelineView> {
        self.app_service
            .list_timeline(
                request.topic.as_str(),
                request.cursor,
                request.limit.unwrap_or(50),
            )
            .await
    }

    pub async fn list_thread(&self, request: ListThreadRequest) -> Result<TimelineView> {
        self.app_service
            .list_thread(
                request.topic.as_str(),
                request.thread_id.as_str(),
                request.cursor,
                request.limit.unwrap_or(50),
            )
            .await
    }

    pub async fn get_sync_status(&self) -> Result<SyncStatus> {
        self.app_service.get_sync_status().await
    }

    pub async fn import_peer_ticket(&self, request: ImportPeerTicketRequest) -> Result<()> {
        self.app_service
            .import_peer_ticket(request.ticket.as_str())
            .await
    }

    pub async fn unsubscribe_topic(&self, request: UnsubscribeTopicRequest) -> Result<()> {
        self.app_service
            .unsubscribe_topic(request.topic.as_str())
            .await
    }

    pub async fn local_peer_ticket(&self) -> Result<Option<String>> {
        self.app_service.peer_ticket().await
    }

    pub async fn shutdown(&self) {
        self.app_service.shutdown().await;
    }
}

impl SharedIrohStack {
    async fn new(root: &Path, network_config: TransportNetworkConfig) -> Result<Self> {
        let node = IrohDocsNode::persistent_with_config(root, network_config.clone()).await?;
        let transport = Arc::new(IrohGossipTransport::from_shared_parts(
            node.endpoint().clone(),
            node.gossip().clone(),
            node.discovery(),
            network_config,
        ));
        let docs_sync = Arc::new(IrohDocsSync::new(node.clone()));
        let blob_service = Arc::new(IrohBlobService::new(node.clone()));
        Ok(Self {
            _node: node,
            transport,
            docs_sync,
            blob_service,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::time::{Duration, sleep, timeout};

    #[tokio::test]
    async fn desktop_runtime_persists_posts_and_author_identity_after_restart() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("kukuri-next.db");
        let runtime = DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime");
        let event_id = runtime
            .create_post(CreatePostRequest {
                topic: "kukuri:topic:runtime".into(),
                content: "persist me".into(),
                reply_to: None,
            })
            .await
            .expect("create post");
        drop(runtime);

        let restarted = DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime restart");
        let restarted_event_id = restarted
            .create_post(CreatePostRequest {
                topic: "kukuri:topic:runtime".into(),
                content: "persist me again".into(),
                reply_to: None,
            })
            .await
            .expect("create post after restart");
        let timeline = restarted
            .list_timeline(ListTimelineRequest {
                topic: "kukuri:topic:runtime".into(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("timeline");

        assert!(timeline.items.iter().any(|post| post.id == event_id));
        assert!(
            timeline
                .items
                .iter()
                .any(|post| post.id == restarted_event_id)
        );
        let original_post = timeline
            .items
            .iter()
            .find(|post| post.id == event_id)
            .expect("original post");
        let restarted_post = timeline
            .items
            .iter()
            .find(|post| post.id == restarted_event_id)
            .expect("restarted post");
        assert_eq!(original_post.author_pubkey, restarted_post.author_pubkey);
        assert_eq!(restarted.db_path(), db_path.as_path());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn desktop_runtime_syncs_between_instances() {
        let dir = tempdir().expect("tempdir");
        let db_a = dir.path().join("a.db");
        let db_b = dir.path().join("b.db");
        let runtime_a = DesktopRuntime::new_with_config_and_identity(
            &db_a,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime a");
        let runtime_b = DesktopRuntime::new_with_config_and_identity(
            &db_b,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime b");
        let ticket_a = runtime_a
            .local_peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = runtime_b
            .local_peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");

        runtime_a
            .import_peer_ticket(ImportPeerTicketRequest { ticket: ticket_b })
            .await
            .expect("import b");
        runtime_b
            .import_peer_ticket(ImportPeerTicketRequest { ticket: ticket_a })
            .await
            .expect("import a");

        let topic = "kukuri:topic:desktop-runtime";
        let _ = runtime_b
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe b");
        let event_id = runtime_a
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "hello desktop runtime".into(),
                reply_to: None,
            })
            .await
            .expect("create post");

        let received = timeout(Duration::from_secs(10), async {
            loop {
                let timeline = runtime_b
                    .list_timeline(ListTimelineRequest {
                        topic: topic.into(),
                        cursor: None,
                        limit: Some(20),
                    })
                    .await
                    .expect("timeline");
                if let Some(post) = timeline.items.iter().find(|post| post.id == event_id) {
                    return post.clone();
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("runtime sync timeout");

        assert_eq!(received.content, "hello desktop runtime");
        let status = runtime_b.get_sync_status().await.expect("sync status");
        assert!(status.last_sync_ts.is_some());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn late_joiner_backfills_timeline_from_docs() {
        let dir = tempdir().expect("tempdir");
        let db_a = dir.path().join("late-a.db");
        let db_b = dir.path().join("late-b.db");
        let runtime_a = DesktopRuntime::new_with_config_and_identity(
            &db_a,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime a");
        let topic = "kukuri:topic:late-join";
        let event_id = runtime_a
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "hello from before join".into(),
                reply_to: None,
            })
            .await
            .expect("create post before join");
        let ticket_a = runtime_a
            .local_peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");

        let runtime_b = DesktopRuntime::new_with_config_and_identity(
            &db_b,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime b");
        runtime_b
            .import_peer_ticket(ImportPeerTicketRequest { ticket: ticket_a })
            .await
            .expect("import a into b");

        let received = timeout(Duration::from_secs(10), async {
            loop {
                let timeline = runtime_b
                    .list_timeline(ListTimelineRequest {
                        topic: topic.into(),
                        cursor: None,
                        limit: Some(20),
                    })
                    .await
                    .expect("timeline b");
                if let Some(post) = timeline.items.iter().find(|post| post.id == event_id) {
                    return post.clone();
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("late join timeout");

        assert_eq!(received.content, "hello from before join");
    }

    #[tokio::test]
    async fn sqlite_deletion_does_not_lose_shared_state() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("delete-sqlite.db");
        let runtime = DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime");
        let topic = "kukuri:topic:sqlite-delete";
        let root_id = runtime
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "root".into(),
                reply_to: None,
            })
            .await
            .expect("root post");
        let reply_id = runtime
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "reply".into(),
                reply_to: Some(root_id.clone()),
            })
            .await
            .expect("reply post");
        runtime.shutdown().await;
        drop(runtime);
        std::fs::remove_file(&db_path).expect("delete sqlite");

        let restarted = DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("restart");
        let timeline = restarted
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("timeline");
        let thread = restarted
            .list_thread(ListThreadRequest {
                topic: topic.into(),
                thread_id: root_id.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("thread");

        assert!(timeline.items.iter().any(|post| post.id == root_id));
        assert!(timeline.items.iter().any(|post| post.id == reply_id));
        assert!(thread.items.iter().any(|post| post.id == root_id));
        assert!(thread.items.iter().any(|post| post.id == reply_id));
    }

    #[tokio::test]
    async fn restart_restores_from_docs_blobs_without_sqlite_seed() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("restart-no-seed.db");
        let runtime = DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime");
        let topic = "kukuri:topic:restart-no-seed";
        let event_id = runtime
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "restored from docs".into(),
                reply_to: None,
            })
            .await
            .expect("create post");
        runtime.shutdown().await;
        drop(runtime);
        std::fs::remove_file(&db_path).expect("delete sqlite");

        let restarted = DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("restart");
        let timeline = restarted
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("timeline");

        let restored = timeline
            .items
            .iter()
            .find(|post| post.id == event_id)
            .expect("restored post");
        assert_eq!(restored.content, "restored from docs");
    }
}
