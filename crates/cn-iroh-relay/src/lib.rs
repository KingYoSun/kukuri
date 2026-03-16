use std::net::SocketAddr;

use anyhow::{Context, Result};
use iroh_relay::server::{AccessConfig, RelayConfig as HttpRelayConfig, Server, ServerConfig};
use tracing_subscriber::EnvFilter;

#[derive(Clone, Copy, Debug)]
pub struct IrohRelayConfig {
    pub http_bind_addr: SocketAddr,
}

impl IrohRelayConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            http_bind_addr: std::env::var("COMMUNITY_NODE_IROH_RELAY_HTTP_BIND_ADDR")
                .unwrap_or_else(|_| "127.0.0.1:3340".to_string())
                .parse()
                .context("failed to parse COMMUNITY_NODE_IROH_RELAY_HTTP_BIND_ADDR")?,
        })
    }
}

pub struct SpawnedIrohRelay {
    _server: Server,
    http_addr: SocketAddr,
}

impl SpawnedIrohRelay {
    pub fn http_addr(&self) -> SocketAddr {
        self.http_addr
    }
}

pub async fn spawn_server(config: IrohRelayConfig) -> Result<SpawnedIrohRelay> {
    let server_config = ServerConfig::<(), ()> {
        relay: Some(HttpRelayConfig {
            http_bind_addr: config.http_bind_addr,
            tls: None,
            limits: Default::default(),
            key_cache_capacity: Some(1024),
            access: AccessConfig::Everyone,
        }),
        quic: None,
        metrics_addr: None,
    };
    let server = Server::spawn(server_config)
        .await
        .context("failed to spawn iroh relay")?;
    let http_addr = server
        .http_addr()
        .context("iroh relay did not expose an http listener")?;
    Ok(SpawnedIrohRelay {
        _server: server,
        http_addr,
    })
}

pub fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,kukuri_cn_iroh_relay=debug"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .try_init();
}
