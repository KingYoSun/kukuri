use anyhow::{anyhow, Result};
use base64::prelude::*;
use clap::{Args, Parser, Subcommand};
use iroh::{
    discovery::{
        dns::DnsDiscovery,
        mdns::MdnsDiscovery,
        pkarr::{dht::DhtDiscovery, PkarrPublisher},
        static_provider::StaticProvider,
    },
    endpoint::Builder as EndpointBuilder,
    protocol::Router,
    Endpoint, EndpointAddr, EndpointId, SecretKey,
};
use iroh_gossip::net::Gossip;
use serde_json::Value;
use std::fs;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

mod bootstrap_cache;
mod e2e_invite;
mod e2e_seed;

use bootstrap_cache::{resolve_export_path, write_cache, CliBootstrapCache};

pub(crate) const TOPIC_NAMESPACE: &str = "kukuri:";
const DEFAULT_PUBLIC_TOPIC_ID: &str = "kukuri:global";

#[derive(Parser)]
#[command(name = "cn", version, about = "Kukuri community node CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    UserApi,
    AdminApi,
    Relay,
    Bootstrap,
    Index,
    Moderation,
    Trust,
    AccessControl {
        #[command(subcommand)]
        command: AccessControlCommand,
    },
    E2e {
        #[command(subcommand)]
        command: E2eCommand,
    },
    Migrate,
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    Admin {
        #[command(subcommand)]
        command: AdminCommand,
    },
    NodeKey {
        #[command(subcommand)]
        command: NodeKeyCommand,
    },
    P2p {
        #[command(subcommand)]
        command: P2pCommand,
    },
}

#[derive(Subcommand)]
enum ConfigCommand {
    Seed,
}

#[derive(Subcommand)]
enum E2eCommand {
    Seed,
    Cleanup,
    Invite(e2e_invite::E2eInviteArgs),
}

#[derive(Subcommand)]
enum AdminCommand {
    Bootstrap {
        #[arg(long)]
        username: String,
        #[arg(long)]
        password: String,
    },
    ResetPassword {
        #[arg(long)]
        username: String,
        #[arg(long)]
        password: String,
    },
}

#[derive(Subcommand)]
enum NodeKeyCommand {
    Generate {
        #[arg(long)]
        path: Option<String>,
    },
    Rotate {
        #[arg(long)]
        path: Option<String>,
    },
    Show {
        #[arg(long)]
        path: Option<String>,
    },
}

#[derive(Subcommand)]
enum AccessControlCommand {
    Rotate(AccessControlRotateArgs),
    Revoke(AccessControlRevokeArgs),
}

#[derive(Args, Clone)]
struct AccessControlRotateArgs {
    /// Topic name or topic id
    #[arg(long)]
    topic: String,

    /// Scope to rotate (friend/invite/friend_plus)
    #[arg(long, default_value = "invite")]
    scope: String,

    /// Pretty-print JSON output
    #[arg(long, default_value_t = false)]
    pretty: bool,
}

#[derive(Args, Clone)]
struct AccessControlRevokeArgs {
    /// Topic name or topic id
    #[arg(long)]
    topic: String,

    /// Scope to revoke (friend/invite/friend_plus)
    #[arg(long, default_value = "invite")]
    scope: String,

    /// Member pubkey to revoke
    #[arg(long)]
    pubkey: String,

    /// Optional revoke reason
    #[arg(long)]
    reason: Option<String>,

    /// Pretty-print JSON output
    #[arg(long, default_value_t = false)]
    pretty: bool,
}

#[derive(Args, Clone)]
struct P2pArgs {
    /// Bind address for the node (used for P2P only)
    #[arg(short, long, default_value = "0.0.0.0:11223", env = "BIND_ADDRESS")]
    bind: String,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info", env = "LOG_LEVEL")]
    log_level: String,

    /// Enable JSON logging
    #[arg(long, env = "JSON_LOGS")]
    json_logs: bool,

    /// Optional base64-encoded 32-byte secret key for deterministic node identity
    #[arg(long, env = "KUKURI_SECRET_KEY")]
    secret_key: Option<String>,
}

