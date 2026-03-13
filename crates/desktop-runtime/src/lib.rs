mod identity;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use kukuri_app_api::{AppService, PendingAttachment, SyncStatus, TimelineView};
use kukuri_blob_service::IrohBlobService;
use kukuri_core::AssetRole;
use kukuri_docs_sync::{IrohDocsNode, IrohDocsSync};
use kukuri_store::{SqliteStore, TimelineCursor};
use kukuri_transport::{IrohGossipTransport, TransportNetworkConfig};
use serde::{Deserialize, Serialize};

use crate::identity::{IdentityStorageMode, load_or_create_keys};

const DB_FILE_NAME: &str = "kukuri.db";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePostRequest {
    pub topic: String,
    pub content: String,
    pub reply_to: Option<String>,
    #[serde(default)]
    pub attachments: Vec<CreateAttachmentRequest>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateAttachmentRequest {
    pub file_name: Option<String>,
    pub mime: String,
    pub byte_size: u64,
    pub data_base64: String,
    pub role: Option<String>,
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetBlobPreviewRequest {
    pub hash: String,
    pub mime: String,
}

pub struct DesktopRuntime {
    app_service: AppService,
    db_path: PathBuf,
    iroh_stack: SharedIrohStack,
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
        migrate_legacy_runtime_data(&db_path)?;
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
            iroh_stack,
        })
    }

    pub async fn from_env(db_path: impl AsRef<Path>) -> Result<Self> {
        Self::new_with_config(db_path, TransportNetworkConfig::from_env()?).await
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub async fn create_post(&self, request: CreatePostRequest) -> Result<String> {
        let attachments = request
            .attachments
            .into_iter()
            .map(pending_attachment_from_request)
            .collect::<Result<Vec<_>>>()?;
        self.app_service
            .create_post_with_attachments(
                request.topic.as_str(),
                request.content.as_str(),
                request.reply_to.as_deref(),
                attachments,
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

    pub async fn get_blob_preview_url(
        &self,
        request: GetBlobPreviewRequest,
    ) -> Result<Option<String>> {
        self.app_service
            .blob_preview_data_url(request.hash.as_str(), request.mime.as_str())
            .await
    }

    pub async fn shutdown(&self) {
        self.app_service.shutdown().await;
        self.iroh_stack.shutdown().await;
    }
}

pub fn resolve_db_path_from_env(base_app_data_dir: &Path) -> Result<PathBuf> {
    let mut app_data_dir = std::env::var("KUKURI_APP_DATA_DIR")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| base_app_data_dir.to_path_buf());

    if app_data_dir == base_app_data_dir
        && let Some(instance) = std::env::var("KUKURI_INSTANCE")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    {
        app_data_dir = app_data_dir.join(instance);
    }

    fs::create_dir_all(&app_data_dir)
        .with_context(|| format!("failed to create app data dir `{}`", app_data_dir.display()))?;

    let db_path = app_data_dir.join(DB_FILE_NAME);
    migrate_legacy_runtime_data(&db_path)?;
    Ok(db_path)
}

fn migrate_legacy_runtime_data(db_path: &Path) -> Result<()> {
    let parent = db_path.parent().ok_or_else(|| {
        anyhow::anyhow!(
            "runtime db path `{}` has no parent directory",
            db_path.display()
        )
    })?;
    let legacy_db_path = parent.join(legacy_db_file_name());

    migrate_if_missing(&legacy_db_path, db_path)?;
    migrate_if_missing(
        &legacy_db_path.with_extension("iroh-data"),
        &db_path.with_extension("iroh-data"),
    )?;
    migrate_if_missing(
        &legacy_db_path.with_extension("nsec"),
        &db_path.with_extension("nsec"),
    )?;
    migrate_if_missing(
        &legacy_db_path.with_extension("identity-store"),
        &db_path.with_extension("identity-store"),
    )?;

    Ok(())
}

fn migrate_if_missing(old_path: &Path, new_path: &Path) -> Result<()> {
    if new_path.exists() || !old_path.exists() {
        return Ok(());
    }

    fs::rename(old_path, new_path).with_context(|| {
        format!(
            "failed to migrate `{}` to `{}`",
            old_path.display(),
            new_path.display()
        )
    })
}

fn legacy_db_file_name() -> String {
    format!("kukuri-{}.db", "next")
}

fn pending_attachment_from_request(request: CreateAttachmentRequest) -> Result<PendingAttachment> {
    let bytes = BASE64_STANDARD
        .decode(request.data_base64.as_bytes())
        .context("failed to decode attachment data")?;
    let role = match request.role.as_deref() {
        Some("image_preview") => AssetRole::ImagePreview,
        Some("video_poster") => AssetRole::VideoPoster,
        Some("video_manifest") => AssetRole::VideoManifest,
        Some("attachment") => AssetRole::Attachment,
        _ => AssetRole::ImageOriginal,
    };
    Ok(PendingAttachment {
        mime: request.mime,
        bytes,
        role,
    })
}

#[cfg(test)]
fn legacy_iroh_data_dir_name() -> String {
    format!("kukuri-{}.iroh-data", "next")
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

    async fn shutdown(&self) {
        let _ = self._node.clone().shutdown().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
    use std::sync::{Mutex, OnceLock};
    use tempfile::tempdir;
    use tokio::time::{Duration, sleep, timeout};

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock")
    }

    fn clear_runtime_env() {
        let legacy_app_data_dir = legacy_env("APP_DATA_DIR");
        let legacy_instance = legacy_env("INSTANCE");
        for key in [
            "KUKURI_APP_DATA_DIR",
            "KUKURI_INSTANCE",
            legacy_app_data_dir.as_str(),
            legacy_instance.as_str(),
        ] {
            unsafe { std::env::remove_var(key) };
        }
    }

    fn legacy_env(name: &str) -> String {
        format!("KUKURI_{}_{}", "NEXT", name)
    }

    fn image_attachment_request(name: &str, mime: &str, bytes: &[u8]) -> CreateAttachmentRequest {
        CreateAttachmentRequest {
            file_name: Some(name.to_string()),
            mime: mime.to_string(),
            byte_size: bytes.len() as u64,
            data_base64: BASE64_STANDARD.encode(bytes),
            role: Some("image_original".to_string()),
        }
    }

    #[test]
    fn legacy_next_db_migrates_to_kukuri_db() {
        let _guard = env_lock();
        clear_runtime_env();
        let dir = tempdir().expect("tempdir");
        let legacy_db_path = dir.path().join(legacy_db_file_name());
        fs::write(&legacy_db_path, b"sqlite").expect("legacy db");

        let resolved = resolve_db_path_from_env(dir.path()).expect("resolved db path");

        assert_eq!(resolved, dir.path().join("kukuri.db"));
        assert!(resolved.exists());
        assert!(!legacy_db_path.exists());
    }

    #[test]
    fn legacy_next_iroh_data_migrates_to_kukuri_data_dir() {
        let _guard = env_lock();
        clear_runtime_env();
        let dir = tempdir().expect("tempdir");
        let legacy_data_dir = dir.path().join(legacy_iroh_data_dir_name());
        fs::create_dir_all(&legacy_data_dir).expect("legacy data dir");
        fs::write(legacy_data_dir.join("blob.bin"), b"blob").expect("legacy blob");

        let resolved = resolve_db_path_from_env(dir.path()).expect("resolved db path");
        let new_data_dir = resolved.with_extension("iroh-data");

        assert!(new_data_dir.join("blob.bin").exists());
        assert!(!legacy_data_dir.exists());
    }

    #[tokio::test]
    async fn desktop_runtime_persists_posts_and_author_identity_after_restart() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("kukuri.db");
        let runtime = timeout(
            Duration::from_secs(15),
            DesktopRuntime::new_with_config_and_identity(
                &db_path,
                TransportNetworkConfig::loopback(),
                IdentityStorageMode::FileOnly,
            ),
        )
        .await
        .expect("runtime creation timeout")
        .expect("runtime");
        let event_id = runtime
            .create_post(CreatePostRequest {
                topic: "kukuri:topic:runtime".into(),
                content: "persist me".into(),
                reply_to: None,
                attachments: vec![],
            })
            .await
            .expect("create post");
        timeout(Duration::from_secs(15), runtime.shutdown())
            .await
            .expect("runtime shutdown timeout");
        drop(runtime);

        let restarted = timeout(
            Duration::from_secs(15),
            DesktopRuntime::new_with_config_and_identity(
                &db_path,
                TransportNetworkConfig::loopback(),
                IdentityStorageMode::FileOnly,
            ),
        )
        .await
        .expect("runtime restart timeout")
        .expect("runtime restart");
        let restarted_event_id = restarted
            .create_post(CreatePostRequest {
                topic: "kukuri:topic:runtime".into(),
                content: "persist me again".into(),
                reply_to: None,
                attachments: vec![],
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
        timeout(Duration::from_secs(15), restarted.shutdown())
            .await
            .expect("restarted shutdown timeout");
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
                attachments: vec![],
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
                attachments: vec![],
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn late_joiner_backfills_image_post_from_docs() {
        let dir = tempdir().expect("tempdir");
        let db_a = dir.path().join("late-image-a.db");
        let db_b = dir.path().join("late-image-b.db");
        let runtime_a = DesktopRuntime::new_with_config_and_identity(
            &db_a,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime a");
        let topic = "kukuri:topic:late-image-runtime";
        let event_id = runtime_a
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "late image".into(),
                reply_to: None,
                attachments: vec![image_attachment_request(
                    "late.png",
                    "image/png",
                    b"late-image-runtime",
                )],
            })
            .await
            .expect("create image post before join");
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
        .expect("late image timeout");

        assert_eq!(received.attachments.len(), 1);
        let preview = runtime_b
            .get_blob_preview_url(GetBlobPreviewRequest {
                hash: received.attachments[0].hash.clone(),
                mime: received.attachments[0].mime.clone(),
            })
            .await
            .expect("blob preview");
        assert!(preview.is_some());
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
                attachments: vec![],
            })
            .await
            .expect("root post");
        let reply_id = runtime
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "reply".into(),
                reply_to: Some(root_id.clone()),
                attachments: vec![],
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
                attachments: vec![],
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

    #[tokio::test]
    async fn restart_restores_image_post_preview() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("restart-image.db");
        let runtime = DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime");
        let topic = "kukuri:topic:restart-image";
        let event_id = runtime
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "restored image".into(),
                reply_to: None,
                attachments: vec![image_attachment_request(
                    "restored.png",
                    "image/png",
                    b"restart-image-preview",
                )],
            })
            .await
            .expect("create image post");
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
            .expect("restored image post");

        assert_eq!(restored.attachments.len(), 1);
        let preview = restarted
            .get_blob_preview_url(GetBlobPreviewRequest {
                hash: restored.attachments[0].hash.clone(),
                mime: restored.attachments[0].mime.clone(),
            })
            .await
            .expect("preview after restart");
        assert!(preview.is_some());
    }
}
