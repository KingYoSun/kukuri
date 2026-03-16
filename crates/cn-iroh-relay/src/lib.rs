use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use iroh_relay::defaults::DEFAULT_RELAY_QUIC_PORT;
use iroh_relay::server::{
    AccessConfig, CertConfig, QuicConfig, RelayConfig as HttpRelayConfig, Server, ServerConfig,
    TlsConfig,
};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tracing_subscriber::EnvFilter;

const DEFAULT_HTTP_BIND_ADDR: &str = "127.0.0.1:3340";
const DEFAULT_TLS_CERT_PATH: &str = "/certs/default.crt";
const DEFAULT_TLS_KEY_PATH: &str = "/certs/default.key";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrohRelayTlsConfig {
    pub https_bind_addr: SocketAddr,
    pub quic_bind_addr: Option<SocketAddr>,
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
}

impl IrohRelayTlsConfig {
    fn effective_quic_bind_addr(&self) -> SocketAddr {
        self.quic_bind_addr
            .unwrap_or_else(|| SocketAddr::new(self.https_bind_addr.ip(), DEFAULT_RELAY_QUIC_PORT))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrohRelayConfig {
    pub http_bind_addr: SocketAddr,
    pub tls: Option<IrohRelayTlsConfig>,
}

impl IrohRelayConfig {
    pub fn from_env() -> Result<Self> {
        Self::from_lookup(&|var_name| std::env::var(var_name).ok())
    }

    fn from_lookup<F>(lookup: &F) -> Result<Self>
    where
        F: Fn(&str) -> Option<String>,
    {
        Ok(Self {
            http_bind_addr: parse_socket_addr_lookup(
                lookup,
                "COMMUNITY_NODE_IROH_RELAY_HTTP_BIND_ADDR",
                DEFAULT_HTTP_BIND_ADDR,
            )?,
            tls: parse_tls_config_from_lookup(lookup)?,
        })
    }
}

pub struct SpawnedIrohRelay {
    _server: Server,
    http_addr: SocketAddr,
    https_addr: Option<SocketAddr>,
    quic_addr: Option<SocketAddr>,
}

impl SpawnedIrohRelay {
    pub fn http_addr(&self) -> SocketAddr {
        self.http_addr
    }

    pub fn https_addr(&self) -> Option<SocketAddr> {
        self.https_addr
    }

    pub fn quic_addr(&self) -> Option<SocketAddr> {
        self.quic_addr
    }
}

pub async fn spawn_server(config: IrohRelayConfig) -> Result<SpawnedIrohRelay> {
    let (relay_tls, quic) = match config.tls.as_ref() {
        Some(tls_config) => {
            let (certs, server_config) = load_tls_materials(tls_config)?;
            let relay_tls = TlsConfig {
                https_bind_addr: tls_config.https_bind_addr,
                quic_bind_addr: tls_config.effective_quic_bind_addr(),
                cert: CertConfig::<(), ()>::Manual { certs },
                server_config: server_config.clone(),
            };
            let quic = tls_config.quic_bind_addr.map(|bind_addr| QuicConfig {
                bind_addr,
                server_config,
            });
            (Some(relay_tls), quic)
        }
        None => (None, None),
    };

    let server_config = ServerConfig::<(), ()> {
        relay: Some(HttpRelayConfig {
            http_bind_addr: config.http_bind_addr,
            tls: relay_tls,
            limits: Default::default(),
            key_cache_capacity: Some(1024),
            access: AccessConfig::Everyone,
        }),
        quic,
        metrics_addr: None,
    };
    let server = Server::spawn(server_config)
        .await
        .context("failed to spawn iroh relay")?;
    let http_addr = server
        .http_addr()
        .context("iroh relay did not expose an http listener")?;
    let https_addr = server.https_addr();
    let quic_addr = server.quic_addr();
    Ok(SpawnedIrohRelay {
        _server: server,
        http_addr,
        https_addr,
        quic_addr,
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

fn parse_tls_config_from_lookup<F>(lookup: &F) -> Result<Option<IrohRelayTlsConfig>>
where
    F: Fn(&str) -> Option<String>,
{
    let https_bind_addr =
        optional_socket_addr_lookup(lookup, "COMMUNITY_NODE_IROH_RELAY_HTTPS_BIND_ADDR")?;
    let quic_bind_addr =
        optional_socket_addr_lookup(lookup, "COMMUNITY_NODE_IROH_RELAY_QUIC_BIND_ADDR")?;
    let cert_path = optional_path_lookup(lookup, "COMMUNITY_NODE_IROH_RELAY_TLS_CERT_PATH");
    let key_path = optional_path_lookup(lookup, "COMMUNITY_NODE_IROH_RELAY_TLS_KEY_PATH");

    if https_bind_addr.is_none()
        && quic_bind_addr.is_none()
        && cert_path.is_none()
        && key_path.is_none()
    {
        return Ok(None);
    }

    let https_bind_addr = https_bind_addr.context(
        "COMMUNITY_NODE_IROH_RELAY_HTTPS_BIND_ADDR is required when enabling iroh relay TLS/QUIC",
    )?;
    Ok(Some(IrohRelayTlsConfig {
        https_bind_addr,
        quic_bind_addr,
        cert_path: cert_path.unwrap_or_else(|| PathBuf::from(DEFAULT_TLS_CERT_PATH)),
        key_path: key_path.unwrap_or_else(|| PathBuf::from(DEFAULT_TLS_KEY_PATH)),
    }))
}

fn parse_socket_addr_lookup<F>(lookup: &F, var_name: &str, default: &str) -> Result<SocketAddr>
where
    F: Fn(&str) -> Option<String>,
{
    lookup_value(lookup, var_name)
        .unwrap_or_else(|| default.to_string())
        .parse()
        .with_context(|| format!("failed to parse {var_name}"))
}

fn optional_socket_addr_lookup<F>(lookup: &F, var_name: &str) -> Result<Option<SocketAddr>>
where
    F: Fn(&str) -> Option<String>,
{
    lookup_value(lookup, var_name)
        .map(|value| {
            value
                .parse()
                .with_context(|| format!("failed to parse {var_name}"))
        })
        .transpose()
}

fn optional_path_lookup<F>(lookup: &F, var_name: &str) -> Option<PathBuf>
where
    F: Fn(&str) -> Option<String>,
{
    lookup_value(lookup, var_name).map(PathBuf::from)
}

fn lookup_value<F>(lookup: &F, var_name: &str) -> Option<String>
where
    F: Fn(&str) -> Option<String>,
{
    lookup(var_name)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn load_tls_materials(
    config: &IrohRelayTlsConfig,
) -> Result<(Vec<CertificateDer<'static>>, rustls::ServerConfig)> {
    let certs = load_certs(config.cert_path.as_path())?;
    if certs.is_empty() {
        bail!(
            "iroh relay certificate file `{}` did not contain any certificates",
            config.cert_path.display()
        );
    }
    let private_key = load_secret_key(config.key_path.as_path())?;
    let server_config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .expect("protocols supported by ring")
    .with_no_client_auth()
    .with_single_cert(certs.clone(), private_key)
    .context("failed to build iroh relay tls server config")?;
    Ok((certs, server_config))
}

fn load_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>> {
    CertificateDer::pem_file_iter(path)
        .with_context(|| format!("failed to open certificate file `{}`", path.display()))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("failed to read certificates from `{}`", path.display()))
}

fn load_secret_key(path: &Path) -> Result<PrivateKeyDer<'static>> {
    PrivateKeyDer::from_pem_file(path)
        .with_context(|| format!("failed to read private key from `{}`", path.display()))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::net::{IpAddr, Ipv4Addr};

    use super::*;

    #[test]
    fn from_env_defaults_to_http_only() {
        let config = config_from_vars(&[]).expect("config");

        assert_eq!(
            config.http_bind_addr,
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 3340)
        );
        assert!(config.tls.is_none());
    }

    #[test]
    fn from_env_parses_tls_and_quic_inputs() {
        let config = config_from_vars(&[
            ("COMMUNITY_NODE_IROH_RELAY_HTTP_BIND_ADDR", "0.0.0.0:3340"),
            ("COMMUNITY_NODE_IROH_RELAY_HTTPS_BIND_ADDR", "0.0.0.0:3443"),
            ("COMMUNITY_NODE_IROH_RELAY_QUIC_BIND_ADDR", "0.0.0.0:7842"),
            ("COMMUNITY_NODE_IROH_RELAY_TLS_CERT_PATH", "/certs/iroh.crt"),
            ("COMMUNITY_NODE_IROH_RELAY_TLS_KEY_PATH", "/certs/iroh.key"),
        ])
        .expect("config");
        let tls = config.tls.expect("tls config");

        assert_eq!(
            config.http_bind_addr,
            SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 3340)
        );
        assert_eq!(
            tls.https_bind_addr,
            SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 3443)
        );
        assert_eq!(
            tls.quic_bind_addr,
            Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 7842))
        );
        assert_eq!(tls.cert_path, PathBuf::from("/certs/iroh.crt"));
        assert_eq!(tls.key_path, PathBuf::from("/certs/iroh.key"));
    }

    #[test]
    fn from_env_rejects_quic_without_https_bind_addr() {
        let error =
            config_from_vars(&[("COMMUNITY_NODE_IROH_RELAY_QUIC_BIND_ADDR", "0.0.0.0:7842")])
                .expect_err("expected error");

        assert!(
            error
                .to_string()
                .contains("COMMUNITY_NODE_IROH_RELAY_HTTPS_BIND_ADDR is required")
        );
    }

    fn config_from_vars(vars: &[(&str, &str)]) -> Result<IrohRelayConfig> {
        let values = vars
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect::<BTreeMap<_, _>>();
        IrohRelayConfig::from_lookup(&|var_name| values.get(var_name).cloned())
    }
}
