use super::*;

impl IrohGossipTransport {
    pub async fn peer_state(&self) -> TransportPeerState {
        TransportPeerState {
            imported_peers: self.imported_peers.lock().await.values().cloned().collect(),
        }
    }

    pub async fn restore_peer_state(&self, state: TransportPeerState) -> Result<()> {
        for endpoint_addr in state.imported_peers {
            self.insert_imported_peer_addr(endpoint_addr).await;
        }
        *self.last_error.lock().await = None;
        Ok(())
    }

    pub(crate) async fn bootstrap_peers(&self) -> Vec<EndpointAddr> {
        let mut peers = self
            .configured_seed_peers
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for peer in self.bootstrap_seed_peers.lock().await.values() {
            if !peers.iter().any(|existing| existing.id == peer.id) {
                peers.push(peer.clone());
            }
        }
        for peer in self.imported_peers.lock().await.values() {
            if !peers.iter().any(|existing| existing.id == peer.id) {
                peers.push(peer.clone());
            }
        }
        peers
    }

    pub(crate) async fn configured_seed_peer_ids(&self) -> Vec<String> {
        self.configured_seed_peers
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>()
    }

    pub(crate) async fn bootstrap_seed_peer_ids(&self) -> Vec<String> {
        self.bootstrap_seed_peers
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>()
    }

    async fn configured_peer_ids(&self) -> Vec<String> {
        self.bootstrap_peers()
            .await
            .into_iter()
            .map(|peer| peer.id.to_string())
            .collect::<Vec<_>>()
    }

    pub(crate) async fn connected_peer_ids(&self) -> Vec<String> {
        let mut connected = BTreeSet::new();
        for (_, state) in self.topic_states.lock().await.iter() {
            for peer in state.neighbors.read().await.iter() {
                connected.insert(peer.clone());
            }
        }
        connected.into_iter().collect::<Vec<_>>()
    }

    pub(crate) async fn transport_peers_impl(&self) -> Result<PeerSnapshot> {
        let topic_states = self
            .topic_states
            .lock()
            .await
            .iter()
            .map(|(topic, state)| {
                (
                    topic.clone(),
                    state.bootstrap_peer_ids.iter().cloned().collect::<Vec<_>>(),
                    Arc::clone(&state.neighbors),
                    Arc::clone(&state.last_received_at),
                    Arc::clone(&state.last_error),
                )
            })
            .collect::<Vec<_>>();
        let mut connected = BTreeSet::new();
        let configured_peers = self.configured_peer_ids().await;
        let mut topic_diagnostics = Vec::with_capacity(topic_states.len());
        for (topic, configured_peer_ids, neighbors, last_received_at, last_error) in topic_states {
            let peers = neighbors.read().await.iter().cloned().collect::<Vec<_>>();
            let last_received_at = *last_received_at.lock().await;
            let last_error = last_error.lock().await.clone();
            for peer in &peers {
                connected.insert(peer.clone());
            }
            let configured_peer_count = configured_peer_ids.len();
            let connected_peer_count = peers.len();
            let missing_peer_ids = configured_peer_ids
                .iter()
                .filter(|peer| !peers.iter().any(|connected_peer| connected_peer == *peer))
                .cloned()
                .collect::<Vec<_>>();
            topic_diagnostics.push(TopicPeerSnapshot {
                topic,
                joined: !peers.is_empty(),
                peer_count: connected_peer_count,
                connected_peers: peers,
                configured_peer_ids,
                missing_peer_ids,
                last_received_at,
                status_detail: topic_status_detail(configured_peer_count, connected_peer_count),
                last_error,
            });
        }
        topic_diagnostics.sort_by(|left, right| left.topic.cmp(&right.topic));
        let subscribed_topics = self
            .subscribed_topics
            .lock()
            .await
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let connected_peers = connected.into_iter().collect::<Vec<_>>();
        let configured_peer_count = configured_peers.len();
        let connected_peer_count = connected_peers.len();
        let subscribed_topic_count = topic_diagnostics.len();

        Ok(PeerSnapshot {
            connected: !connected_peers.is_empty(),
            peer_count: connected_peer_count,
            connected_peers,
            configured_peers,
            subscribed_topics,
            pending_events: 0,
            status_detail: peer_status_detail(
                configured_peer_count,
                connected_peer_count,
                subscribed_topic_count,
            ),
            last_error: self.last_error.lock().await.clone(),
            topic_diagnostics,
        })
    }

    pub(crate) async fn transport_export_ticket_impl(&self) -> Result<Option<String>> {
        let endpoint_addr = self.endpoint.addr();
        let ticket_config = ticket_network_config(
            &endpoint_addr,
            &self.endpoint.bound_sockets(),
            &self.network_config,
        );
        Ok(Some(encode_endpoint_ticket(
            &endpoint_addr,
            &ticket_config,
        )?))
    }
}
