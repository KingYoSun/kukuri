use anyhow::Result;
use clap::{Parser, Subcommand};
use iroh::{Endpoint, NodeId};
use iroh_gossip::net::Gossip;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{error, info, debug};

#[derive(Parser)]
#[command(name = "kukuri-cli")]
#[command(about = "Kukuri DHT Bootstrap Node", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Bind address for the node
    #[arg(short, long, default_value = "0.0.0.0:11223", env = "BIND_ADDRESS")]
    bind: String,
    
    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info", env = "LOG_LEVEL")]
    log_level: String,
    
    /// Enable JSON logging
    #[arg(long, env = "JSON_LOGS")]
    json_logs: bool,
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
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    init_logging(&cli.log_level, cli.json_logs)?;
    
    info!("Starting Kukuri CLI node v{}", env!("CARGO_PKG_VERSION"));
    
    match cli.command {
        Commands::Bootstrap { peers } => run_bootstrap_node(&cli.bind, peers).await?,
        Commands::Relay { topics } => run_relay_node(&cli.bind, &topics).await?,
    }
    
    Ok(())
}

async fn run_bootstrap_node(bind_addr: &str, bootstrap_peers: Vec<String>) -> Result<()> {
    info!("Starting DHT bootstrap node on {}", bind_addr);
    
    // Parse bind address
    let _bind_addr = SocketAddr::from_str(bind_addr)?;
    
    // Create iroh endpoint
    let endpoint = Endpoint::builder()
        .bind()
        .await?;
    
    let node_id = endpoint.node_id();
    let _node_addr = endpoint.node_addr();
    
    info!("Node ID: {}", node_id);
    debug!("Node address configured");
    
    // Parse and connect to bootstrap peers
    for peer_str in bootstrap_peers {
        if let Ok(node_addr) = parse_node_addr(&peer_str) {
            info!("Connecting to bootstrap peer: {}", node_addr.node_id);
            if let Err(e) = endpoint.connect(node_addr.clone(), iroh_gossip::ALPN).await {
                error!("Failed to connect to peer {}: {}", node_addr.node_id, e);
            }
        } else {
            error!("Invalid peer address format: {}", peer_str);
        }
    }
    
    // Create gossip service for topic-based messaging
    let _gossip = Arc::new(Gossip::builder().spawn(endpoint.clone()));
    
    info!("DHT bootstrap node is running. Press Ctrl+C to stop.");
    
    // Keep the node running
    tokio::signal::ctrl_c().await?;
    info!("Shutting down bootstrap node...");
    
    Ok(())
}

async fn run_relay_node(bind_addr: &str, topics: &str) -> Result<()> {
    info!("Starting relay node on {} for topics: {}", bind_addr, topics);
    
    // Parse bind address
    let _bind_addr = SocketAddr::from_str(bind_addr)?;
    
    // Create iroh endpoint with DHT discovery
    let endpoint = Endpoint::builder()
        .discovery(iroh::discovery::pkarr::dht::DhtDiscovery::default())
        .bind()
        .await?;
    
    let node_id = endpoint.node_id();
    let _node_addr = endpoint.node_addr();
    
    info!("Node ID: {}", node_id);
    debug!("Node address configured");
    
    // Create gossip service
    let gossip = Arc::new(Gossip::builder().spawn(endpoint.clone()));
    
    // Subscribe to topics
    let topic_list: Vec<&str> = topics.split(',').collect();
    for topic in topic_list {
        let topic_id = blake3::hash(topic.as_bytes());
        let topic_bytes = *topic_id.as_bytes();
        
        info!("Subscribing to topic: {} (hash: {})", topic, topic_id);
        gossip.subscribe(topic_bytes.into(), vec![]).await?;
    }
    
    info!("Relay node is running. Press Ctrl+C to stop.");
    
    // Keep the node running
    tokio::signal::ctrl_c().await?;
    info!("Shutting down relay node...");
    
    Ok(())
}

fn init_logging(level: &str, json: bool) -> Result<()> {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};
    
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level));
    
    let fmt_layer = if json {
        fmt::layer()
            .json()
            .with_current_span(false)
            .boxed()
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

fn parse_node_addr(s: &str) -> Result<iroh::NodeAddr> {
    // Format: node_id@host:port
    let parts: Vec<&str> = s.split('@').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid format. Expected: node_id@host:port");
    }
    
    let node_id = NodeId::from_str(parts[0])?;
    let socket_addr = SocketAddr::from_str(parts[1])?;
    
    Ok(iroh::NodeAddr::new(node_id).with_direct_addresses([socket_addr]))
}