#[derive(Subcommand)]
enum P2pCommand {
    /// Print the node ID derived from the configured secret key
    NodeId {
        #[command(flatten)]
        args: P2pArgs,
    },
    /// Run as DHT bootstrap node
    Bootstrap {
        #[command(flatten)]
        args: P2pArgs,
        /// Additional bootstrap peers (format: node_id@host:port)
        #[arg(long)]
        peers: Vec<String>,
        /// Optional export path for writing discovered bootstrap list
        #[arg(long)]
        export_path: Option<String>,
    },
    /// Run as relay node
    Relay {
        #[command(flatten)]
        args: P2pArgs,
        /// Topics to relay (comma-separated)
        #[arg(long, default_value = DEFAULT_PUBLIC_TOPIC_ID, env = "RELAY_TOPICS")]
        topics: String,
    },
    /// Attempt to connect to a peer and exit (for connectivity debugging)
    Connect {
        #[command(flatten)]
        args: P2pArgs,
        /// Peer identifier (node_id or node_id@host:port)
        #[arg(long)]
        peer: String,
        /// Disable DHT discovery (enabled by default)
        #[arg(long, default_value_t = false)]
        no_dht: bool,
        /// Enable mDNS (local network) discovery
        #[arg(long, default_value_t = false)]
        mdns: bool,
        /// Connection timeout in seconds
        #[arg(long, default_value_t = 15)]
        timeout: u64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::UserApi => {
            let config = cn_user_api::load_config()?;
            cn_user_api::run(config).await?;
        }
        Commands::AdminApi => {
            let config = cn_admin_api::load_config()?;
            cn_admin_api::run(config).await?;
        }
        Commands::Relay => {
            let config = cn_relay::load_config()?;
            cn_relay::run(config).await?;
        }
        Commands::Bootstrap => {
            let config = cn_bootstrap::load_config()?;
            cn_bootstrap::run(config).await?;
        }
        Commands::Index => {
            let config = cn_index::load_config()?;
            cn_index::run(config).await?;
        }
        Commands::Moderation => {
            let config = cn_moderation::load_config()?;
            cn_moderation::run(config).await?;
        }
        Commands::Trust => {
            let config = cn_trust::load_config()?;
            cn_trust::run(config).await?;
        }
        Commands::AccessControl { command } => {
            cn_core::logging::init("cn-cli");
            let database_url = cn_core::config::required_env("DATABASE_URL")?;
            let pool = cn_core::db::connect(&database_url).await?;
            let node_key_path =
                cn_core::node_key::key_path_from_env("NODE_KEY_PATH", "data/node_key.json")?;
            let node_keys = cn_core::node_key::load_or_generate(&node_key_path)?;

            match command {
                AccessControlCommand::Rotate(args) => {
                    let topic_id = generate_topic_id(&args.topic)
                        .ok_or_else(|| anyhow!("topic is empty"))?;
                    let scope = cn_core::access_control::normalize_scope(&args.scope)?;
                    let summary = cn_core::access_control::rotate_epoch(
                        &pool,
                        &node_keys,
                        &topic_id,
                        &scope,
                    )
                    .await?;
                    let output = serde_json::json!({
                        "topic_id": summary.topic_id,
                        "scope": summary.scope,
                        "previous_epoch": summary.previous_epoch,
                        "new_epoch": summary.new_epoch,
                        "recipients": summary.recipients
                    });
                    let rendered = if args.pretty {
                        serde_json::to_string_pretty(&output)?
                    } else {
                        serde_json::to_string(&output)?
                    };
                    println!("{rendered}");
                }
                AccessControlCommand::Revoke(args) => {
                    let topic_id = generate_topic_id(&args.topic)
                        .ok_or_else(|| anyhow!("topic is empty"))?;
                    let scope = cn_core::access_control::normalize_scope(&args.scope)?;
                    let pubkey = cn_core::access_control::normalize_pubkey(&args.pubkey)?;
                    let summary = cn_core::access_control::revoke_member_and_rotate(
                        &pool,
                        &node_keys,
                        &topic_id,
                        &scope,
                        &pubkey,
                        args.reason.as_deref(),
                    )
                    .await?;
                    let output = serde_json::json!({
                        "topic_id": summary.topic_id,
                        "scope": summary.scope,
                        "revoked_pubkey": summary.revoked_pubkey,
                        "previous_epoch": summary.rotation.previous_epoch,
                        "new_epoch": summary.rotation.new_epoch,
                        "recipients": summary.rotation.recipients
                    });
                    let rendered = if args.pretty {
                        serde_json::to_string_pretty(&output)?
                    } else {
                        serde_json::to_string(&output)?
                    };
                    println!("{rendered}");
                }
            }
        }
        Commands::E2e { command } => {
            match command {
                E2eCommand::Seed => {
                    cn_core::logging::init("cn-cli");
                    let summary = e2e_seed::seed().await?;
                    let summary_json = serde_json::to_string(&summary)?;
                    println!("E2E_SEED_JSON={summary_json}");
                }
                E2eCommand::Cleanup => {
                    cn_core::logging::init("cn-cli");
                    e2e_seed::cleanup().await?;
                }
                E2eCommand::Invite(args) => {
                    e2e_invite::issue_invite(args)?;
                }
            }
        }
        Commands::Migrate => {
            cn_core::logging::init("cn-cli");
            let database_url = cn_core::config::required_env("DATABASE_URL")?;
            let pool = cn_core::db::connect(&database_url).await?;
            cn_core::migrations::run(&pool).await?;
            tracing::info!("migrations applied");
        }
        Commands::Config { command } => {
            cn_core::logging::init("cn-cli");
            let database_url = cn_core::config::required_env("DATABASE_URL")?;
            let pool = cn_core::db::connect(&database_url).await?;
            match command {
                ConfigCommand::Seed => {
                    let seeded = cn_core::admin::seed_service_configs(&pool).await?;
                    if seeded.is_empty() {
                        tracing::info!("no new service configs were inserted");
                    } else {
                        tracing::info!(services = ?seeded, "service configs seeded");
                    }
                }
            }
        }
        Commands::Admin { command } => {
            cn_core::logging::init("cn-cli");
            let database_url = cn_core::config::required_env("DATABASE_URL")?;
            let pool = cn_core::db::connect(&database_url).await?;
            match command {
                AdminCommand::Bootstrap { username, password } => {
                    let created = cn_core::admin::bootstrap_admin(&pool, &username, &password)
                        .await?;
                    if created {
                        tracing::info!("admin user created");
                    } else {
                        tracing::info!("admin user already exists");
                    }
                }
                AdminCommand::ResetPassword { username, password } => {
                    cn_core::admin::reset_admin_password(&pool, &username, &password).await?;
                    tracing::info!("admin password reset");
                }
            }
        }
        Commands::NodeKey { command } => {
            handle_node_key(command).await?;
        }
        Commands::P2p { command } => {
            handle_p2p(command).await?;
        }
    }

