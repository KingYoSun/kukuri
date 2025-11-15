use crate::shared::config::NetworkConfig;
use iroh::discovery::{
    dns::DnsDiscovery,
    mdns::MdnsDiscovery,
    pkarr::{PkarrPublisher, dht::DhtDiscovery},
};

/// P2Pネットワークのディスカバリー設定
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiscoveryOptions {
    pub enable_dns: bool,
    pub enable_mainline: bool,
    pub enable_local: bool,
}

impl DiscoveryOptions {
    pub fn new(enable_dns: bool, enable_mainline: bool, enable_local: bool) -> Self {
        Self {
            enable_dns,
            enable_mainline,
            enable_local,
        }
    }

    pub fn with_mainline(mut self, enabled: bool) -> Self {
        self.enable_mainline = enabled;
        self
    }

    pub fn enable_mainline(&self) -> bool {
        self.enable_mainline
    }

    pub fn apply_to_builder(
        &self,
        mut builder: iroh::endpoint::Builder,
    ) -> iroh::endpoint::Builder {
        builder = builder.clear_discovery();

        if self.enable_dns {
            builder = builder.discovery(PkarrPublisher::n0_dns());
            builder = builder.discovery(DnsDiscovery::n0_dns());
        }
        if self.enable_mainline {
            builder = builder.discovery(
                DhtDiscovery::builder()
                    .include_direct_addresses(true)
                    .n0_dns_pkarr_relay(),
            );
        }
        if self.enable_local {
            builder = builder.discovery(MdnsDiscovery::builder());
        }
        builder
    }
}

impl Default for DiscoveryOptions {
    fn default() -> Self {
        Self {
            enable_dns: true,
            enable_mainline: true,
            enable_local: false,
        }
    }
}

impl From<&NetworkConfig> for DiscoveryOptions {
    fn from(cfg: &NetworkConfig) -> Self {
        Self {
            enable_dns: cfg.enable_dns,
            enable_mainline: cfg.enable_dht,
            enable_local: cfg.enable_local,
        }
    }
}
