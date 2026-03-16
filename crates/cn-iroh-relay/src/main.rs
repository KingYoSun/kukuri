use anyhow::{Context, Result};

#[tokio::main]
async fn main() -> Result<()> {
    kukuri_cn_iroh_relay::init_tracing();
    let config = kukuri_cn_iroh_relay::IrohRelayConfig::from_env()?;
    match config.public_origin_mode() {
        kukuri_cn_iroh_relay::IrohRelayPublicOriginMode::LocalHttps => tracing::info!(
            http_origin = %config.http_bind_addr,
            https_origin = ?config.tls.as_ref().and_then(|tls| tls.https_bind_addr),
            "cn-iroh-relay will serve relay traffic over the local HTTPS listener; when TLS is enabled the HTTP listener only serves /generate_204, so reverse proxies must target the HTTPS origin"
        ),
        kukuri_cn_iroh_relay::IrohRelayPublicOriginMode::UpstreamTlsTermination => tracing::info!(
            http_origin = %config.http_bind_addr,
            quic_origin = ?config.tls.as_ref().and_then(|tls| tls.quic_bind_addr),
            "cn-iroh-relay will serve relay traffic over the local HTTP listener for upstream TLS termination; point the public HTTPS reverse proxy or tunnel to the HTTP origin"
        ),
        kukuri_cn_iroh_relay::IrohRelayPublicOriginMode::HttpOnly => tracing::info!(
            http_origin = %config.http_bind_addr,
            "cn-iroh-relay is running without local TLS or QUIC"
        ),
    }
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
