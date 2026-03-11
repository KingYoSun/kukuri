use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use next_app_api::{AppService, SyncStatus, TimelineView};
use next_store::{SqliteStore, TimelineCursor};
use next_transport::{IrohGossipTransport, TransportNetworkConfig};
use serde::{Deserialize, Serialize};

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
}

impl DesktopRuntime {
    pub async fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        Self::new_with_config(db_path, TransportNetworkConfig::loopback()).await
    }

    pub async fn new_with_config(
        db_path: impl AsRef<Path>,
        network_config: TransportNetworkConfig,
    ) -> Result<Self> {
        let db_path = db_path.as_ref().to_path_buf();
        let store = Arc::new(SqliteStore::connect_file(&db_path).await?);
        let transport = Arc::new(IrohGossipTransport::bind(network_config).await?);
        let app_service = AppService::new(store, transport);

        Ok(Self {
            app_service,
            db_path,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::time::{Duration, sleep, timeout};

    #[tokio::test]
    async fn desktop_runtime_persists_posts_after_restart() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("kukuri-next.db");
        let runtime = DesktopRuntime::new(&db_path).await.expect("runtime");
        let event_id = runtime
            .create_post(CreatePostRequest {
                topic: "kukuri:topic:runtime".into(),
                content: "persist me".into(),
                reply_to: None,
            })
            .await
            .expect("create post");
        drop(runtime);

        let restarted = DesktopRuntime::new(&db_path)
            .await
            .expect("runtime restart");
        let timeline = restarted
            .list_timeline(ListTimelineRequest {
                topic: "kukuri:topic:runtime".into(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("timeline");

        assert!(timeline.items.iter().any(|post| post.id == event_id));
        assert_eq!(restarted.db_path(), db_path.as_path());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn desktop_runtime_syncs_between_instances() {
        let dir = tempdir().expect("tempdir");
        let db_a = dir.path().join("a.db");
        let db_b = dir.path().join("b.db");
        let runtime_a = DesktopRuntime::new(&db_a).await.expect("runtime a");
        let runtime_b = DesktopRuntime::new(&db_b).await.expect("runtime b");
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
}