    Ok(())
}

async fn handle_node_key(command: NodeKeyCommand) -> Result<()> {
    cn_core::logging::init("cn-cli");
    let path = resolve_node_key_path(match &command {
        NodeKeyCommand::Generate { path }
        | NodeKeyCommand::Rotate { path }
        | NodeKeyCommand::Show { path } => path.clone(),
    })?;

    match command {
        NodeKeyCommand::Generate { .. } => {
            if path.exists() {
                return Err(anyhow!("node key already exists: {}", path.display()));
            }
            let keys = cn_core::node_key::generate_keys(&path)?;
            audit_node_key("node_key.generate", &keys).await?;
            println!("{}", cn_core::node_key::public_key_hex(&keys));
        }
        NodeKeyCommand::Rotate { .. } => {
            let keys = cn_core::node_key::rotate_keys(&path)?;
            audit_node_key("node_key.rotate", &keys).await?;
            println!("{}", cn_core::node_key::public_key_hex(&keys));
        }
        NodeKeyCommand::Show { .. } => {
            let keys = cn_core::node_key::read_keys(&path)?;
            println!("{}", cn_core::node_key::public_key_hex(&keys));
        }
    }

    Ok(())
}

async fn audit_node_key(action: &str, keys: &nostr_sdk::Keys) -> Result<()> {
    let database_url = cn_core::config::required_env("DATABASE_URL")?;
    let pool = cn_core::db::connect(&database_url).await?;
    let diff = serde_json::json!({
        "public_key": cn_core::node_key::public_key_hex(keys)
    });
    cn_core::admin::log_audit(
        &pool,
        "system",
        action,
        "node_key",
        Some(diff),
        None,
    )
    .await?;
    Ok(())
}

fn resolve_node_key_path(explicit: Option<String>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return Ok(PathBuf::from(path));
    }
    cn_core::node_key::key_path_from_env("NODE_KEY_PATH", "data/node_key.json")
}

