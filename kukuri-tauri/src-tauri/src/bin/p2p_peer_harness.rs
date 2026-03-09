use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use base64::Engine as _;
use chrono::{DateTime, Utc};
use iroh::SecretKey as IrohSecretKey;
use kukuri_lib::test_support::application::services::p2p_service::{P2PService, P2PStack};
use kukuri_lib::test_support::application::shared::nostr::EventPublisher;
use kukuri_lib::test_support::domain::entities::Event as DomainEvent;
use kukuri_lib::test_support::infrastructure::p2p::iroh_network_service::{
    IrohNetworkService, configured_custom_relay_url_strings,
};
use kukuri_lib::test_support::shared::config::{AppConfig, BootstrapSource, NetworkConfig};
use nostr_sdk::prelude::{
    Event as NostrEvent, Keys as NostrKeys, Metadata, SecretKey as NostrSecretKey,
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{info, warn};

const DEFAULT_TOPIC_ID: &str =
    "kukuri:tauri:731051a1c14a65ee3735ee4ab3b97198cae1633700f9b87fcde205e64c5a56b0";
const DEFAULT_BOOTSTRAP_PEER: &str =
    "03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8@127.0.0.1:11233";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PeerMode {
    Listener,
    Publisher,
    Echo,
}

impl PeerMode {
    fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "publisher" => Self::Publisher,
            "echo" => Self::Echo,
            _ => Self::Listener,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Listener => "listener",
            Self::Publisher => "publisher",
            Self::Echo => "echo",
        }
    }
}

#[derive(Debug, Clone)]
struct HarnessConfig {
    peer_name: String,
    mode: PeerMode,
    topic_id: String,
    bootstrap_peers: Vec<String>,
    publish_interval_ms: u64,
    publish_max: Option<u64>,
    publish_prefix: String,
    publish_content: Option<String>,
    reply_to_event_id: Option<String>,
    startup_delay_ms: u64,
    run_seconds: Option<u64>,
    summary_path: Option<PathBuf>,
    node_address_path: Option<PathBuf>,
    command_dir: Option<PathBuf>,
    iroh_secret_key_b64: Option<String>,
    nostr_secret_key_b64: Option<String>,
    publish_metadata: bool,
    publish_on_peer_join: bool,
    profile_name: Option<String>,
    profile_about: Option<String>,
}

#[derive(Debug, Default, Serialize, Clone)]
struct HarnessStats {
    published_count: u64,
    metadata_published_count: u64,
    received_count: u64,
    echoed_count: u64,
    peer_joined_events: u64,
    peer_left_events: u64,
    unique_event_ids: usize,
    recent_contents: Vec<String>,
    last_error: Option<String>,
}

#[derive(Debug, Serialize)]
struct HarnessSummary {
    peer_name: String,
    mode: String,
    topic_id: String,
    bootstrap_peers: Vec<String>,
    node_addresses: Vec<String>,
    relay_urls: Vec<String>,
    connection_hints: Vec<String>,
    preferred_address: String,
    started_at: DateTime<Utc>,
    finished_at: DateTime<Utc>,
    uptime_ms: u64,
    status_peer_count: usize,
    status_connection: String,
    stats: HarnessStats,
}

