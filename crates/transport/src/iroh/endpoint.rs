use super::*;

impl IrohGossipTransport {
    pub async fn bind(network_config: TransportNetworkConfig) -> Result<Self> {
        Self::bind_with_options(
            network_config,
            DhtDiscoveryOptions::disabled(),
            TransportRelayConfig::default(),
        )
        .await
    }

    pub async fn bind_with_options(
        network_config: TransportNetworkConfig,
        dht_options: DhtDiscoveryOptions,
        relay_config: TransportRelayConfig,
    ) -> Result<Self> {
        let relay_config = relay_config.normalized();
        let relay_urls = Arc::new(StdRwLock::new(relay_config.parsed_relay_urls()?));
        let (endpoint, discovery) = bind_endpoint_with_options(
            network_config.bind_addr,
            &dht_options,
            &relay_config,
            Arc::clone(&relay_urls),
            None,
        )
        .await?;
        Ok(Self::spawn_gossip_transport(
            endpoint,
            discovery,
            network_config,
            &relay_config,
            relay_urls,
        ))
    }

    /// relay-only endpoint(IP transport なし)で bind するテスト専用コンストラクタ。
    /// loopback では直接 addr が holepunch で有効化されるため、実データが relay 経由に
    /// なる構成はこの knob でしか安定して再現できない。
    #[cfg(test)]
    pub(crate) async fn bind_relay_only_for_tests(
        network_config: TransportNetworkConfig,
        relay_config: TransportRelayConfig,
    ) -> Result<Self> {
        let relay_config = relay_config.normalized();
        let relay_urls = Arc::new(StdRwLock::new(relay_config.parsed_relay_urls()?));
        let (endpoint, discovery) =
            bind_endpoint_relay_only(&relay_config, Arc::clone(&relay_urls)).await?;
        Ok(Self::spawn_gossip_transport(
            endpoint,
            discovery,
            network_config,
            &relay_config,
            relay_urls,
        ))
    }

