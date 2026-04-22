use super::*;

impl IrohGossipTransport {
    pub(crate) async fn insert_imported_peer_addr(&self, endpoint_addr: EndpointAddr) {
        self.discovery.add_endpoint_info(endpoint_addr.clone());
        self.imported_peers
            .lock()
            .await
            .insert(endpoint_addr.id.to_string(), endpoint_addr.clone());
        self.extend_active_topic_peers(vec![endpoint_addr], "imported-peer")
            .await;
    }

    pub(crate) async fn transport_import_ticket_impl(&self, ticket: &str) -> Result<()> {
        let endpoint_addr = match parse_endpoint_ticket(ticket) {
            Ok(endpoint_addr) => endpoint_addr,
            Err(error) => {
                let message = format!("failed to import peer ticket: {error}");
                *self.last_error.lock().await = Some(message.clone());
                return Err(anyhow!(message));
            }
        };
        self.insert_imported_peer_addr(endpoint_addr).await;
        *self.last_error.lock().await = None;
        Ok(())
    }

    pub(crate) async fn transport_configure_discovery_impl(
        &self,
        mode: DiscoveryMode,
        env_locked: bool,
        configured_seed_peers: Vec<SeedPeer>,
        bootstrap_seed_peers: Vec<SeedPeer>,
    ) -> Result<()> {
        let relay_urls = self
            .relay_urls
            .read()
            .expect("transport relay urls poisoned")
            .clone();
        if !relay_urls.is_empty() {
            let endpoint = self.endpoint.clone();
            tokio::spawn(async move {
                endpoint.online().await;
            });
        }
        let mut configured = BTreeMap::new();
        for seed in configured_seed_peers {
            let endpoint_addr = seed.to_endpoint_addr_with_relays(&relay_urls)?;
            self.discovery.add_endpoint_info(endpoint_addr.clone());
            configured.insert(endpoint_addr.id.to_string(), endpoint_addr);
        }
        let mut bootstrap = BTreeMap::new();
        for seed in bootstrap_seed_peers {
            let endpoint_addr = seed.to_endpoint_addr_with_relays(&relay_urls)?;
            self.discovery.add_endpoint_info(endpoint_addr.clone());
            bootstrap.insert(endpoint_addr.id.to_string(), endpoint_addr);
        }
        let updated_topic_peers = configured
            .values()
            .chain(bootstrap.values())
            .cloned()
            .collect::<Vec<_>>();
        *self.discovery_mode.lock().await = mode;
        *self.env_locked.lock().await = env_locked;
        *self.configured_seed_peers.lock().await = configured;
        *self.bootstrap_seed_peers.lock().await = bootstrap;
        *self.last_error.lock().await = None;
        self.extend_active_topic_peers(updated_topic_peers, "seed-update")
            .await;
        Ok(())
    }

    pub(crate) async fn transport_discovery_impl(&self) -> Result<DiscoverySnapshot> {
        let configured_seed_peer_ids = self.configured_seed_peer_ids().await;
        let bootstrap_seed_peer_ids = self.bootstrap_seed_peer_ids().await;
        let manual_ticket_peer_ids = self
            .imported_peers
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        Ok(DiscoverySnapshot {
            mode: self.discovery_mode.lock().await.clone(),
            connect_mode: self.connect_mode.lock().await.clone(),
            env_locked: *self.env_locked.lock().await,
            configured_seed_peer_ids,
            bootstrap_seed_peer_ids,
            manual_ticket_peer_ids,
            connected_peer_ids: self.connected_peer_ids().await,
            local_endpoint_id: self.endpoint.id().to_string(),
            last_discovery_error: self.last_error.lock().await.clone(),
        })
    }
}