#[derive(Debug, Clone)]
struct TopicPublishRequest {
    topic_id: String,
    content: String,
    reply_to_event_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PeerCommand {
    command_id: String,
    action: String,
    topic_id: Option<String>,
    content: Option<String>,
    reply_to_event_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct PeerCommandResult {
    command_id: String,
    ok: bool,
    processed_at: DateTime<Utc>,
    topic_id: Option<String>,
    content: Option<String>,
    event_id: Option<String>,
    published_count: u64,
    error: Option<String>,
}

fn parse_env_list(raw: Option<String>) -> Vec<String> {
    raw.unwrap_or_default()
        .split(',')
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

fn parse_optional_u64(raw: Option<String>) -> Option<u64> {
    raw.and_then(|value| value.trim().parse::<u64>().ok())
}

struct SummaryContext<'a> {
    cfg: &'a HarnessConfig,
    started_at: &'a DateTime<Utc>,
    start_instant: &'a Instant,
    node_addresses: &'a [String],
    relay_urls: &'a [String],
    connection_hints: &'a [String],
    preferred_address: &'a str,
    stats: &'a HarnessStats,
    status_peer_count: usize,
    status_connection: String,
}

fn build_summary(context: SummaryContext<'_>) -> HarnessSummary {
    HarnessSummary {
        peer_name: context.cfg.peer_name.clone(),
        mode: context.cfg.mode.as_str().to_string(),
        topic_id: context.cfg.topic_id.clone(),
        bootstrap_peers: context.cfg.bootstrap_peers.clone(),
        node_addresses: context.node_addresses.to_vec(),
        relay_urls: context.relay_urls.to_vec(),
        connection_hints: context.connection_hints.to_vec(),
        preferred_address: context.preferred_address.to_string(),
        started_at: context.started_at.to_owned(),
        finished_at: Utc::now(),
        uptime_ms: context.start_instant.elapsed().as_millis() as u64,
        status_peer_count: context.status_peer_count,
        status_connection: context.status_connection,
        stats: context.stats.clone(),
    }
}

fn parse_optional_string(raw: Option<String>) -> Option<String> {
    raw.map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn parse_env_bool(raw: Option<String>, default: bool) -> bool {
    match raw {
        Some(value) => match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => default,
        },
        None => default,
    }
}

fn parse_required_string(key: &str, default_value: &str) -> String {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default_value.to_string())
}

fn build_config() -> HarnessConfig {
    let peer_name = parse_required_string("KUKURI_PEER_NAME", "peer-client");
    let bootstrap_peers = {
        let explicit = parse_env_list(std::env::var("KUKURI_PEER_BOOTSTRAP_PEERS").ok());
        if !explicit.is_empty() {
            explicit
        } else {
            let fallback = parse_env_list(std::env::var("KUKURI_BOOTSTRAP_PEERS").ok());
            if fallback.is_empty() {
                vec![DEFAULT_BOOTSTRAP_PEER.to_string()]
            } else {
                fallback
            }
        }
    };

    HarnessConfig {
        peer_name: peer_name.clone(),
        mode: PeerMode::parse(
            &std::env::var("KUKURI_PEER_MODE").unwrap_or_else(|_| "listener".to_string()),
        ),
        topic_id: parse_required_string("KUKURI_PEER_TOPIC", DEFAULT_TOPIC_ID),
        bootstrap_peers,
        publish_interval_ms: parse_optional_u64(
            std::env::var("KUKURI_PEER_PUBLISH_INTERVAL_MS").ok(),
        )
        .unwrap_or(2_000)
        .max(100),
        publish_max: parse_optional_u64(std::env::var("KUKURI_PEER_PUBLISH_MAX").ok()),
        publish_prefix: parse_required_string("KUKURI_PEER_PUBLISH_PREFIX", "multi-peer-publisher"),
        publish_content: parse_optional_string(std::env::var("KUKURI_PEER_PUBLISH_CONTENT").ok()),
        reply_to_event_id: parse_optional_string(
            std::env::var("KUKURI_PEER_REPLY_TO_EVENT_ID").ok(),
        ),
        startup_delay_ms: parse_optional_u64(std::env::var("KUKURI_PEER_STARTUP_DELAY_MS").ok())
            .unwrap_or(0),
        run_seconds: parse_optional_u64(std::env::var("KUKURI_PEER_RUN_SECONDS").ok()),
        summary_path: std::env::var("KUKURI_PEER_SUMMARY_PATH")
            .ok()
            .map(|path| path.trim().to_string())
            .filter(|path| !path.is_empty())
            .map(PathBuf::from),
        node_address_path: std::env::var("KUKURI_PEER_NODE_ADDRESS_PATH")
            .ok()
            .map(|path| path.trim().to_string())
            .filter(|path| !path.is_empty())
            .map(PathBuf::from),
        command_dir: std::env::var("KUKURI_PEER_COMMAND_DIR")
            .ok()
            .map(|path| path.trim().to_string())
            .filter(|path| !path.is_empty())
            .map(PathBuf::from),
        iroh_secret_key_b64: std::env::var("KUKURI_PEER_IROH_SECRET_KEY_B64").ok(),
        nostr_secret_key_b64: std::env::var("KUKURI_PEER_NOSTR_SECRET_KEY_B64").ok(),
        publish_metadata: parse_env_bool(std::env::var("KUKURI_PEER_PUBLISH_METADATA").ok(), false),
        publish_on_peer_join: parse_env_bool(
            std::env::var("KUKURI_PEER_PUBLISH_ON_PEER_JOIN").ok(),
            false,
        ),
        profile_name: parse_optional_string(std::env::var("KUKURI_PEER_PROFILE_NAME").ok()),
        profile_about: parse_optional_string(std::env::var("KUKURI_PEER_PROFILE_ABOUT").ok()),
    }
}

fn decode_base64_32bytes(encoded: &str) -> anyhow::Result<[u8; 32]> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .map_err(|err| anyhow::anyhow!("base64 decode error: {err}"))?;
    if bytes.len() != 32 {
        return Err(anyhow::anyhow!(
            "expected 32-byte secret key, got {} bytes",
            bytes.len()
        ));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn build_iroh_secret(encoded: Option<&str>) -> anyhow::Result<IrohSecretKey> {
    if let Some(value) = encoded {
        let decoded = decode_base64_32bytes(value)?;
        return Ok(IrohSecretKey::from_bytes(&decoded));
    }
    let random_bytes: [u8; 32] = rand::random();
    Ok(IrohSecretKey::from_bytes(&random_bytes))
}

fn build_nostr_keys(encoded: Option<&str>) -> anyhow::Result<NostrKeys> {
    if let Some(value) = encoded {
        let decoded = decode_base64_32bytes(value)?;
        let secret = NostrSecretKey::from_hex(&hex::encode(decoded))
            .map_err(|err| anyhow::anyhow!("invalid nostr secret key: {err}"))?;
        return Ok(NostrKeys::new(secret));
    }
    Ok(NostrKeys::generate())
}

fn convert_nostr_event(event: &NostrEvent) -> anyhow::Result<DomainEvent> {
    let created_at = DateTime::<Utc>::from_timestamp(event.created_at.as_secs() as i64, 0)
        .ok_or_else(|| anyhow::anyhow!("invalid nostr event timestamp"))?;
    Ok(DomainEvent {
        id: event.id.to_string(),
        pubkey: event.pubkey.to_string(),
        created_at,
        kind: event.kind.as_u16() as u32,
        tags: event.tags.iter().map(|tag| tag.clone().to_vec()).collect(),
        content: event.content.clone(),
        sig: event.sig.to_string(),
    })
}

fn build_network_config(cfg: &HarnessConfig) -> NetworkConfig {
    let mut network = AppConfig::from_env().network;
    network.bootstrap_peers = cfg.bootstrap_peers.clone();
    network.bootstrap_source = BootstrapSource::Env;
    network.enable_dht = true;
    network.enable_dns = false;
    network.enable_local = true;
    network
}

async fn publish_topic_event(
    publisher: &EventPublisher,
    stack: &kukuri_lib::test_support::application::services::p2p_service::P2PStack,
    cfg: &HarnessConfig,
    request: &TopicPublishRequest,
    stats: &mut HarnessStats,
) -> anyhow::Result<String> {
    let reply_to = match request.reply_to_event_id.as_deref() {
        Some(raw) => Some(
            nostr_sdk::prelude::EventId::from_hex(raw)
                .map_err(|err| anyhow::anyhow!("invalid reply_to event id: {err}"))?,
        ),
        None => None,
    };
    let nostr_event =
        publisher.create_topic_post(&request.topic_id, &request.content, reply_to, None, None)?;
    let domain_event = convert_nostr_event(&nostr_event)?;
    stack
        .gossip_service
        .broadcast(&request.topic_id, &domain_event)
        .await?;
    stats.published_count += 1;
    info!(
        peer = %cfg.peer_name,
        topic = %request.topic_id,
        event_id = %domain_event.id,
        published = stats.published_count,
        "Peer harness published topic event"
    );
    Ok(domain_event.id)
}

async fn publish_profile_metadata(
    publisher: &EventPublisher,
    stack: &kukuri_lib::test_support::application::services::p2p_service::P2PStack,
    cfg: &HarnessConfig,
    stats: &mut HarnessStats,
) -> anyhow::Result<()> {
    let mut metadata = Metadata::new().name(
        cfg.profile_name
            .clone()
            .unwrap_or_else(|| cfg.peer_name.clone()),
    );
    if let Some(about) = cfg.profile_about.as_deref() {
        metadata = metadata.about(about);
    }
    let metadata_event = publisher.create_metadata(metadata)?;
    let domain_event = convert_nostr_event(&metadata_event)?;
    stack
        .gossip_service
        .broadcast(&cfg.topic_id, &domain_event)
        .await?;
    stats.metadata_published_count += 1;
    info!(
        peer = %cfg.peer_name,
        topic = %cfg.topic_id,
        event_id = %domain_event.id,
        metadata_published = stats.metadata_published_count,
        "Peer harness published metadata event"
    );
    Ok(())
}

fn push_recent_content(stats: &mut HarnessStats, content: &str) {
    if stats.recent_contents.len() >= 20 {
        stats.recent_contents.remove(0);
    }
    stats.recent_contents.push(content.to_string());
}

fn build_default_publish_request(cfg: &HarnessConfig) -> TopicPublishRequest {
    TopicPublishRequest {
        topic_id: cfg.topic_id.clone(),
        content: cfg.publish_content.clone().unwrap_or_else(|| {
            format!(
                "{} [{}] {}",
                cfg.publish_prefix,
                cfg.peer_name,
                Utc::now().to_rfc3339()
            )
        }),
        reply_to_event_id: cfg.reply_to_event_id.clone(),
    }
}

#[derive(Debug, Serialize)]
struct NodeAddressSnapshot {
    peer_name: String,
    node_addresses: Vec<String>,
    relay_urls: Vec<String>,
    connection_hints: Vec<String>,
    preferred_address: String,
    written_at: DateTime<Utc>,
}

fn dedupe_in_order(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for value in values {
        if seen.insert(value.clone()) {
            deduped.push(value);
        }
    }
    deduped
}

fn parse_node_id_from_address(address: &str) -> Option<String> {
    let trimmed = address.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some((node_id, _)) = trimmed.split_once('|') {
        let node_id = node_id.trim();
        if !node_id.is_empty() {
            return Some(node_id.to_string());
        }
        return None;
    }
    if let Some((node_id, _)) = trimmed.split_once('@') {
        let node_id = node_id.trim();
        if !node_id.is_empty() {
            return Some(node_id.to_string());
        }
        return None;
    }
    Some(trimmed.to_string())
}

fn parse_endpoint_from_address(address: &str) -> Option<String> {
    for segment in address.split('|').skip(1) {
        let (key, value) = segment.split_once('=')?;
        if key.trim().eq_ignore_ascii_case("addr") {
            let endpoint = value.trim();
            if !endpoint.is_empty() {
                return Some(endpoint.to_string());
            }
        }
    }
    let (_, endpoint) = address.split_once('@')?;
    let endpoint = endpoint.trim();
    if endpoint.is_empty() {
        return None;
    }
    Some(endpoint.to_string())
}

fn build_connection_hints(node_addresses: &[String], relay_urls: &[String]) -> Vec<String> {
    let node_id = node_addresses
        .iter()
        .find_map(|address| parse_node_id_from_address(address));
    let mut hints = node_addresses.to_vec();

    if let Some(node_id) = node_id {
        let endpoints: Vec<_> = node_addresses
            .iter()
            .filter_map(|address| parse_endpoint_from_address(address))
            .collect();

        if !endpoints.is_empty() {
            for endpoint in endpoints {
                for relay_url in relay_urls {
                    hints.push(format!("{node_id}|relay={relay_url}|addr={endpoint}"));
                }
            }
        }

        let has_relay_only_hint = node_addresses
            .iter()
            .any(|address| address.contains("|relay=") && !address.contains("|addr="));
        if !has_relay_only_hint {
            for relay_url in relay_urls {
                hints.push(format!("{node_id}|relay={relay_url}"));
            }
        }
    }
    dedupe_in_order(hints)
}

async fn resolve_relay_urls(stack: &P2PStack) -> Vec<String> {
    let Some(network_service) = stack
        .network_service
        .as_any()
        .downcast_ref::<IrohNetworkService>()
    else {
        return Vec::new();
    };

    if tokio::time::timeout(Duration::from_secs(10), network_service.endpoint().online())
        .await
        .is_err()
    {
        warn!("Timed out waiting for endpoint online state; relay URL snapshot may be empty");
    }
    let endpoint_addr = network_service.endpoint().addr();
    let relay_urls = endpoint_addr
        .relay_urls()
        .map(|relay_url| relay_url.to_string())
        .chain(
            configured_custom_relay_url_strings()
                .unwrap_or_default()
                .into_iter(),
        )
        .collect::<Vec<_>>();
    dedupe_in_order(relay_urls)
}

fn endpoint_host(address: &str) -> Option<String> {
    let (_, endpoint) = address.split_once('@')?;
    let endpoint = endpoint.trim();
    if endpoint.is_empty() {
        return None;
    }
    if endpoint.starts_with('[') {
        let close = endpoint.find(']')?;
        return Some(endpoint[1..close].to_string());
    }
    endpoint
        .rsplit_once(':')
        .map(|(host, _)| host.trim().to_string())
}

fn is_loopback_host(host: &str) -> bool {
    let normalized = host.trim().to_ascii_lowercase();
    normalized == "127.0.0.1" || normalized == "localhost" || normalized == "::1"
}

fn pick_preferred_address(addresses: &[String]) -> Option<String> {
    if let Some(non_loopback) = addresses.iter().find(|entry| {
        endpoint_host(entry)
            .map(|host| !is_loopback_host(&host))
            .unwrap_or(false)
    }) {
        return Some(non_loopback.clone());
    }
    addresses.first().cloned()
}

async fn write_node_address_snapshot(
    path: &Path,
    snapshot: &NodeAddressSnapshot,
) -> anyhow::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(snapshot)?)?;
    Ok(())
}

