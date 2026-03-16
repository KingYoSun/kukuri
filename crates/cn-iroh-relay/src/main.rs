use std::net::SocketAddr;

use anyhow::{Context, Result};
use clap::Parser;

#[derive(Debug, Parser)]
struct Args {
    #[arg(
        long,
        env = "COMMUNITY_NODE_IROH_RELAY_HTTP_BIND_ADDR",
        default_value = "127.0.0.1:3340"
    )]
    http_bind_addr: SocketAddr,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    kukuri_cn_iroh_relay::init_tracing();
    let server = kukuri_cn_iroh_relay::spawn_server(kukuri_cn_iroh_relay::IrohRelayConfig {
        http_bind_addr: args.http_bind_addr,
    })
    .await
    .context("failed to spawn iroh relay")?;
    tracing::info!(
        http_addr = %server.http_addr(),
        "community-node iroh relay listening"
    );

    tokio::signal::ctrl_c()
        .await
        .context("failed to wait for ctrl-c")?;
    drop(server);
    Ok(())
}
