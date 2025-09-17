use anyhow::{Result, anyhow};
use base64::prelude::*;
use clap::{Parser, Subcommand};
use iroh::{
    Endpoint, NodeAddr, NodeId, SecretKey, endpoint::Builder as EndpointBuilder, protocol::Router,
};
use iroh_gossip::net::Gossip;
use serde_json::Value;
use std::fs;
use std::net::{SocketAddr, ToSocketAddrs};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

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
    /// Run as DHT bootstrap node
    Bootstrap {
        /// Additional bootstrap peers (format: node_id@host:port)
        #[arg(long)]
        peers: Vec<String>,
    },
    /// Run as relay node
    Relay {
        /// Topics to relay (comma-separated)
        #[arg(long, default_value = "kukuri")]
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
        Commands::Bootstrap { ref peers } => run_bootstrap_node(&cli, peers.clone()).await?,
        Commands::Relay { ref topics } => run_relay_node(&cli, topics).await?,
        Commands::Connect {
            ref peer,
            no_dht,
            mdns,
            timeout,
        } => run_connectivity_probe(&cli, peer, !no_dht, mdns, timeout).await?,
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

async fn run_bootstrap_node(cli: &Cli, bootstrap_peers: Vec<String>) -> Result<()> {
    info!("Starting DHT bootstrap node on {}", cli.bind);

    let bind_addr = SocketAddr::from_str(&cli.bind)?;

    let builder = Endpoint::builder().discovery_dht();
    let builder = apply_bind_address(builder, bind_addr);
    let builder = apply_secret_key(builder, cli)?;
    let endpoint = builder.bind().await?;

    let node_id = endpoint.node_id();
    let _node_addr = endpoint.node_addr();

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
    for peer_str in peers {
        match parse_node_addr(&peer_str) {
            Ok(node_addr) => {
                info!("Connecting to bootstrap peer: {}", node_addr.node_id);
                if let Err(e) = endpoint.connect(node_addr.clone(), iroh_gossip::ALPN).await {
                    error!("Failed to connect to peer {}: {}", node_addr.node_id, e);
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

    // Keep the node running
    tokio::signal::ctrl_c().await?;
    info!("Shutting down bootstrap node...");

    Ok(())
}

async fn run_relay_node(cli: &Cli, topics: &str) -> Result<()> {
    info!("Starting relay node on {} for topics: {}", cli.bind, topics);

    let bind_addr = SocketAddr::from_str(&cli.bind)?;

    let builder = Endpoint::builder().discovery_dht();
    let builder = apply_bind_address(builder, bind_addr);
    let builder = apply_secret_key(builder, cli)?;
    let endpoint = builder.bind().await?;

    let node_id = endpoint.node_id();
    let _node_addr = endpoint.node_addr();

    info!("Node ID: {}", node_id);
    debug!("Node address configured");

    // Create gossip service
    let gossip = Arc::new(Gossip::builder().spawn(endpoint.clone()));
    let _router = Router::builder(endpoint.clone())
        .accept(iroh_gossip::ALPN, gossip.clone())
        .spawn();

    // Subscribe to topics
    let topic_list: Vec<&str> = topics.split(',').collect();
    for topic in topic_list {
        let topic_id = blake3::hash(topic.as_bytes());
        let topic_bytes = *topic_id.as_bytes();

        info!("Subscribing to topic: {} (hash: {})", topic, topic_id);
        gossip.subscribe(topic_bytes.into(), vec![]).await?;
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

    let mut builder = Endpoint::builder();
    if enable_dht {
        builder = builder.discovery_dht();
    }

    if enable_mdns {
        builder = builder.discovery_local_network();
    }

    builder = apply_bind_address(builder, bind_addr);
    let builder = apply_secret_key(builder, cli)?;
    let endpoint = builder.bind().await?;
    info!("Local node id: {}", endpoint.node_id());

    let peer_target = parse_peer_target(peer)?;
    let timeout_duration = Duration::from_secs(timeout_secs);

    info!(
        "Attempting connection using {:?} with timeout {:?}",
        peer_target, timeout_duration
    );

    let connect_result = timeout(timeout_duration, async {
        match peer_target {
            PeerTarget::NodeAddr(ref addr) => {
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
    NodeId(NodeId),
    NodeAddr(NodeAddr),
}

fn parse_peer_target(s: &str) -> Result<PeerTarget> {
    if s.contains('@') {
        Ok(PeerTarget::NodeAddr(parse_node_addr(s)?))
    } else {
        Ok(PeerTarget::NodeId(NodeId::from_str(s)?))
    }
}

fn parse_node_addr(s: &str) -> Result<NodeAddr> {
    // Format: node_id@host:port
    let parts: Vec<&str> = s.split('@').collect();
    if parts.len() != 2 {
        return Err(anyhow!("Invalid format. Expected: node_id@host:port"));
    }

    let node_id = NodeId::from_str(parts[0])?;
    let address_part = parts[1];

    if let Ok(socket_addr) = SocketAddr::from_str(address_part) {
        return Ok(NodeAddr::new(node_id).with_direct_addresses([socket_addr]));
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
        Ok(NodeAddr::new(node_id).with_direct_addresses([addr]))
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