async fn write_summary(path: &Path, summary: &HarnessSummary) -> anyhow::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(summary)?)?;
    Ok(())
}

fn peer_command_result_path(command_path: &Path) -> PathBuf {
    let file_name = command_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("command.json");
    let stem = file_name.strip_suffix(".json").unwrap_or(file_name);
    command_path.with_file_name(format!("{stem}.result.json"))
}

fn load_pending_command_paths(command_dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    if !command_dir.exists() {
        fs::create_dir_all(command_dir)?;
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(command_dir)? {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !file_name.ends_with(".json") || file_name.ends_with(".result.json") {
            continue;
        }
        entries.push(path);
    }
    entries.sort();
    Ok(entries)
}

fn build_publish_request_from_command(
    cfg: &HarnessConfig,
    command: &PeerCommand,
) -> anyhow::Result<TopicPublishRequest> {
    let topic_id = command
        .topic_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(cfg.topic_id.as_str())
        .to_string();
    let content = command
        .content
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("command content is required"))?
        .to_string();
    let reply_to_event_id = command
        .reply_to_event_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    Ok(TopicPublishRequest {
        topic_id,
        content,
        reply_to_event_id,
    })
}

async fn process_peer_commands(
    command_dir: &Path,
    publisher: &EventPublisher,
    stack: &kukuri_lib::test_support::application::services::p2p_service::P2PStack,
    cfg: &HarnessConfig,
    stats: &mut HarnessStats,
) -> anyhow::Result<()> {
    for command_path in load_pending_command_paths(command_dir)? {
        let command_result_path = peer_command_result_path(&command_path);
        let raw = fs::read_to_string(&command_path)?;
        let parsed = serde_json::from_str::<PeerCommand>(&raw);
        let result = match parsed {
            Ok(command) => {
                let execution: anyhow::Result<PeerCommandResult> = match command.action.trim() {
                    "publish_topic_event" => {
                        let request = build_publish_request_from_command(cfg, &command)?;
                        let event_id =
                            publish_topic_event(publisher, stack, cfg, &request, stats).await?;
                        Ok(PeerCommandResult {
                            command_id: command.command_id,
                            ok: true,
                            processed_at: Utc::now(),
                            topic_id: Some(request.topic_id),
                            content: Some(request.content),
                            event_id: Some(event_id),
                            published_count: stats.published_count,
                            error: None,
                        })
                    }
                    other => Ok(PeerCommandResult {
                        command_id: command.command_id,
                        ok: false,
                        processed_at: Utc::now(),
                        topic_id: command.topic_id,
                        content: command.content,
                        event_id: None,
                        published_count: stats.published_count,
                        error: Some(format!("unsupported peer command action: {other}")),
                    }),
                };
                execution?
            }
            Err(err) => PeerCommandResult {
                command_id: command_path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or("invalid-command")
                    .to_string(),
                ok: false,
                processed_at: Utc::now(),
                topic_id: None,
                content: None,
                event_id: None,
                published_count: stats.published_count,
                error: Some(format!("failed to parse peer command: {err}")),
            },
        };

        fs::write(&command_result_path, serde_json::to_vec_pretty(&result)?)?;
        fs::remove_file(&command_path)?;
        if let Some(error) = &result.error {
            stats.last_error = Some(error.clone());
            warn!(
                peer = %cfg.peer_name,
                command_id = %result.command_id,
                error = %error,
                "Peer harness command failed"
            );
        } else {
            info!(
                peer = %cfg.peer_name,
                command_id = %result.command_id,
                published = stats.published_count,
                "Peer harness command completed"
            );
        }
    }
    Ok(())
}

