use anyhow::{Context, Result};

#[tokio::main]
async fn main() -> Result<()> {
    kukuri_cn_iroh_relay::init_tracing();
    let config = kukuri_cn_iroh_relay::IrohRelayConfig::from_env()?;
    let server = kukuri_cn_iroh_relay::spawn_server(config)
        .await
        .context("failed to spawn iroh relay")?;
    tracing::info!(
        http_addr = %server.http_addr(),
        https_addr = ?server.https_addr(),
        quic_addr = ?server.quic_addr(),
        "community-node iroh relay listening"
    );

    tokio::signal::ctrl_c()
        .await
        .context("failed to wait for ctrl-c")?;
    drop(server);
    Ok(())
}
