use anyhow::{Result, anyhow};
use base64::prelude::*;
use clap::{Parser, Subcommand};
use iroh::{
    Endpoint, EndpointAddr, EndpointId, SecretKey,
    discovery::{
        dns::DnsDiscovery,
        mdns::MdnsDiscovery,
        pkarr::{PkarrPublisher, dht::DhtDiscovery},
        static_provider::StaticProvider,
    },
    endpoint::Builder as EndpointBuilder,
    protocol::Router,
};
use iroh_gossip::net::Gossip;
use serde_json::Value;
use std::fs;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

mod bootstrap_cache;

use bootstrap_cache::{CliBootstrapCache, resolve_export_path, write_cache};

const TOPIC_NAMESPACE: &str = "kukuri:tauri:";
const DEFAULT_PUBLIC_TOPIC_ID: &str = "kukuri:tauri:public";

#[derive(Parser)]
#[command(name = "kukuri-cli")]
#[command(about = "Kukuri DHT Bootstrap / Relay / Connectivity helper", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Bind address for the node (currently used for logging only)
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
enum Commands {
    /// Print the node ID derived from the configured secret key
    NodeId,
    /// Run as DHT bootstrap node
    Bootstrap {
        /// Additional bootstrap peers (format: node_id@host:port)
        #[arg(long)]
        peers: Vec<String>,
        /// Optional export path for writing discovered bootstrap list
        #[arg(long)]
        export_path: Option<String>,
    },
    /// Run as relay node
    Relay {
        /// Topics to relay (comma-separated)
        #[arg(long, default_value = DEFAULT_PUBLIC_TOPIC_ID, env = "RELAY_TOPICS")]
        topics: String,
    },
    /// Attempt to connect to a peer and exit (for connectivity debugging)
    Connect {
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

    // Initialize logging
    init_logging(&cli.log_level, cli.json_logs)?;

    info!("Starting Kukuri CLI node v{}", env!("CARGO_PKG_VERSION"));

    match cli.command {
        Commands::Bootstrap {
            ref peers,
            ref export_path,
        } => run_bootstrap_node(&cli, peers.clone(), export_path.clone()).await?,
        Commands::Relay { ref topics } => run_relay_node(&cli, topics).await?,
        Commands::Connect {
            ref peer,
            no_dht,
            mdns,
            timeout,
        } => run_connectivity_probe(&cli, peer, !no_dht, mdns, timeout).await?,
        Commands::NodeId => {
            let bind_addr = SocketAddr::from_str(&cli.bind)?;
            let builder = Endpoint::builder();
            let builder = apply_bind_address(builder, bind_addr);
            let builder = apply_secret_key(builder, &cli)?;
            let endpoint = builder.bind().await?;
            println!("{}", endpoint.id());
        }
    }

    Ok(())
}

fn apply_secret_key(mut builder: EndpointBuilder, cli: &Cli) -> Result<EndpointBuilder> {
    if let Some(ref encoded) = cli.secret_key {
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
    cli: &Cli,
    bootstrap_peers: Vec<String>,
    export_path: Option<String>,
) -> Result<()> {
    info!("Starting DHT bootstrap node on {}", cli.bind);

    let bind_addr = SocketAddr::from_str(&cli.bind)?;

    let static_discovery = Arc::new(StaticProvider::new());
    let builder = Endpoint::builder();
    let builder = apply_bind_address(builder, bind_addr);
    let builder = apply_secret_key(builder, cli)?;
    let builder = apply_discovery_services(builder, true, false, &static_discovery);
    let endpoint = builder.bind().await?;

    let node_id = endpoint.id();
    let _node_addr = endpoint.addr();

    info!("Node ID: {}", node_id);
    debug!("Node address configured");

    // Resolve peers: CLI args first, then JSON config if empty
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

    // Parse and connect to bootstrap peers
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

    // Create gossip service and router for topic-based messaging
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

    // Keep the node running
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
    let mut entries = Vec::new();
    entries.push(format!("{node_id}@{bind_addr}"));
    for peer in peers {
        if !entries.iter().any(|existing| existing == peer) {
            entries.push(peer.clone());
        }
    }
    let cache = CliBootstrapCache::new(entries);
    write_cache(cache, path)
}

fn ensure_kukuri_namespace(topic: &str) -> Option<String> {
    let trimmed = topic.trim();
    if trimmed.is_empty() {
        return None;
    }
    let normalized = trimmed.to_lowercase();
    if normalized == "public" || normalized == DEFAULT_PUBLIC_TOPIC_ID {
        return Some(DEFAULT_PUBLIC_TOPIC_ID.to_string());
    }
    if normalized.starts_with(TOPIC_NAMESPACE) {
        Some(normalized)
    } else {
        Some(format!("{TOPIC_NAMESPACE}{normalized}"))
    }
}

fn topic_bytes(canonical: &str) -> [u8; 32] {
    if let Some(tail) = canonical.strip_prefix(TOPIC_NAMESPACE) {
        if tail.len() == 64 && tail.chars().all(|c| c.is_ascii_hexdigit()) {
            if let Ok(decoded) = hex::decode(tail) {
                if decoded.len() >= 32 {
                    let mut out = [0u8; 32];
                    out.copy_from_slice(&decoded[..32]);
                    return out;
                }
            }
        }
        let mut out = [0u8; 32];
        let bytes = canonical.as_bytes();
        if bytes.len() >= 32 {
            out.copy_from_slice(&bytes[..32]);
        } else {
            out[..bytes.len()].copy_from_slice(bytes);
        }
        return out;
    }

    *blake3::hash(canonical.as_bytes()).as_bytes()
}

async fn run_relay_node(cli: &Cli, topics: &str) -> Result<()> {
    info!("Starting relay node on {} for topics: {}", cli.bind, topics);

    let bind_addr = SocketAddr::from_str(&cli.bind)?;

    let static_discovery = Arc::new(StaticProvider::new());
    let builder = Endpoint::builder();
    let builder = apply_bind_address(builder, bind_addr);
    let builder = apply_secret_key(builder, cli)?;
    let builder = apply_discovery_services(builder, true, false, &static_discovery);
    let endpoint = builder.bind().await?;

    let node_id = endpoint.id();
    let _node_addr = endpoint.addr();

    info!("Node ID: {}", node_id);
    debug!("Node address configured");

    // Create gossip service
    let gossip = Arc::new(Gossip::builder().spawn(endpoint.clone()));
    let _router = Router::builder(endpoint.clone())
        .accept(iroh_gossip::ALPN, gossip.clone())
        .spawn();

    // Subscribe to topics
    let mut subscribed = 0usize;
    for topic in topics.split(',') {
        let Some(namespaced_topic) = ensure_kukuri_namespace(topic) else {
            continue;
        };
        let topic_bytes = topic_bytes(&namespaced_topic);

        info!(
            "Subscribing to topic: {} -> {}",
            topic.trim(),
            namespaced_topic
        );
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
    cli: &Cli,
    peer: &str,
    enable_dht: bool,
    enable_mdns: bool,
    timeout_secs: u64,
) -> Result<()> {
    info!(
        "Connectivity probe using bind {} -> peer {}",
        cli.bind, peer
    );

    let bind_addr = SocketAddr::from_str(&cli.bind)?;

    let static_discovery = Arc::new(StaticProvider::new());
    let builder = Endpoint::builder();
    let builder = apply_bind_address(builder, bind_addr);
    let builder = apply_secret_key(builder, cli)?;
    let builder = apply_discovery_services(builder, enable_dht, enable_mdns, &static_discovery);
    let endpoint = builder.bind().await?;
    info!("Local node id: {}", endpoint.id());

    let peer_target = parse_peer_target(peer)?;
    let timeout_duration = Duration::from_secs(timeout_secs);

    info!(
        "Attempting connection using {:?} with timeout {:?}",
        peer_target, timeout_duration
    );

    let connect_result = timeout(timeout_duration, async {
        match peer_target {
            PeerTarget::NodeAddr(ref addr) => {
                static_discovery.add_endpoint_info(addr.clone());
                endpoint.connect(addr.clone(), iroh_gossip::ALPN).await
            }
            PeerTarget::NodeId(id) => endpoint.connect(id, iroh_gossip::ALPN).await,
        }
    })
    .await;

    match connect_result {
        Ok(Ok(_connection)) => {
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
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};

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
    // Format: node_id@host:port
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
    // Path via env or default to local file
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap_cache::CliBootstrapCache;
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
    fn ensure_kukuri_namespace_normalizes_topics() {
        assert_eq!(
            super::ensure_kukuri_namespace("Public Topic"),
            Some("kukuri:tauri:public topic".to_string())
        );
        assert_eq!(
            super::ensure_kukuri_namespace("kukuri:tauri:custom"),
            Some("kukuri:tauri:custom".to_string())
        );
        assert_eq!(
            super::ensure_kukuri_namespace("public"),
            Some(super::DEFAULT_PUBLIC_TOPIC_ID.to_string())
        );
        assert_eq!(super::ensure_kukuri_namespace("   "), None);
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
        assert_eq!(cache.nodes[0], format!("{node_id}@{bind_addr}"));
        assert_eq!(cache.nodes.len(), 2, "duplicates and blanks are removed");
        assert!(
            cache
                .nodes
                .contains(&format!("{SECOND_NODE_ID}@10.0.0.2:7000"))
        );

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
