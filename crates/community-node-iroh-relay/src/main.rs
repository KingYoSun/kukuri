use std::net::SocketAddr;

use anyhow::{Context, Result};
use clap::Parser;
use iroh_relay::server::{AccessConfig, RelayConfig as HttpRelayConfig, Server, ServerConfig};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, env = "COMMUNITY_NODE_IROH_RELAY_HTTP_BIND_ADDR", default_value = "127.0.0.1:3340")]
    http_bind_addr: SocketAddr,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    init_tracing();

    let config = ServerConfig::<(), ()> {
        relay: Some(HttpRelayConfig {
            http_bind_addr: args.http_bind_addr,
            tls: None,
            limits: Default::default(),
            key_cache_capacity: Some(1024),
            access: AccessConfig::Everyone,
        }),
        quic: None,
        metrics_addr: None,
    };
    let server = Server::spawn(config).await.context("failed to spawn iroh relay")?;
    tracing::info!(
        http_addr = ?server.http_addr(),
        https_addr = ?server.https_addr(),
        "community-node iroh relay listening"
    );

    tokio::signal::ctrl_c()
        .await
        .context("failed to wait for ctrl-c")?;
    drop(server);
    Ok(())
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,kukuri_community_node_iroh_relay=debug"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .try_init();
}
