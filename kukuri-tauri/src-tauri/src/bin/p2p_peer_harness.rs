use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use base64::Engine as _;
use chrono::{DateTime, Utc};
use iroh::SecretKey as IrohSecretKey;
use kukuri_lib::test_support::application::services::p2p_service::P2PService;
use kukuri_lib::test_support::application::shared::nostr::EventPublisher;
use kukuri_lib::test_support::domain::entities::Event as DomainEvent;
use kukuri_lib::test_support::shared::config::{AppConfig, BootstrapSource, NetworkConfig};
use nostr_sdk::prelude::{Event as NostrEvent, Keys as NostrKeys, SecretKey as NostrSecretKey};
use serde::Serialize;
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
    startup_delay_ms: u64,
    run_seconds: Option<u64>,
    summary_path: Option<PathBuf>,
    iroh_secret_key_b64: Option<String>,
    nostr_secret_key_b64: Option<String>,
}

#[derive(Debug, Default, Serialize)]
struct HarnessStats {
    published_count: u64,
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
    started_at: DateTime<Utc>,
    finished_at: DateTime<Utc>,
    uptime_ms: u64,
    status_peer_count: usize,
    status_connection: String,
    stats: HarnessStats,
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

fn parse_required_string(key: &str, default_value: &str) -> String {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default_value.to_string())
}

fn build_config() -> HarnessConfig {
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
        peer_name: parse_required_string("KUKURI_PEER_NAME", "peer-client"),
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
        startup_delay_ms: parse_optional_u64(std::env::var("KUKURI_PEER_STARTUP_DELAY_MS").ok())
            .unwrap_or(0),
        run_seconds: parse_optional_u64(std::env::var("KUKURI_PEER_RUN_SECONDS").ok()),
        summary_path: std::env::var("KUKURI_PEER_SUMMARY_PATH")
            .ok()
            .map(|path| path.trim().to_string())
            .filter(|path| !path.is_empty())
            .map(PathBuf::from),
        iroh_secret_key_b64: std::env::var("KUKURI_PEER_IROH_SECRET_KEY_B64").ok(),
        nostr_secret_key_b64: std::env::var("KUKURI_PEER_NOSTR_SECRET_KEY_B64").ok(),
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
    stats: &mut HarnessStats,
) -> anyhow::Result<()> {
    let content = format!(
        "{} [{}] {}",
        cfg.publish_prefix,
        cfg.peer_name,
        Utc::now().to_rfc3339()
    );
    let nostr_event = publisher.create_topic_post(&cfg.topic_id, &content, None, None, None)?;
    let domain_event = convert_nostr_event(&nostr_event)?;
    stack
        .gossip_service
        .broadcast(&cfg.topic_id, &domain_event)
        .await?;
    stats.published_count += 1;
    info!(
        peer = %cfg.peer_name,
        topic = %cfg.topic_id,
        event_id = %domain_event.id,
        published = stats.published_count,
        "Peer harness published topic event"
    );
    Ok(())
}

fn push_recent_content(stats: &mut HarnessStats, content: &str) {
    if stats.recent_contents.len() >= 20 {
        stats.recent_contents.remove(0);
    }
    stats.recent_contents.push(content.to_string());
}

async fn write_summary(path: &PathBuf, summary: &HarnessSummary) -> anyhow::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(summary)?)?;
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
    let mut publish_tick = tokio::time::interval(Duration::from_millis(cfg.publish_interval_ms));
    publish_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    if cfg.mode == PeerMode::Publisher
        && let Err(err) = publish_topic_event(&publisher, &stack, &cfg, &mut stats).await
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
            _ = publish_tick.tick(), if cfg.mode == PeerMode::Publisher => {
                if let Err(err) = publish_topic_event(&publisher, &stack, &cfg, &mut stats).await {
                    stats.last_error = Some(err.to_string());
                    warn!(peer = %cfg.peer_name, error = %err, "Periodic publish failed");
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
    let summary = HarnessSummary {
        peer_name: cfg.peer_name.clone(),
        mode: cfg.mode.as_str().to_string(),
        topic_id: cfg.topic_id.clone(),
        bootstrap_peers: cfg.bootstrap_peers.clone(),
        started_at,
        finished_at: Utc::now(),
        uptime_ms: start_instant.elapsed().as_millis() as u64,
        status_peer_count: status.peers.len(),
        status_connection: format!("{:?}", status.connection_status).to_ascii_lowercase(),
        stats,
    };

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
