use std::collections::BTreeSet;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::str::FromStr;

use anyhow::{Context, Result};
use iroh::{RelayMap, RelayMode, RelayUrl};
use pkarr::Client as PkarrClient;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransportNetworkConfig {
    pub bind_addr: SocketAddr,
    pub advertised_host: Option<String>,
    pub advertised_port: Option<u16>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryMode {
    #[default]
    StaticPeer,
    SeededDht,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectMode {
    #[default]
    DirectOnly,
    DirectOrRelay,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportRelayConfig {
    #[serde(default)]
    pub iroh_relay_urls: Vec<String>,
}

impl TransportRelayConfig {
    pub fn normalized(mut self) -> Self {
        self.iroh_relay_urls = self
            .iroh_relay_urls
            .into_iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        self
    }

    pub fn connect_mode(&self) -> ConnectMode {
        if self.iroh_relay_urls.is_empty() {
            ConnectMode::DirectOnly
        } else {
            ConnectMode::DirectOrRelay
        }
    }

    pub fn parsed_relay_urls(&self) -> Result<Vec<RelayUrl>> {
        self.iroh_relay_urls
            .iter()
            .map(|value| {
                value
                    .parse::<RelayUrl>()
                    .with_context(|| format!("invalid iroh relay url `{value}`"))
            })
            .collect()
    }

    pub fn relay_mode(&self) -> Result<RelayMode> {
        if self.iroh_relay_urls.is_empty() {
            return Ok(RelayMode::Disabled);
        }
        let relay_urls = self.parsed_relay_urls()?;
        Ok(RelayMode::Custom(RelayMap::from_iter(relay_urls)))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeedPeer {
    pub endpoint_id: String,
    pub addr_hint: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoverySnapshot {
    pub mode: DiscoveryMode,
    pub connect_mode: ConnectMode,
    pub env_locked: bool,
    pub configured_seed_peer_ids: Vec<String>,
    pub bootstrap_seed_peer_ids: Vec<String>,
    pub manual_ticket_peer_ids: Vec<String>,
    pub connected_peer_ids: Vec<String>,
    pub local_endpoint_id: String,
    pub last_discovery_error: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct DhtDiscoveryOptions {
    pub enabled: bool,
    pub client: Option<PkarrClient>,
}

impl DhtDiscoveryOptions {
    pub fn disabled() -> Self {
        Self::default()
    }

    pub fn seeded_dht() -> Self {
        Self {
            enabled: true,
            client: None,
        }
    }

    pub fn with_client(client: PkarrClient) -> Self {
        Self {
            enabled: true,
            client: Some(client),
        }
    }

    pub(crate) fn publish_client(&self) -> Result<Option<PkarrClient>> {
        if !self.enabled {
            return Ok(None);
        }
        if let Some(client) = self.client.as_ref() {
            return Ok(Some(client.clone()));
        }
        let mut builder = PkarrClient::builder();
        builder.no_default_network();
        builder.cache_size(0);
        builder.dht(|dht| dht);
        Ok(Some(builder.build().context(
            "failed to build pkarr client for endpoint publication",
        )?))
    }
}

impl Default for TransportNetworkConfig {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)),
            advertised_host: None,
            advertised_port: None,
        }
    }
}

impl TransportNetworkConfig {
    pub fn loopback() -> Self {
        Self {
            bind_addr: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)),
            advertised_host: None,
            advertised_port: None,
        }
    }

    pub fn from_env() -> Result<Self> {
        let bind_addr = std::env::var("KUKURI_BIND_ADDR")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(|value| SocketAddr::from_str(value.trim()))
            .transpose()
            .context("failed to parse KUKURI_BIND_ADDR")?
            .unwrap_or_else(|| SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)));
        let advertised_host = std::env::var("KUKURI_ADVERTISE_HOST")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let advertised_port = std::env::var("KUKURI_ADVERTISE_PORT")
            .ok()
            .map(|value| value.trim().parse::<u16>())
            .transpose()
            .context("failed to parse KUKURI_ADVERTISE_PORT")?;

        Ok(Self {
            bind_addr,
            advertised_host,
            advertised_port,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock")
    }

    fn legacy_env(name: &str) -> String {
        format!("KUKURI_{}_{}", "NEXT", name)
    }

    #[test]
    fn old_next_env_vars_are_not_used() {
        let _guard = env_lock();
        let legacy_bind_addr = legacy_env("BIND_ADDR");
        let legacy_advertise_host = legacy_env("ADVERTISE_HOST");
        let legacy_advertise_port = legacy_env("ADVERTISE_PORT");
        for key in [
            "KUKURI_BIND_ADDR",
            "KUKURI_ADVERTISE_HOST",
            "KUKURI_ADVERTISE_PORT",
            legacy_bind_addr.as_str(),
            legacy_advertise_host.as_str(),
            legacy_advertise_port.as_str(),
        ] {
            unsafe { std::env::remove_var(key) };
        }
        unsafe {
            std::env::set_var(legacy_bind_addr, "127.0.0.1:40123");
            std::env::set_var(legacy_advertise_host, "legacy-host");
            std::env::set_var(legacy_advertise_port, "40123");
        }

        let config = TransportNetworkConfig::from_env().expect("config");

        assert_eq!(
            config.bind_addr,
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))
        );
        assert_eq!(config.advertised_host, None);
        assert_eq!(config.advertised_port, None);
    }

    #[test]
    fn dht_publish_client_disables_pkarr_cache() {
        let client = DhtDiscoveryOptions::seeded_dht()
            .publish_client()
            .expect("publish client")
            .expect("pkarr client");

        assert!(client.cache().is_none());
    }
}
