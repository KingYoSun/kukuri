mod identity;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use kukuri_app_api::{
    AppService, BlobMediaPayload, CreateGameRoomInput, CreateLiveSessionInput, GameRoomView,
    GameScoreView, LiveSessionView, PendingAttachment, SyncStatus, TimelineView,
    UpdateGameRoomInput,
};
use kukuri_blob_service::{BlobService, IrohBlobService};
use kukuri_core::{AssetRole, GameRoomStatus};
use kukuri_docs_sync::{DocsSync, IrohDocsNode, IrohDocsSync};
use kukuri_store::{SqliteStore, TimelineCursor};
use kukuri_transport::{
    ConnectMode, DhtDiscoveryOptions, DiscoveryMode, IrohGossipTransport, SeedPeer, Transport,
    TransportNetworkConfig, parse_seed_peer,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::identity::{IdentityStorageMode, load_or_create_keys};

const DB_FILE_NAME: &str = "kukuri.db";
const DISCOVERY_CONFIG_FILE_EXTENSION: &str = "discovery.json";
const DISCOVERY_MODE_ENV: &str = "KUKURI_DISCOVERY_MODE";
const DISCOVERY_SEEDS_ENV: &str = "KUKURI_DISCOVERY_SEEDS";

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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetBlobMediaRequest {
    pub hash: String,
    pub mime: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListLiveSessionsRequest {
    pub topic: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateLiveSessionRequest {
    pub topic: String,
    pub title: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveSessionCommandRequest {
    pub topic: String,
    pub session_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListGameRoomsRequest {
    pub topic: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateGameRoomRequest {
    pub topic: String,
    pub title: String,
    pub description: String,
    pub participants: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateGameRoomRequest {
    pub topic: String,
    pub room_id: String,
    pub status: GameRoomStatus,
    pub phase_label: Option<String>,
    pub scores: Vec<GameScoreView>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    pub mode: DiscoveryMode,
    pub connect_mode: ConnectMode,
    pub env_locked: bool,
    pub seed_peers: Vec<SeedPeer>,
}

impl DiscoveryConfig {
    fn static_peer_default() -> Self {
        Self {
            mode: DiscoveryMode::StaticPeer,
            connect_mode: ConnectMode::DirectOnly,
            env_locked: false,
            seed_peers: Vec::new(),
        }
    }

    fn seeded_dht_default() -> Self {
        Self {
            mode: DiscoveryMode::SeededDht,
            connect_mode: ConnectMode::DirectOnly,
            env_locked: false,
            seed_peers: Vec::new(),
        }
    }

    fn from_stored(stored: StoredDiscoveryConfig, env_locked: bool) -> Self {
        Self {
            mode: stored.mode,
            connect_mode: ConnectMode::DirectOnly,
            env_locked,
            seed_peers: normalize_seed_peers(stored.seed_peers),
        }
    }

    fn stored(&self) -> StoredDiscoveryConfig {
        StoredDiscoveryConfig {
            mode: self.mode.clone(),
            seed_peers: normalize_seed_peers(self.seed_peers.clone()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetDiscoverySeedsRequest {
    pub seed_entries: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct StoredDiscoveryConfig {
    #[serde(default)]
    mode: DiscoveryMode,
    #[serde(default)]
    seed_peers: Vec<SeedPeer>,
}

pub struct DesktopRuntime {
    app_service: AppService,
    db_path: PathBuf,
    store: Arc<SqliteStore>,
    iroh_stack: SharedIrohStack,
    discovery_config: Arc<Mutex<DiscoveryConfig>>,
}

struct SharedIrohStack {
    _node: Arc<IrohDocsNode>,
    transport: Arc<IrohGossipTransport>,
    docs_sync: Arc<IrohDocsSync>,
    blob_service: Arc<IrohBlobService>,
}

impl DesktopRuntime {
    pub async fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        Self::new_with_config_and_identity_and_discovery(
            db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::from_env(),
            DiscoveryConfig::static_peer_default(),
            DhtDiscoveryOptions::disabled(),
        )
        .await
    }

    pub async fn new_with_config(
        db_path: impl AsRef<Path>,
        network_config: TransportNetworkConfig,
    ) -> Result<Self> {
        Self::new_with_config_and_identity_and_discovery(
            db_path,
            network_config,
            IdentityStorageMode::from_env(),
            DiscoveryConfig::static_peer_default(),
            DhtDiscoveryOptions::disabled(),
        )
        .await
    }

    #[cfg(test)]
    async fn new_with_config_and_identity(
        db_path: impl AsRef<Path>,
        network_config: TransportNetworkConfig,
        identity_mode: IdentityStorageMode,
    ) -> Result<Self> {
        Self::new_with_config_and_identity_and_discovery(
            db_path,
            network_config,
            identity_mode,
            DiscoveryConfig::static_peer_default(),
            DhtDiscoveryOptions::disabled(),
        )
        .await
    }

    async fn new_with_config_and_identity_and_discovery(
        db_path: impl AsRef<Path>,
        network_config: TransportNetworkConfig,
        identity_mode: IdentityStorageMode,
        discovery_config: DiscoveryConfig,
        dht_options: DhtDiscoveryOptions,
    ) -> Result<Self> {
        let db_path = db_path.as_ref().to_path_buf();
        migrate_legacy_runtime_data(&db_path)?;
        let docs_root = db_path.with_extension("iroh-data");
        let store = Arc::new(SqliteStore::connect_file(&db_path).await?);
        let iroh_stack = SharedIrohStack::new(
            &docs_root,
            network_config.clone(),
            &discovery_config,
            dht_options,
        )
        .await?;
        let keys = load_or_create_keys(&db_path, identity_mode)?;
        let app_service = AppService::new_with_services(
            store.clone(),
            store.clone(),
            iroh_stack.transport.clone(),
            iroh_stack.transport.clone(),
            iroh_stack.docs_sync.clone(),
            iroh_stack.blob_service.clone(),
            keys,
        );

        Ok(Self {
            app_service,
            db_path,
            store,
            iroh_stack,
            discovery_config: Arc::new(Mutex::new(discovery_config)),
        })
    }

    pub async fn from_env(db_path: impl AsRef<Path>) -> Result<Self> {
        let db_path = db_path.as_ref().to_path_buf();
        migrate_legacy_runtime_data(&db_path)?;
        let discovery_config = resolve_discovery_config_from_env(&db_path)?;
        let dht_options = match discovery_config.mode {
            DiscoveryMode::SeededDht => DhtDiscoveryOptions::seeded_dht(),
            DiscoveryMode::StaticPeer => DhtDiscoveryOptions::disabled(),
        };
        Self::new_with_config_and_identity_and_discovery(
            &db_path,
            TransportNetworkConfig::from_env()?,
            IdentityStorageMode::from_env(),
            discovery_config,
            dht_options,
        )
        .await
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

    pub async fn get_discovery_config(&self) -> Result<DiscoveryConfig> {
        Ok(self.discovery_config.lock().await.clone())
    }

    pub async fn list_live_sessions(
        &self,
        request: ListLiveSessionsRequest,
    ) -> Result<Vec<LiveSessionView>> {
        self.app_service
            .list_live_sessions(request.topic.as_str())
            .await
    }

    pub async fn create_live_session(&self, request: CreateLiveSessionRequest) -> Result<String> {
        self.app_service
            .create_live_session(
                request.topic.as_str(),
                CreateLiveSessionInput {
                    title: request.title,
                    description: request.description,
                },
            )
            .await
    }

    pub async fn end_live_session(&self, request: LiveSessionCommandRequest) -> Result<()> {
        self.app_service
            .end_live_session(request.topic.as_str(), request.session_id.as_str())
            .await
    }

    pub async fn join_live_session(&self, request: LiveSessionCommandRequest) -> Result<()> {
        self.app_service
            .join_live_session(request.topic.as_str(), request.session_id.as_str())
            .await
    }

    pub async fn leave_live_session(&self, request: LiveSessionCommandRequest) -> Result<()> {
        self.app_service
            .leave_live_session(request.topic.as_str(), request.session_id.as_str())
            .await
    }

    pub async fn list_game_rooms(
        &self,
        request: ListGameRoomsRequest,
    ) -> Result<Vec<GameRoomView>> {
        self.app_service
            .list_game_rooms(request.topic.as_str())
            .await
    }

    pub async fn create_game_room(&self, request: CreateGameRoomRequest) -> Result<String> {
        self.app_service
            .create_game_room(
                request.topic.as_str(),
                CreateGameRoomInput {
                    title: request.title,
                    description: request.description,
                    participants: request.participants,
                },
            )
            .await
    }

    pub async fn update_game_room(&self, request: UpdateGameRoomRequest) -> Result<()> {
        self.app_service
            .update_game_room(
                request.topic.as_str(),
                request.room_id.as_str(),
                UpdateGameRoomInput {
                    status: request.status,
                    phase_label: request.phase_label,
                    scores: request.scores,
                },
            )
            .await
    }

    pub async fn import_peer_ticket(&self, request: ImportPeerTicketRequest) -> Result<()> {
        self.app_service
            .import_peer_ticket(request.ticket.as_str())
            .await
    }

    pub async fn set_discovery_seeds(
        &self,
        request: SetDiscoverySeedsRequest,
    ) -> Result<DiscoveryConfig> {
        let mut next_config = self.discovery_config.lock().await.clone();
        if next_config.env_locked {
            bail!("discovery configuration is locked by environment variables");
        }
        next_config.seed_peers = parse_seed_entries(&request.seed_entries)?;
        self.app_service
            .set_discovery_seeds(
                next_config.mode.clone(),
                next_config.env_locked,
                next_config.seed_peers.clone(),
            )
            .await?;
        save_discovery_config(&self.db_path, &next_config.stored())?;
        *self.discovery_config.lock().await = next_config.clone();
        Ok(next_config)
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

    pub async fn get_blob_media_payload(
        &self,
        request: GetBlobMediaRequest,
    ) -> Result<Option<BlobMediaPayload>> {
        self.app_service
            .blob_media_payload(request.hash.as_str(), request.mime.as_str())
            .await
    }

    pub async fn shutdown(&self) {
        self.app_service.shutdown().await;
        self.iroh_stack.shutdown().await;
        self.store.close().await;
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

fn discovery_config_path(db_path: &Path) -> PathBuf {
    db_path.with_extension(DISCOVERY_CONFIG_FILE_EXTENSION)
}

fn load_discovery_config_from_file(db_path: &Path) -> Result<Option<StoredDiscoveryConfig>> {
    let path = discovery_config_path(db_path);
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read discovery config `{}`", path.display()))?;
    let config = serde_json::from_str::<StoredDiscoveryConfig>(&raw)
        .with_context(|| format!("failed to parse discovery config `{}`", path.display()))?;
    Ok(Some(config))
}

fn save_discovery_config(db_path: &Path, config: &StoredDiscoveryConfig) -> Result<()> {
    let path = discovery_config_path(db_path);
    let json = serde_json::to_vec_pretty(config)
        .with_context(|| format!("failed to encode discovery config `{}`", path.display()))?;
    fs::write(&path, json)
        .with_context(|| format!("failed to write discovery config `{}`", path.display()))
}

fn resolve_discovery_config_from_env(db_path: &Path) -> Result<DiscoveryConfig> {
    let env_mode = std::env::var(DISCOVERY_MODE_ENV).ok();
    let env_seeds = std::env::var(DISCOVERY_SEEDS_ENV).ok();
    let env_locked = env_mode.is_some() || env_seeds.is_some();

    if env_locked {
        let mode = match env_mode.as_deref() {
            Some(value) => parse_discovery_mode(value)?,
            None => DiscoveryMode::SeededDht,
        };
        let seed_peers = parse_seed_entries_from_csv(env_seeds.as_deref().unwrap_or(""))?;
        return Ok(DiscoveryConfig {
            mode,
            connect_mode: ConnectMode::DirectOnly,
            env_locked: true,
            seed_peers,
        });
    }

    if let Some(stored) = load_discovery_config_from_file(db_path)? {
        return Ok(DiscoveryConfig::from_stored(stored, false));
    }

    Ok(DiscoveryConfig::seeded_dht_default())
}

fn parse_discovery_mode(value: &str) -> Result<DiscoveryMode> {
    match value.trim() {
        "static_peer" => Ok(DiscoveryMode::StaticPeer),
        "seeded_dht" => Ok(DiscoveryMode::SeededDht),
        other => Err(anyhow!(
            "invalid {} value `{}` (expected static_peer or seeded_dht)",
            DISCOVERY_MODE_ENV,
            other
        )),
    }
}

fn parse_seed_entries(entries: &[String]) -> Result<Vec<SeedPeer>> {
    parse_seed_entries_from_iter(entries.iter().map(String::as_str))
}

fn parse_seed_entries_from_csv(value: &str) -> Result<Vec<SeedPeer>> {
    parse_seed_entries_from_iter(value.split(','))
}

fn parse_seed_entries_from_iter<'a>(
    entries: impl IntoIterator<Item = &'a str>,
) -> Result<Vec<SeedPeer>> {
    let mut parsed = Vec::new();
    for entry in entries {
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            continue;
        }
        parsed.push(parse_seed_peer(trimmed)?);
    }
    Ok(normalize_seed_peers(parsed))
}

fn normalize_seed_peers(peers: Vec<SeedPeer>) -> Vec<SeedPeer> {
    let mut deduped = std::collections::BTreeMap::new();
    for peer in peers {
        deduped.insert(peer.display(), peer);
    }
    deduped.into_values().collect()
}

#[cfg(test)]
fn legacy_iroh_data_dir_name() -> String {
    format!("kukuri-{}.iroh-data", "next")
}

impl SharedIrohStack {
    async fn new(
        root: &Path,
        network_config: TransportNetworkConfig,
        discovery_config: &DiscoveryConfig,
        dht_options: DhtDiscoveryOptions,
    ) -> Result<Self> {
        let node = IrohDocsNode::persistent_with_discovery_config(
            root,
            network_config.clone(),
            dht_options,
        )
        .await?;
        let transport = Arc::new(IrohGossipTransport::from_shared_parts(
            node.endpoint().clone(),
            node.gossip().clone(),
            node.discovery(),
            network_config,
        ));
        let docs_sync = Arc::new(IrohDocsSync::new(node.clone()));
        let blob_service = Arc::new(IrohBlobService::new(node.clone()));
        transport
            .configure_discovery(
                discovery_config.mode.clone(),
                discovery_config.env_locked,
                discovery_config.seed_peers.clone(),
            )
            .await?;
        docs_sync
            .set_seed_peers(discovery_config.seed_peers.clone())
            .await?;
        blob_service
            .set_seed_peers(discovery_config.seed_peers.clone())
            .await?;
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
    use iroh::address_lookup::EndpointInfo;
    use pkarr::{Client as PkarrClient, mainline::Testnet};
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
            DISCOVERY_MODE_ENV,
            DISCOVERY_SEEDS_ENV,
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

    fn video_attachment_request(
        name: &str,
        mime: &str,
        bytes: &[u8],
        role: &str,
    ) -> CreateAttachmentRequest {
        CreateAttachmentRequest {
            file_name: Some(name.to_string()),
            mime: mime.to_string(),
            byte_size: bytes.len() as u64,
            data_base64: BASE64_STANDARD.encode(bytes),
            role: Some(role.to_string()),
        }
    }

    fn dht_test_client(testnet: &Testnet) -> PkarrClient {
        let mut builder = PkarrClient::builder();
        builder.no_default_network().bootstrap(&testnet.bootstrap);
        builder.build().expect("pkarr client")
    }

    async fn publish_runtime_endpoint_to_testnet(runtime: &DesktopRuntime, testnet: &Testnet) {
        let endpoint = runtime.iroh_stack._node.endpoint();
        let client = dht_test_client(testnet);
        let signed_packet = EndpointInfo::from(endpoint.addr())
            .to_pkarr_signed_packet(endpoint.secret_key(), 1)
            .expect("signed packet");
        client
            .publish(&signed_packet, None)
            .await
            .expect("publish endpoint info");
        let public_key =
            pkarr::PublicKey::try_from(endpoint.id().as_bytes()).expect("pkarr public key");
        let expected = signed_packet.as_bytes().clone();
        timeout(Duration::from_secs(5), async {
            loop {
                if client
                    .resolve_most_recent(&public_key)
                    .await
                    .as_ref()
                    .is_some_and(|packet| packet.as_bytes() == &expected)
                {
                    return;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("resolve published endpoint info");
    }

    fn seeded_dht_config(seed_peers: Vec<SeedPeer>) -> DiscoveryConfig {
        DiscoveryConfig {
            mode: DiscoveryMode::SeededDht,
            connect_mode: ConnectMode::DirectOnly,
            env_locked: false,
            seed_peers,
        }
    }

    async fn new_seeded_dht_runtime_with_config(
        db_path: &Path,
        testnet: &Testnet,
        discovery_config: DiscoveryConfig,
    ) -> DesktopRuntime {
        let runtime = DesktopRuntime::new_with_config_and_identity_and_discovery(
            db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
            discovery_config,
            DhtDiscoveryOptions::with_client(dht_test_client(testnet)),
        )
        .await
        .expect("seeded dht runtime");
        publish_runtime_endpoint_to_testnet(&runtime, testnet).await;
        runtime
    }

    async fn new_seeded_dht_runtime(db_path: &Path, testnet: &Testnet) -> DesktopRuntime {
        new_seeded_dht_runtime_with_config(db_path, testnet, seeded_dht_config(Vec::new())).await
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
    async fn set_discovery_seeds_reapplies_runtime_without_restart() {
        let dir = tempdir().expect("tempdir");
        let db_a = dir.path().join("seeded-a.db");
        let db_b = dir.path().join("seeded-b.db");
        let testnet = Testnet::new(5).expect("testnet");
        let runtime_a = new_seeded_dht_runtime(&db_a, &testnet).await;
        let runtime_b = new_seeded_dht_runtime(&db_b, &testnet).await;
        let endpoint_a = runtime_a
            .get_sync_status()
            .await
            .expect("status a")
            .discovery
            .local_endpoint_id;
        let endpoint_b = runtime_b
            .get_sync_status()
            .await
            .expect("status b")
            .discovery
            .local_endpoint_id;

        runtime_a
            .set_discovery_seeds(SetDiscoverySeedsRequest {
                seed_entries: vec![endpoint_b.clone()],
            })
            .await
            .expect("set seeds a");
        runtime_b
            .set_discovery_seeds(SetDiscoverySeedsRequest {
                seed_entries: vec![endpoint_a.clone()],
            })
            .await
            .expect("set seeds b");

        let config_a = runtime_a
            .get_discovery_config()
            .await
            .expect("discovery config a");
        let config_b = runtime_b
            .get_discovery_config()
            .await
            .expect("discovery config b");
        assert_eq!(config_a.seed_peers[0].endpoint_id, endpoint_b);
        assert_eq!(config_b.seed_peers[0].endpoint_id, endpoint_a);
        let topic = "kukuri:topic:runtime-seeded-dht";
        let _ = runtime_a
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe a");
        let _ = runtime_b
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe b");
        timeout(Duration::from_secs(20), async {
            loop {
                let status_a = runtime_a.get_sync_status().await.expect("status a");
                let status_b = runtime_b.get_sync_status().await.expect("status b");
                let ready_a = status_a
                    .topic_diagnostics
                    .iter()
                    .any(|topic_status| topic_status.topic == topic && topic_status.peer_count > 0);
                let ready_b = status_b
                    .topic_diagnostics
                    .iter()
                    .any(|topic_status| topic_status.topic == topic && topic_status.peer_count > 0);
                if ready_a && ready_b {
                    return;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("seeded runtime ready timeout");

        let event_id = runtime_a
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "hello seeded runtime".into(),
                reply_to: None,
                attachments: vec![],
            })
            .await
            .expect("create post");

        let received = timeout(Duration::from_secs(20), async {
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
        .expect("seeded runtime sync timeout");

        assert_eq!(received.content, "hello seeded runtime");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn restart_restores_seeded_dht_config_and_reconnects() {
        let dir = tempdir().expect("tempdir");
        let db_a = dir.path().join("restart-seeded-a.db");
        let db_b = dir.path().join("restart-seeded-b.db");
        let testnet = Testnet::new(5).expect("testnet");
        let runtime_a = new_seeded_dht_runtime(&db_a, &testnet).await;
        let runtime_b = new_seeded_dht_runtime(&db_b, &testnet).await;
        let endpoint_a = runtime_a
            .get_sync_status()
            .await
            .expect("status a")
            .discovery
            .local_endpoint_id;
        let endpoint_b = runtime_b
            .get_sync_status()
            .await
            .expect("status b")
            .discovery
            .local_endpoint_id;

        runtime_a
            .set_discovery_seeds(SetDiscoverySeedsRequest {
                seed_entries: vec![endpoint_b.clone()],
            })
            .await
            .expect("set seeds a");
        runtime_b
            .set_discovery_seeds(SetDiscoverySeedsRequest {
                seed_entries: vec![endpoint_a.clone()],
            })
            .await
            .expect("set seeds b");

        timeout(Duration::from_secs(15), runtime_a.shutdown())
            .await
            .expect("shutdown a");
        timeout(Duration::from_secs(15), runtime_b.shutdown())
            .await
            .expect("shutdown b");
        drop(runtime_a);
        drop(runtime_b);

        let restored_a =
            resolve_discovery_config_from_env(&db_a).expect("restored discovery config a");
        let restored_b =
            resolve_discovery_config_from_env(&db_b).expect("restored discovery config b");
        let restarted_a =
            new_seeded_dht_runtime_with_config(&db_a, &testnet, restored_a.clone()).await;
        let restarted_b =
            new_seeded_dht_runtime_with_config(&db_b, &testnet, restored_b.clone()).await;
        let restarted_endpoint_a = restarted_a
            .get_sync_status()
            .await
            .expect("restarted status a")
            .discovery
            .local_endpoint_id;
        let restarted_endpoint_b = restarted_b
            .get_sync_status()
            .await
            .expect("restarted status b")
            .discovery
            .local_endpoint_id;

        assert_eq!(restored_a.mode, DiscoveryMode::SeededDht);
        assert_eq!(restored_b.mode, DiscoveryMode::SeededDht);
        assert_eq!(restored_a.seed_peers[0].endpoint_id, endpoint_b);
        assert_eq!(restored_b.seed_peers[0].endpoint_id, endpoint_a);
        assert_eq!(restarted_endpoint_a, endpoint_a);
        assert_eq!(restarted_endpoint_b, endpoint_b);
        let topic = "kukuri:topic:runtime-seeded-restart";
        let _ = restarted_a
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe restarted a");
        let _ = restarted_b
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe restarted b");

        let event_id = restarted_a
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "hello after restart".into(),
                reply_to: None,
                attachments: vec![],
            })
            .await
            .expect("create post");

        let received = timeout(Duration::from_secs(20), async {
            loop {
                let timeline = restarted_b
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
        .expect("restart seeded dht sync timeout");

        assert_eq!(received.content, "hello after restart");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn invalid_seed_entry_rejected_without_mutating_runtime() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("invalid-seed.db");
        let testnet = Testnet::new(5).expect("testnet");
        let runtime = new_seeded_dht_runtime(&db_path, &testnet).await;

        let error = runtime
            .set_discovery_seeds(SetDiscoverySeedsRequest {
                seed_entries: vec!["not-a-node-id".into()],
            })
            .await
            .expect_err("invalid seed should fail");
        assert!(error.to_string().contains("invalid seed endpoint id"));

        let config = runtime
            .get_discovery_config()
            .await
            .expect("discovery config");
        assert!(config.seed_peers.is_empty());
        assert!(!discovery_config_path(&db_path).exists());
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn late_joiner_backfills_video_media_payload() {
        let dir = tempdir().expect("tempdir");
        let db_a = dir.path().join("late-video-a.db");
        let db_b = dir.path().join("late-video-b.db");
        let runtime_a = DesktopRuntime::new_with_config_and_identity(
            &db_a,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime a");
        let topic = "kukuri:topic:late-video-runtime";
        let event_id = runtime_a
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "late video".into(),
                reply_to: None,
                attachments: vec![
                    video_attachment_request(
                        "late-video.mp4",
                        "video/mp4",
                        b"late-video-runtime",
                        "video_manifest",
                    ),
                    video_attachment_request(
                        "late-poster.jpg",
                        "image/jpeg",
                        b"late-video-poster",
                        "video_poster",
                    ),
                ],
            })
            .await
            .expect("create video post before join");
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
        .expect("late video timeout");

        let poster = received
            .attachments
            .iter()
            .find(|attachment| attachment.role == "video_poster")
            .expect("video poster");
        let preview = runtime_b
            .get_blob_media_payload(GetBlobMediaRequest {
                hash: poster.hash.clone(),
                mime: poster.mime.clone(),
            })
            .await
            .expect("video poster payload");
        assert!(preview.is_some());
        let manifest = received
            .attachments
            .iter()
            .find(|attachment| attachment.role == "video_manifest")
            .expect("video manifest");
        let playback = runtime_b
            .get_blob_media_payload(GetBlobMediaRequest {
                hash: manifest.hash.clone(),
                mime: manifest.mime.clone(),
            })
            .await
            .expect("video playback payload");
        assert!(playback.is_some());
    }

    #[tokio::test]
    async fn blob_media_payload_roundtrip() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("blob-media-roundtrip.db");
        let runtime = DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime");
        let topic = "kukuri:topic:blob-media-roundtrip";
        let event_id = runtime
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "roundtrip".into(),
                reply_to: None,
                attachments: vec![image_attachment_request(
                    "roundtrip.png",
                    "image/png",
                    b"blob-media-roundtrip",
                )],
            })
            .await
            .expect("create image post");
        let timeline = runtime
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("timeline");
        let created = timeline
            .items
            .iter()
            .find(|post| post.id == event_id)
            .expect("created post");

        let payload = runtime
            .get_blob_media_payload(GetBlobMediaRequest {
                hash: created.attachments[0].hash.clone(),
                mime: created.attachments[0].mime.clone(),
            })
            .await
            .expect("blob media payload")
            .expect("blob media payload present");

        assert_eq!(payload.mime, "image/png");
        assert_eq!(
            payload.bytes_base64,
            BASE64_STANDARD.encode(b"blob-media-roundtrip")
        );
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

    #[tokio::test]
    async fn restart_restores_video_media_payload() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("restart-video.db");
        let runtime = DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime");
        let topic = "kukuri:topic:restart-video";
        let event_id = runtime
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "restored video".into(),
                reply_to: None,
                attachments: vec![
                    video_attachment_request(
                        "clip.mp4",
                        "video/mp4",
                        b"restart-video-manifest",
                        "video_manifest",
                    ),
                    video_attachment_request(
                        "clip-poster.jpg",
                        "image/jpeg",
                        b"restart-video-poster",
                        "video_poster",
                    ),
                ],
            })
            .await
            .expect("create video post");
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
            .expect("restored video post");

        let poster = restored
            .attachments
            .iter()
            .find(|attachment| attachment.role == "video_poster")
            .expect("restored poster");
        let preview = restarted
            .get_blob_media_payload(GetBlobMediaRequest {
                hash: poster.hash.clone(),
                mime: poster.mime.clone(),
            })
            .await
            .expect("video payload after restart");
        assert!(preview.is_some());
        let manifest = restored
            .attachments
            .iter()
            .find(|attachment| attachment.role == "video_manifest")
            .expect("restored video manifest");
        let playback = restarted
            .get_blob_media_payload(GetBlobMediaRequest {
                hash: manifest.hash.clone(),
                mime: manifest.mime.clone(),
            })
            .await
            .expect("video playback payload after restart");
        assert!(playback.is_some());
    }

    #[tokio::test]
    async fn restart_restores_live_session_manifest() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("restart-live.db");
        let runtime = DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime");
        let topic = "kukuri:topic:restart-live";
        let session_id = runtime
            .create_live_session(CreateLiveSessionRequest {
                topic: topic.into(),
                title: "restart live".into(),
                description: "session".into(),
            })
            .await
            .expect("create live session");
        runtime
            .join_live_session(LiveSessionCommandRequest {
                topic: topic.into(),
                session_id: session_id.clone(),
            })
            .await
            .expect("join live session");
        runtime
            .end_live_session(LiveSessionCommandRequest {
                topic: topic.into(),
                session_id: session_id.clone(),
            })
            .await
            .expect("end live session");
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
        let sessions = restarted
            .list_live_sessions(ListLiveSessionsRequest {
                topic: topic.into(),
            })
            .await
            .expect("list live sessions");
        let restored = sessions
            .iter()
            .find(|session| session.session_id == session_id)
            .expect("restored live session");
        assert_eq!(restored.status, kukuri_core::LiveSessionStatus::Ended);
        assert_eq!(restored.viewer_count, 0);
        assert!(!restored.joined_by_me);
    }

    #[tokio::test]
    async fn restart_restores_game_room_manifest() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("restart-game.db");
        let runtime = DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime");
        let topic = "kukuri:topic:restart-game";
        let room_id = runtime
            .create_game_room(CreateGameRoomRequest {
                topic: topic.into(),
                title: "restart finals".into(),
                description: "set".into(),
                participants: vec!["Alice".into(), "Bob".into()],
            })
            .await
            .expect("create game room");
        runtime
            .update_game_room(UpdateGameRoomRequest {
                topic: topic.into(),
                room_id: room_id.clone(),
                status: GameRoomStatus::InProgress,
                phase_label: Some("Round 3".into()),
                scores: vec![
                    GameScoreView {
                        participant_id: "participant-1".into(),
                        label: "Alice".into(),
                        score: 2,
                    },
                    GameScoreView {
                        participant_id: "participant-2".into(),
                        label: "Bob".into(),
                        score: 1,
                    },
                ],
            })
            .await
            .expect("update game room");
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
        let rooms = restarted
            .list_game_rooms(ListGameRoomsRequest {
                topic: topic.into(),
            })
            .await
            .expect("list game rooms");
        let restored = rooms
            .iter()
            .find(|room| room.room_id == room_id)
            .expect("restored game room");
        assert_eq!(restored.status, GameRoomStatus::InProgress);
        assert_eq!(restored.phase_label.as_deref(), Some("Round 3"));
        assert_eq!(
            restored
                .scores
                .iter()
                .find(|score| score.label == "Alice")
                .map(|score| score.score),
            Some(2)
        );
    }
}