    fn spawn_gossip_transport(
        endpoint: Endpoint,
        discovery: Arc<MemoryLookup>,
        network_config: TransportNetworkConfig,
        relay_config: &TransportRelayConfig,
        relay_urls: Arc<StdRwLock<Vec<RelayUrl>>>,
    ) -> Self {
        let gossip = Gossip::builder().spawn(endpoint.clone());
        let router = Router::builder(endpoint.clone())
            .accept(GOSSIP_ALPN, gossip.clone())
            .spawn();

        Self {
            endpoint,
            gossip,
            _router: Some(router),
            discovery,
            network_config,
            configured_seed_peers: Arc::new(Mutex::new(BTreeMap::new())),
            bootstrap_seed_peers: Arc::new(Mutex::new(BTreeMap::new())),
            imported_peers: Arc::new(Mutex::new(BTreeMap::new())),
            subscribed_topics: Arc::new(Mutex::new(BTreeSet::new())),
            topic_states: Arc::new(Mutex::new(HashMap::new())),
            topic_warmups: Arc::new(TopicWarmupCoordinator::default()),
            last_error: Arc::new(Mutex::new(None)),
            discovery_mode: Arc::new(Mutex::new(DiscoveryMode::StaticPeer)),
            connect_mode: Arc::new(Mutex::new(relay_config.connect_mode())),
            relay_urls,
            env_locked: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn bind_with_discovery(
        network_config: TransportNetworkConfig,
        dht_options: DhtDiscoveryOptions,
    ) -> Result<Self> {
        Self::bind_with_options(network_config, dht_options, TransportRelayConfig::default()).await
    }

    pub fn from_shared_parts(
        endpoint: Endpoint,
        gossip: Gossip,
        discovery: Arc<MemoryLookup>,
        network_config: TransportNetworkConfig,
        relay_config: TransportRelayConfig,
    ) -> Result<Self> {
        let relay_config = relay_config.normalized();
        let relay_urls = Arc::new(StdRwLock::new(relay_config.parsed_relay_urls()?));
        discovery.add_endpoint_info(endpoint.addr());
        Ok(Self {
            endpoint,
            gossip,
            _router: None,
            discovery,
            network_config,
            configured_seed_peers: Arc::new(Mutex::new(BTreeMap::new())),
            bootstrap_seed_peers: Arc::new(Mutex::new(BTreeMap::new())),
            imported_peers: Arc::new(Mutex::new(BTreeMap::new())),
            subscribed_topics: Arc::new(Mutex::new(BTreeSet::new())),
            topic_states: Arc::new(Mutex::new(HashMap::new())),
            topic_warmups: Arc::new(TopicWarmupCoordinator::default()),
            last_error: Arc::new(Mutex::new(None)),
            discovery_mode: Arc::new(Mutex::new(DiscoveryMode::StaticPeer)),
            connect_mode: Arc::new(Mutex::new(relay_config.connect_mode())),
            relay_urls,
            env_locked: Arc::new(Mutex::new(false)),
        })
    }

    pub async fn bind_local() -> Result<Self> {
        Self::bind(TransportNetworkConfig::loopback()).await
    }
}

pub(crate) async fn bind_endpoint_with_options(
    bind_addr: SocketAddr,
    dht_options: &DhtDiscoveryOptions,
    relay_config: &TransportRelayConfig,
    relay_urls: Arc<StdRwLock<Vec<RelayUrl>>>,
    secret_key: Option<SecretKey>,
) -> Result<(Endpoint, Arc<MemoryLookup>)> {
    let discovery = Arc::new(MemoryLookup::new());
    let mut builder = build_endpoint_builder(
        EndpointBuilder::new(presets::Minimal).relay_mode(relay_config.relay_mode()?),
        &discovery,
        Some(dht_options),
        relay_urls,
    )?;
    if let Some(secret_key) = secret_key {
        builder = builder.secret_key(secret_key);
    }
    #[cfg(test)]
    {
        builder = builder.ca_tls_config(CaTlsConfig::insecure_skip_verify());
    }
    builder = apply_bind(builder, bind_addr)?;
    let endpoint = builder
        .bind()
        .await
        .context("failed to bind iroh endpoint")?;
    prepare_endpoint_for_discovery(&endpoint, &discovery, relay_config).await?;
    Ok((endpoint, discovery))
}
/// relay-only endpoint bind(テスト専用)。IP transport を除去し、実データを relay 経由に強制する。
/// iroh 本家の relay-only テストと同じ `relay_mode(Custom) + clear_ip_transports` パターン。
#[cfg(test)]
async fn bind_endpoint_relay_only(
    relay_config: &TransportRelayConfig,
    relay_urls: Arc<StdRwLock<Vec<RelayUrl>>>,
) -> Result<(Endpoint, Arc<MemoryLookup>)> {
    let discovery = Arc::new(MemoryLookup::new());
    let mut builder = build_endpoint_builder(
        EndpointBuilder::new(presets::Minimal).relay_mode(relay_config.relay_mode()?),
        &discovery,
        Some(&DhtDiscoveryOptions::disabled()),
        relay_urls,
    )?;
    builder = builder.ca_tls_config(CaTlsConfig::insecure_skip_verify());
    builder = builder.clear_ip_transports();
    let endpoint = builder
        .bind()
        .await
        .context("failed to bind relay-only iroh endpoint")?;
    prepare_endpoint_for_discovery(&endpoint, &discovery, relay_config).await?;
    Ok((endpoint, discovery))
}

fn apply_bind(builder: EndpointBuilder, bind_addr: SocketAddr) -> Result<EndpointBuilder> {
    match bind_addr {
        SocketAddr::V4(addr) => builder
            .bind_addr(addr)
            .map_err(|error| anyhow!("failed to bind IPv4 address: {error}")),
        SocketAddr::V6(addr) => builder
            .bind_addr(addr)
            .map_err(|error| anyhow!("failed to bind IPv6 address: {error}")),
    }
}