fn init_logging() {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging();
    let cfg = build_config();
    let started_at = Utc::now();
    let start_instant = Instant::now();

    info!(
        peer = %cfg.peer_name,
        mode = %cfg.mode.as_str(),
        topic = %cfg.topic_id,
        publish_metadata = cfg.publish_metadata,
        publish_on_peer_join = cfg.publish_on_peer_join,
        bootstrap_peers = %cfg.bootstrap_peers.join(","),
        "Starting p2p peer harness"
    );

    if cfg.startup_delay_ms > 0 {
        tokio::time::sleep(Duration::from_millis(cfg.startup_delay_ms)).await;
    }

    let iroh_secret = build_iroh_secret(cfg.iroh_secret_key_b64.as_deref())?;
    let nostr_keys = build_nostr_keys(cfg.nostr_secret_key_b64.as_deref())?;
    let own_pubkey = nostr_keys.public_key().to_string();
    let mut publisher = EventPublisher::new();
    publisher.set_keys(nostr_keys);

    let network_config = build_network_config(&cfg);
    let (event_tx, mut event_rx) = broadcast::channel(1024);
    let stack = P2PService::builder(iroh_secret, network_config)
        .with_event_sender(event_tx)
        .build()
        .await?;

    stack.network_service.connect().await?;
    stack
        .p2p_service
        .join_topic(&cfg.topic_id, cfg.bootstrap_peers.clone())
        .await?;
    let mut subscription = stack.gossip_service.subscribe(&cfg.topic_id).await?;

    let mut stats = HarnessStats::default();
    let mut seen_events = HashSet::new();
    let mut publish_enabled = cfg.mode == PeerMode::Publisher && !cfg.publish_on_peer_join;
    let mut publish_tick = tokio::time::interval(Duration::from_millis(cfg.publish_interval_ms));
    let mut summary_tick = tokio::time::interval(Duration::from_secs(1));
    let mut command_tick = tokio::time::interval(Duration::from_millis(250));
    publish_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    summary_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    command_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let node_addresses = match stack.p2p_service.get_node_addresses().await {
        Ok(addresses) => addresses,
        Err(err) => {
            warn!(
                peer = %cfg.peer_name,
                error = %err,
                "Failed to resolve node addresses"
            );
            Vec::new()
        }
    };
    let relay_urls = resolve_relay_urls(&stack).await;
    let connection_hints = build_connection_hints(&node_addresses, &relay_urls);
    let preferred_address = pick_preferred_address(&node_addresses)
        .or_else(|| connection_hints.first().cloned())
        .unwrap_or_default();
    if let Some(path) = &cfg.node_address_path {
        let snapshot = NodeAddressSnapshot {
            peer_name: cfg.peer_name.clone(),
            node_addresses: node_addresses.clone(),
            relay_urls: relay_urls.clone(),
            connection_hints: connection_hints.clone(),
            preferred_address: preferred_address.clone(),
            written_at: Utc::now(),
        };
        if let Err(err) = write_node_address_snapshot(path, &snapshot).await {
            warn!(
                peer = %cfg.peer_name,
                path = %path.display(),
                error = %err,
                "Failed to write node address snapshot"
            );
        }
    }

    if cfg.publish_metadata
        && let Err(err) = publish_profile_metadata(&publisher, &stack, &cfg, &mut stats).await
    {
        stats.last_error = Some(err.to_string());
        warn!(peer = %cfg.peer_name, error = %err, "Initial metadata publish failed");
    }

    if cfg.mode == PeerMode::Publisher
        && publish_enabled
        && let Err(err) = publish_topic_event(
            &publisher,
            &stack,
            &cfg,
            &build_default_publish_request(&cfg),
            &mut stats,
        )
        .await
    {
        stats.last_error = Some(err.to_string());
        warn!(peer = %cfg.peer_name, error = %err, "Initial publish failed");
    }

    let stop_reason = loop {
        if let Some(seconds) = cfg.run_seconds
            && start_instant.elapsed() >= Duration::from_secs(seconds)
        {
            break "timeout";
        }
        if let Some(max) = cfg.publish_max
            && stats.published_count >= max
        {
            break "publish_max_reached";
        }

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                break "ctrl_c";
            }
            _ = publish_tick.tick(), if cfg.mode == PeerMode::Publisher && publish_enabled => {
                if let Err(err) = publish_topic_event(
                    &publisher,
                    &stack,
                    &cfg,
                    &build_default_publish_request(&cfg),
                    &mut stats,
                ).await {
                    stats.last_error = Some(err.to_string());
                    warn!(peer = %cfg.peer_name, error = %err, "Periodic publish failed");
                }
            }
            _ = command_tick.tick(), if cfg.command_dir.is_some() => {
                if let Some(command_dir) = &cfg.command_dir
                    && let Err(err) = process_peer_commands(
                        command_dir,
                        &publisher,
                        &stack,
                        &cfg,
                        &mut stats,
                    ).await
                {
                    stats.last_error = Some(err.to_string());
                    warn!(peer = %cfg.peer_name, error = %err, "Peer command processing failed");
                }
            }
            _ = summary_tick.tick(), if cfg.summary_path.is_some() => {
                match stack.p2p_service.get_status().await {
                    Ok(status) => {
                        let summary = build_summary(SummaryContext {
                            cfg: &cfg,
                            started_at: &started_at,
                            start_instant: &start_instant,
                            node_addresses: &node_addresses,
                            relay_urls: &relay_urls,
                            connection_hints: &connection_hints,
                            preferred_address: &preferred_address,
                            stats: &stats,
                            status_peer_count: status.peers.len(),
                            status_connection: format!("{:?}", status.connection_status)
                                .to_ascii_lowercase(),
                        });
                        if let Some(path) = &cfg.summary_path
                            && let Err(err) = write_summary(path, &summary).await
                        {
                            warn!(
                                peer = %cfg.peer_name,
                                path = %path.display(),
                                error = %err,
                                "Failed to write runtime peer summary"
                            );
                        }
                    }
                    Err(err) => {
                        stats.last_error = Some(err.to_string());
                        warn!(
                            peer = %cfg.peer_name,
                            error = %err,
                            "Failed to capture runtime peer status"
                        );
                    }
                }
            }
            maybe_event = subscription.recv() => {
                match maybe_event {
                    Some(event) => {
                        stats.received_count += 1;
                        push_recent_content(&mut stats, &event.content);
                        let is_new = seen_events.insert(event.id.clone());
                        if cfg.mode == PeerMode::Echo && is_new && event.pubkey != own_pubkey {
                            if let Err(err) = stack.gossip_service.broadcast(&cfg.topic_id, &event).await {
                                stats.last_error = Some(err.to_string());
                                warn!(peer = %cfg.peer_name, error = %err, "Echo broadcast failed");
                            } else {
                                stats.echoed_count += 1;
                            }
                        }
                    }
                    None => {
                        break "subscription_closed";
                    }
                }
            }
            evt = event_rx.recv() => {
                match evt {
                    Ok(kukuri_lib::test_support::domain::p2p::P2PEvent::PeerJoined { .. }) => {
                        stats.peer_joined_events += 1;
                        if cfg.mode == PeerMode::Publisher && cfg.publish_on_peer_join && !publish_enabled {
                            publish_enabled = true;
                            if let Err(err) = publish_topic_event(
                                &publisher,
                                &stack,
                                &cfg,
                                &build_default_publish_request(&cfg),
                                &mut stats,
                            ).await {
                                stats.last_error = Some(err.to_string());
                                warn!(peer = %cfg.peer_name, error = %err, "Peer-joined publish failed");
                            }
                        }
                    }
                    Ok(kukuri_lib::test_support::domain::p2p::P2PEvent::PeerLeft { .. }) => {
                        stats.peer_left_events += 1;
                    }
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {}
        }
    };

    stats.unique_event_ids = seen_events.len();

    let status = stack.p2p_service.get_status().await?;
    let summary = build_summary(SummaryContext {
        cfg: &cfg,
        started_at: &started_at,
        start_instant: &start_instant,
        node_addresses: &node_addresses,
        relay_urls: &relay_urls,
        connection_hints: &connection_hints,
        preferred_address: &preferred_address,
        stats: &stats,
        status_peer_count: status.peers.len(),
        status_connection: format!("{:?}", status.connection_status).to_ascii_lowercase(),
    });

    info!(
        peer = %cfg.peer_name,
        mode = %cfg.mode.as_str(),
        stop_reason = %stop_reason,
        published = summary.stats.published_count,
        received = summary.stats.received_count,
        peers = summary.status_peer_count,
        "Peer harness finished"
    );

    if let Some(path) = &cfg.summary_path {
        if let Err(err) = write_summary(path, &summary).await {
            warn!(
                peer = %cfg.peer_name,
                path = %path.display(),
                error = %err,
                "Failed to write peer summary"
            );
        }
    } else {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    }

    Ok(())
}