async fn handle_p2p(command: P2pCommand) -> Result<()> {
    match command {
        P2pCommand::NodeId { args } => {
            init_logging(&args.log_level, args.json_logs)?;
            let bind_addr = SocketAddr::from_str(&args.bind)?;
            let builder = Endpoint::builder();
            let builder = apply_bind_address(builder, bind_addr);
            let builder = apply_secret_key(builder, &args.secret_key)?;
            let endpoint = builder.bind().await?;
            println!("{}", endpoint.id());
        }
        P2pCommand::Bootstrap {
            args,
            peers,
            export_path,
        } => {
            init_logging(&args.log_level, args.json_logs)?;
            run_bootstrap_node(&args, peers, export_path).await?;
        }
        P2pCommand::Relay { args, topics } => {
            init_logging(&args.log_level, args.json_logs)?;
            run_relay_node(&args, &topics).await?;
        }
        P2pCommand::Connect {
            args,
            peer,
            no_dht,
            mdns,
            timeout,
        } => {
            init_logging(&args.log_level, args.json_logs)?;
            run_connectivity_probe(&args, &peer, !no_dht, mdns, timeout).await?;
        }
    }

    Ok(())
}

fn apply_secret_key(
    mut builder: EndpointBuilder,
    secret_key: &Option<String>,
) -> Result<EndpointBuilder> {
    if let Some(encoded) = secret_key {
        let decoded = BASE64_STANDARD
            .decode(encoded.trim())
            .map_err(|e| anyhow!("Failed to decode secret key: {e}"))?;
        if decoded.len() != 32 {
            return Err(anyhow!(
                "Secret key must decode to 32 bytes, got {}",
                decoded.len()
            ));
        }
        let mut buf = [0u8; 32];
        buf.copy_from_slice(&decoded);
        let secret = SecretKey::from_bytes(&buf);
        builder = builder.secret_key(secret);
    }
    Ok(builder)
}

fn apply_bind_address(builder: EndpointBuilder, bind_addr: SocketAddr) -> EndpointBuilder {
    match bind_addr {
        SocketAddr::V4(addr) => builder.bind_addr_v4(addr),
        SocketAddr::V6(addr) => builder.bind_addr_v6(addr),
    }
}

fn apply_discovery_services(
    mut builder: EndpointBuilder,
    enable_dht: bool,
    enable_mdns: bool,
    static_discovery: &Arc<StaticProvider>,
) -> EndpointBuilder {
    builder = builder.clear_discovery();
    builder = builder.discovery(PkarrPublisher::n0_dns());
    builder = builder.discovery(DnsDiscovery::n0_dns());
    if enable_dht {
        builder = builder.discovery(
            DhtDiscovery::builder()
                .include_direct_addresses(true)
                .n0_dns_pkarr_relay(),
        );
    }
    if enable_mdns {
        builder = builder.discovery(MdnsDiscovery::builder());
    }
    builder.discovery(static_discovery.clone())
}

async fn run_bootstrap_node(
    args: &P2pArgs,
    bootstrap_peers: Vec<String>,
    export_path: Option<String>,
) -> Result<()> {
    info!("Starting DHT bootstrap node on {}", args.bind);

    let bind_addr = SocketAddr::from_str(&args.bind)?;
    let static_discovery = Arc::new(StaticProvider::new());
    let builder = Endpoint::builder();
    let builder = apply_bind_address(builder, bind_addr);
    let builder = apply_secret_key(builder, &args.secret_key)?;
    let builder = apply_discovery_services(builder, true, false, &static_discovery);
    let endpoint = builder.bind().await?;

    let node_id = endpoint.id();
    let _node_addr = endpoint.addr();

    info!("Node ID: {}", node_id);
    debug!("Node address configured");

    let mut peers = bootstrap_peers;
    if peers.is_empty() {
        let from_file = load_bootstrap_peers_from_json();
        if from_file.is_empty() {
            warn!("No bootstrap peers provided and none found in bootstrap_nodes.json");
        } else {
            info!("Loaded {} peers from bootstrap_nodes.json", from_file.len());
            peers.extend(from_file);
        }
    }

    for peer_str in peers.iter() {
        match parse_node_addr(peer_str) {
            Ok(node_addr) => {
                info!("Connecting to bootstrap peer: {}", node_addr.id);
                static_discovery.add_endpoint_info(node_addr.clone());
                if let Err(e) = endpoint.connect(node_addr.clone(), iroh_gossip::ALPN).await {
                    error!("Failed to connect to peer {}: {}", node_addr.id, e);
                }
            }
            Err(e) => error!("Invalid peer address '{}': {e}", peer_str),
        }
    }

    let gossip = Arc::new(Gossip::builder().spawn(endpoint.clone()));
    let _router = Router::builder(endpoint.clone())
        .accept(iroh_gossip::ALPN, gossip.clone())
        .spawn();
    let _gossip = gossip;

    info!("DHT bootstrap node is running. Press Ctrl+C to stop.");

    if let Some(path) = resolve_export_path(export_path) {
        if let Err(err) = export_bootstrap_list(&path, &node_id, &bind_addr, &peers) {
            warn!(
                "Failed to export bootstrap list to {}: {}",
                path.display(),
                err
            );
        } else {
            info!("Exported bootstrap list to {}", path.display());
        }
    }

    tokio::signal::ctrl_c().await?;
    info!("Shutting down bootstrap node...");

    Ok(())
}

fn export_bootstrap_list(
    path: &Path,
    node_id: &EndpointId,
    bind_addr: &SocketAddr,
    peers: &[String],
) -> Result<()> {
    let export_addr = if bind_addr.ip().is_unspecified() {
        let mut addr = *bind_addr;
        if addr.is_ipv4() {
            addr.set_ip(Ipv4Addr::LOCALHOST.into());
        } else {
            addr.set_ip(Ipv6Addr::LOCALHOST.into());
        }
        addr
    } else {
        *bind_addr
    };

    let mut nodes = peers.to_vec();
    nodes.push(format!("{}@{}", node_id, export_addr));

    let cache = CliBootstrapCache::new(nodes);
    write_cache(cache, path)?;
    Ok(())
}

async fn run_relay_node(args: &P2pArgs, topics: &str) -> Result<()> {
    info!("Starting relay node on {} for topics: {}", args.bind, topics);

    let bind_addr = SocketAddr::from_str(&args.bind)?;
    let static_discovery = Arc::new(StaticProvider::new());
    let builder = Endpoint::builder();
    let builder = apply_bind_address(builder, bind_addr);
    let builder = apply_secret_key(builder, &args.secret_key)?;
    let builder = apply_discovery_services(builder, true, false, &static_discovery);
    let endpoint = builder.bind().await?;

    let node_id = endpoint.id();
    let _node_addr = endpoint.addr();

    info!("Node ID: {}", node_id);
    debug!("Node address configured");

    let gossip = Arc::new(Gossip::builder().spawn(endpoint.clone()));
    let _router = Router::builder(endpoint.clone())
        .accept(iroh_gossip::ALPN, gossip.clone())
        .spawn();

    let mut subscribed = 0usize;
    for topic in topics.split(',') {
        let Some(canonical_topic) = generate_topic_id(topic) else {
            continue;
        };
        let topic_bytes = cn_core::topic::topic_id_to_gossip_bytes(&canonical_topic)?;

        info!("Subscribing to topic: {} -> {}", topic.trim(), canonical_topic);
        gossip.subscribe(topic_bytes.into(), vec![]).await?;
        subscribed += 1;
    }

    if subscribed == 0 {
        return Err(anyhow!(
            "No valid topics provided after applying namespace (input: {topics})"
        ));
    }

    info!("Relay node is running. Press Ctrl+C to stop.");

    tokio::signal::ctrl_c().await?;
    info!("Shutting down relay node...");

    Ok(())
}

async fn run_connectivity_probe(
    args: &P2pArgs,
    peer: &str,
    enable_dht: bool,
    enable_mdns: bool,
    timeout_secs: u64,
) -> Result<()> {
    info!("Connectivity probe using bind {} -> peer {}", args.bind, peer);

    let bind_addr = SocketAddr::from_str(&args.bind)?;
    let static_discovery = Arc::new(StaticProvider::new());
    let builder = Endpoint::builder();
    let builder = apply_bind_address(builder, bind_addr);
    let builder = apply_secret_key(builder, &args.secret_key)?;
    let builder = apply_discovery_services(builder, enable_dht, enable_mdns, &static_discovery);
    let endpoint = builder.bind().await?;

    let target = parse_peer_target(peer)?;
    let timeout_duration = Duration::from_secs(timeout_secs);

    let connect_fut = async {
        match target {
            PeerTarget::NodeId(node_id) => endpoint.connect(node_id, iroh_gossip::ALPN).await,
            PeerTarget::NodeAddr(node_addr) => endpoint.connect(node_addr, iroh_gossip::ALPN).await,
        }
    };

    match timeout(timeout_duration, connect_fut).await {
        Ok(Ok(_conn)) => {
            info!("Connection established to peer {}", peer);
        }
        Ok(Err(e)) => {
            return Err(anyhow!("Failed to connect: {e}"));
        }
        Err(_) => {
            return Err(anyhow!(
                "Connection attempt timed out after {:?}",
                timeout_duration
            ));
        }
    }

    Ok(())
}

fn init_logging(level: &str, json: bool) -> Result<()> {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));

    let fmt_layer = if json {
        fmt::layer().json().with_current_span(false).boxed()
    } else {
        fmt::layer()
            .with_target(false)
            .with_thread_ids(false)
            .with_thread_names(false)
            .boxed()
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();

    Ok(())
}

#[derive(Debug)]
enum PeerTarget {
    NodeId(EndpointId),
    NodeAddr(EndpointAddr),
}

fn parse_peer_target(s: &str) -> Result<PeerTarget> {
    if s.contains('@') {
        Ok(PeerTarget::NodeAddr(parse_node_addr(s)?))
    } else {
        Ok(PeerTarget::NodeId(EndpointId::from_str(s)?))
    }
}

fn parse_node_addr(s: &str) -> Result<EndpointAddr> {
    let parts: Vec<&str> = s.split('@').collect();
    if parts.len() != 2 {
        return Err(anyhow!("Invalid format. Expected: node_id@host:port"));
    }

    let node_id = EndpointId::from_str(parts[0])?;
    let address_part = parts[1];

    if let Ok(socket_addr) = SocketAddr::from_str(address_part) {
        return Ok(EndpointAddr::new(node_id).with_ip_addr(socket_addr));
    }

    let (host, port_str) = address_part
        .rsplit_once(':')
        .ok_or_else(|| anyhow!("Invalid format. Expected: node_id@host:port"))?;
    let port: u16 = port_str
        .parse()
        .map_err(|e| anyhow!("Invalid port `{}`: {}", port_str, e))?;

    let mut addrs_iter = (host, port)
        .to_socket_addrs()
        .map_err(|e| anyhow!("Failed to resolve host `{}`: {}", host, e))?;

    if let Some(addr) = addrs_iter.next() {
        Ok(EndpointAddr::new(node_id).with_ip_addr(addr))
    } else {
        Err(anyhow!(
            "Resolved host `{}` but no socket addresses were returned",
            host
        ))
    }
}

fn load_bootstrap_peers_from_json() -> Vec<String> {
    let path = std::env::var("KUKURI_BOOTSTRAP_CONFIG")
        .unwrap_or_else(|_| "bootstrap_nodes.json".to_string());

    match fs::read_to_string(&path) {
        Ok(contents) => match serde_json::from_str::<Value>(&contents) {
            Ok(Value::Array(arr)) => arr
                .into_iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            Ok(Value::Object(map)) => map
                .get("nodes")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            _ => {
                warn!("bootstrap_nodes.json does not contain an array or nodes field");
                Vec::new()
            }
        },
        Err(e) => {
            warn!("Failed to read bootstrap_nodes.json: {e}");
            Vec::new()
        }
    }
}

pub(crate) fn hash_topic_id(base: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(base.as_bytes());
    format!("{}{}", TOPIC_NAMESPACE, hex::encode(hasher.finalize().as_bytes()))
}

fn generate_topic_id(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.starts_with(TOPIC_NAMESPACE) {
        return Some(trimmed.to_string());
    }

    if trimmed.eq_ignore_ascii_case("public") {
        return Some(DEFAULT_PUBLIC_TOPIC_ID.to_string());
    }

    Some(hash_topic_id(trimmed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        sync::{Mutex, OnceLock},
    };

    const SAMPLE_NODE_ID: &str = "03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8";
    const SECOND_NODE_ID: &str = "02bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn temp_file(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        path.push(format!("kukuri_cli_{unique}_{name}"));
        path
    }

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn set_env_var(key: &str, value: &str) {
        unsafe { std::env::set_var(key, value) };
    }

    fn remove_env_var(key: &str) {
        unsafe { std::env::remove_var(key) };
    }

    #[test]
    fn generate_topic_id_normalizes_topics() {
        assert_eq!(
            super::generate_topic_id("Public"),
            Some(super::DEFAULT_PUBLIC_TOPIC_ID.to_string())
        );
        assert_eq!(
            super::generate_topic_id("kukuri:custom"),
            Some("kukuri:custom".to_string())
        );
        assert!(super::generate_topic_id("   ").is_none());
    }

    #[test]
    fn export_bootstrap_list_rewrites_unspecified_bind_addr() {
        let path = temp_file("bootstrap_unspecified.json");
        let node_id = EndpointId::from_str(SAMPLE_NODE_ID).unwrap();
        let bind_addr: SocketAddr = "0.0.0.0:9999".parse().unwrap();
        let peers = Vec::new();

        export_bootstrap_list(&path, &node_id, &bind_addr, &peers).unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        let cache: CliBootstrapCache = serde_json::from_str(&contents).unwrap();
        assert_eq!(cache.nodes[0], format!("{node_id}@127.0.0.1:9999"));

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn export_bootstrap_list_writes_unique_nodes() {
        let path = temp_file("bootstrap.json");
        let node_id = EndpointId::from_str(SAMPLE_NODE_ID).unwrap();
        let bind_addr: SocketAddr = "127.0.0.1:9999".parse().unwrap();
        let peers = vec![
            format!("{SECOND_NODE_ID}@10.0.0.2:7000"),
            format!("{SECOND_NODE_ID}@10.0.0.2:7000"),
            String::new(),
        ];

        export_bootstrap_list(&path, &node_id, &bind_addr, &peers).unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        let cache: CliBootstrapCache = serde_json::from_str(&contents).unwrap();
        assert_eq!(cache.nodes.len(), 2, "duplicates and blanks are removed");
        assert!(cache.nodes.contains(&format!("{node_id}@{bind_addr}")));
        assert!(cache
            .nodes
            .contains(&format!("{SECOND_NODE_ID}@10.0.0.2:7000")));

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn parse_node_addr_supports_ip_and_hostnames() {
        let ip_input = format!("{SAMPLE_NODE_ID}@127.0.0.1:32145");
        let ip_addr = parse_node_addr(&ip_input).expect("ip parse succeeds");
        assert_eq!(ip_addr.id, EndpointId::from_str(SAMPLE_NODE_ID).unwrap());

        let host_input = format!("{SAMPLE_NODE_ID}@localhost:32145");
        let host_addr = parse_node_addr(&host_input).expect("hostname parse succeeds");
        assert_eq!(host_addr.id, EndpointId::from_str(SAMPLE_NODE_ID).unwrap());
    }

    #[test]
    fn parse_node_addr_rejects_invalid_format() {
        let err = parse_node_addr("invalid-format").unwrap_err();
        assert!(
            err.to_string().contains("Invalid format"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn load_bootstrap_peers_reads_env_path() {
        let _guard = env_lock().lock().unwrap();
        let path = temp_file("bootstrap_nodes.json");
        fs::write(
            &path,
            r#"{"nodes": ["node-a@127.0.0.1:9000", "node-b@127.0.0.1:9001"]}"#,
        )
        .unwrap();
        set_env_var("KUKURI_BOOTSTRAP_CONFIG", path.to_str().unwrap());

        let peers = load_bootstrap_peers_from_json();
        assert_eq!(
            peers,
            vec![
                "node-a@127.0.0.1:9000".to_string(),
                "node-b@127.0.0.1:9001".to_string()
            ]
        );

        remove_env_var("KUKURI_BOOTSTRAP_CONFIG");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn resolve_export_path_prefers_explicit_values() {
        let _guard = env_lock().lock().unwrap();
        set_env_var(
            "KUKURI_CLI_BOOTSTRAP_PATH",
            "ignored_env_bootstrap_nodes.json",
        );

        let explicit = PathBuf::from("manual_bootstrap_nodes.json");
        let resolved = resolve_export_path(Some(explicit.to_string_lossy().into()))
            .expect("explicit path available");
        assert_eq!(resolved, explicit);

        remove_env_var("KUKURI_CLI_BOOTSTRAP_PATH");
    }

    #[test]
    fn resolve_export_path_uses_env_when_missing_explicit() {
        let _guard = env_lock().lock().unwrap();
        set_env_var("KUKURI_CLI_BOOTSTRAP_PATH", "env_bootstrap_nodes.json");
        let resolved = resolve_export_path(None).expect("env path available");
        assert!(resolved.ends_with("env_bootstrap_nodes.json"));
        remove_env_var("KUKURI_CLI_BOOTSTRAP_PATH");
    }
}
