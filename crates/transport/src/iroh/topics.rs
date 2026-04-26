use super::*;

pub(crate) fn initial_topic_join_timeout() -> Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        Duration::from_secs(180)
    } else {
        Duration::from_secs(15)
    }
}

fn topic_warmup_retry_delay(attempt: usize, relay_backed: bool) -> Duration {
    if relay_backed {
        return relay_topic_warmup_retry_delay(attempt);
    }
    direct_topic_warmup_retry_delay(attempt)
}

fn direct_topic_warmup_retry_delay(attempt: usize) -> Duration {
    match attempt {
        0 => Duration::from_millis(250),
        1 => Duration::from_millis(500),
        2 => Duration::from_secs(1),
        3 => Duration::from_secs(2),
        _ => Duration::from_secs(5),
    }
}

fn relay_topic_warmup_retry_delay(attempt: usize) -> Duration {
    match attempt {
        0 => Duration::from_secs(1),
        1 => Duration::from_secs(2),
        2 => Duration::from_secs(4),
        3 => Duration::from_secs(8),
        _ => Duration::from_secs(10),
    }
}

fn peers_use_relay(peers: &[EndpointAddr]) -> bool {
    peers.iter().any(|peer| peer.relay_urls().next().is_some())
}

fn direct_warmup_addr(endpoint_addr: &EndpointAddr) -> EndpointAddr {
    endpoint_addr.clone()
}
pub(crate) fn topic_to_gossip_id(topic: &TopicId) -> GossipTopicId {
    let hash = blake3::hash(topic.as_str().as_bytes());
    GossipTopicId::from_bytes(*hash.as_bytes())
}

async fn endpoint_has_active_remote_addr(endpoint: &Endpoint, endpoint_id: EndpointId) -> bool {
    endpoint.remote_info(endpoint_id).await.is_some_and(|info| {
        info.addrs()
            .any(|addr| matches!(addr.usage(), TransportAddrUsage::Active))
    })
}

impl TopicWarmupCoordinator {
    async fn warmup_peers_once(
        &self,
        endpoint: &Endpoint,
        gossip: &Gossip,
        peers: &[EndpointAddr],
    ) {
        let mut tasks = Vec::new();
        for peer in peers.iter().cloned() {
            let coordinator = self.clone();
            let endpoint = endpoint.clone();
            let gossip = gossip.clone();
            tasks.push(tokio::spawn(async move {
                coordinator.warmup_peer(endpoint, gossip, peer).await;
            }));
        }
        for task in tasks {
            let _ = task.await;
        }
    }

    async fn warmup_peer(&self, endpoint: Endpoint, gossip: Gossip, peer: EndpointAddr) {
        let peer_key = peer.id.to_string();
        let Some(_in_flight_guard) = self.try_mark_peer_in_flight(peer_key) else {
            return;
        };

        let Ok(_permit) = self.permits.acquire().await else {
            return;
        };
        if endpoint_has_active_remote_addr(&endpoint, peer.id).await {
            return;
        }

        let warmup_addr = direct_warmup_addr(&peer);
        if let Ok(connection) = endpoint.connect(warmup_addr, GOSSIP_ALPN).await {
            let _ = gossip.handle_connection(connection).await;
        }
    }

    fn try_mark_peer_in_flight(&self, peer_key: String) -> Option<TopicWarmupInFlightGuard> {
        let mut in_flight_peers = self
            .in_flight_peers
            .write()
            .expect("topic warmup in-flight lock poisoned");
        if !in_flight_peers.insert(peer_key.clone()) {
            return None;
        }
        Some(TopicWarmupInFlightGuard {
            peer_key,
            in_flight_peers: Arc::clone(&self.in_flight_peers),
        })
    }

    #[cfg(test)]
    fn try_mark_peer_in_flight_for_test(&self, peer_key: &str) -> bool {
        self.in_flight_peers
            .write()
            .expect("topic warmup in-flight lock poisoned")
            .insert(peer_key.to_string())
    }

    #[cfg(test)]
    fn clear_in_flight_for_test(&self, peer_key: &str) {
        self.in_flight_peers
            .write()
            .expect("topic warmup in-flight lock poisoned")
            .remove(peer_key);
    }
}

struct TopicWarmupInFlightGuard {
    peer_key: String,
    in_flight_peers: Arc<StdRwLock<BTreeSet<String>>>,
}

impl Drop for TopicWarmupInFlightGuard {
    fn drop(&mut self) {
        if let Ok(mut in_flight_peers) = self.in_flight_peers.write() {
            in_flight_peers.remove(&self.peer_key);
        }
    }
}

impl IrohGossipTransport {
    async fn remove_topic_state(&self, topic: &str) {
        if let Some(state) = self.topic_states.lock().await.remove(topic) {
            state._receiver_task.abort();
            drop(state.sender);
        }
        self.subscribed_topics.lock().await.remove(topic);
    }

    pub(crate) async fn extend_active_topic_peers(
        &self,
        endpoint_addrs: Vec<EndpointAddr>,
        reason: &str,
    ) {
        if endpoint_addrs.is_empty() {
            return;
        }
        let mut updates = Vec::new();
        {
            let mut topic_states = self.topic_states.lock().await;
            for (topic, state) in topic_states.iter_mut() {
                let mut join_peer_ids = Vec::new();
                let mut added_peer_ids = Vec::new();
                let mut join_endpoint_addrs = Vec::new();
                for endpoint_addr in &endpoint_addrs {
                    let peer_id = endpoint_addr.id.to_string();
                    if state.bootstrap_peer_ids.insert(peer_id.clone()) {
                        join_peer_ids.push(endpoint_addr.id);
                        added_peer_ids.push(peer_id);
                        join_endpoint_addrs.push(endpoint_addr.clone());
                    }
                }
                if !join_peer_ids.is_empty() {
                    updates.push((
                        topic.clone(),
                        state.sender.clone(),
                        Arc::clone(&state.neighbors),
                        added_peer_ids,
                        join_peer_ids,
                        join_endpoint_addrs,
                    ));
                }
            }
        }

        for (topic, sender, neighbors, added_peer_ids, join_peer_ids, join_endpoint_addrs) in
            updates
        {
            info!(
                topic = %topic,
                reason,
                added_peer_ids = ?added_peer_ids,
                "updating active gossip topic peers"
            );
            if let Err(error) = sender.lock().await.join_peers(join_peer_ids).await {
                warn!(
                    topic = %topic,
                    reason,
                    added_peer_ids = ?added_peer_ids,
                    error = %error,
                    "failed to join updated peers on active gossip topic"
                );
            }

            let endpoint = self.endpoint.clone();
            let gossip = self.gossip.clone();
            let warmups = Arc::clone(&self.topic_warmups);
            tokio::spawn(async move {
                let join_deadline = tokio::time::Instant::now() + initial_topic_join_timeout();
                let relay_backed = peers_use_relay(&join_endpoint_addrs);
                let mut attempt = 0usize;
                loop {
                    let already_connected = {
                        let guard = neighbors.read().await;
                        join_endpoint_addrs
                            .iter()
                            .any(|peer| guard.contains(&peer.id.to_string()))
                    };
                    if already_connected || tokio::time::Instant::now() >= join_deadline {
                        return;
                    }
                    warmups
                        .warmup_peers_once(&endpoint, &gossip, &join_endpoint_addrs)
                        .await;
                    if tokio::time::Instant::now() >= join_deadline {
                        return;
                    }
                    let retry_delay = topic_warmup_retry_delay(attempt, relay_backed);
                    attempt = attempt.saturating_add(1);
                    sleep(retry_delay).await;
                }
            });
        }
    }

    async fn ensure_hint_topic(&self, topic: &TopicId) -> Result<broadcast::Sender<HintEnvelope>> {
        let bootstrap_peers = self.bootstrap_peers().await;
        let bootstrap_peer_ids = bootstrap_peers
            .iter()
            .map(|peer| peer.id.to_string())
            .collect::<BTreeSet<_>>();

        let existing = {
            let topics = self.topic_states.lock().await;
            topics.get(topic.as_str()).map(|state| {
                (
                    state.broadcaster.clone(),
                    state.bootstrap_peer_ids.clone(),
                    Arc::clone(&state.neighbors),
                    Arc::clone(&state.last_error),
                )
            })
        };

        if let Some((broadcaster, existing_bootstrap_peer_ids, neighbors, last_error)) = existing {
            let has_neighbors = !neighbors.read().await.is_empty();
            let timed_out_join = last_error
                .lock()
                .await
                .as_deref()
                .is_some_and(|message| message.contains("initial topic join"));
            if existing_bootstrap_peer_ids == bootstrap_peer_ids
                && (!timed_out_join || has_neighbors)
            {
                self.subscribed_topics.lock().await.insert(topic.0.clone());
                return Ok(broadcaster);
            }
            self.remove_topic_state(topic.as_str()).await;
        }

        let bootstrap = bootstrap_peers
            .iter()
            .map(|peer| peer.id)
            .collect::<Vec<_>>();

        for peer in &bootstrap_peers {
            self.discovery.add_endpoint_info(peer.clone());
        }

        let topic_handle = match self
            .gossip
            .subscribe(topic_to_gossip_id(topic), bootstrap)
            .await
        {
            Ok(topic_handle) => topic_handle,
            Err(error) => {
                let message = format!("failed to subscribe gossip topic: {error}");
                *self.last_error.lock().await = Some(message.clone());
                return Err(anyhow!(message));
            }
        };
        let (sender, mut receiver) = topic_handle.split();
        let (broadcaster, _) = broadcast::channel(256);
        let outbound = broadcaster.clone();
        let topic_name = topic.as_str().to_string();
        let joined = Arc::new(AtomicBool::new(bootstrap_peers.is_empty()));
        let joined_notify = Arc::new(Notify::new());
        let joined_task_state = Arc::clone(&joined);
        let joined_task_notify = Arc::clone(&joined_notify);
        let neighbors = Arc::new(RwLock::new(BTreeSet::new()));
        let neighbors_task = Arc::clone(&neighbors);
        let last_received_at = Arc::new(Mutex::new(None));
        let last_received_at_task = Arc::clone(&last_received_at);
        let last_error = Arc::new(Mutex::new(None));
        let last_error_task = Arc::clone(&last_error);
        let transport_last_error = Arc::clone(&self.last_error);
        let imported_count = bootstrap_peers.len();
        let warm_endpoint = self.endpoint.clone();
        let warm_bootstrap_peers = bootstrap_peers.clone();
        let warm_gossip = self.gossip.clone();
        let warmups = Arc::clone(&self.topic_warmups);

        let task = tokio::spawn(async move {
            if imported_count > 0 {
                let join_timeout = initial_topic_join_timeout();
                let warmup_task = tokio::spawn(async move {
                    let join_deadline = tokio::time::Instant::now() + join_timeout;
                    let relay_backed = peers_use_relay(&warm_bootstrap_peers);
                    let mut attempt = 0usize;
                    loop {
                        warmups
                            .warmup_peers_once(&warm_endpoint, &warm_gossip, &warm_bootstrap_peers)
                            .await;
                        if tokio::time::Instant::now() >= join_deadline {
                            return;
                        }
                        let retry_delay = topic_warmup_retry_delay(attempt, relay_backed);
                        attempt = attempt.saturating_add(1);
                        sleep(retry_delay).await;
                    }
                });
                let joined = timeout(join_timeout, receiver.joined())
                    .await
                    .is_ok_and(|result| result.is_ok());
                warmup_task.abort();
                if joined {
                    joined_task_state.store(true, Ordering::SeqCst);
                    joined_task_notify.notify_waiters();
                    *last_error_task.lock().await = None;
                    *transport_last_error.lock().await = None;
                    let current_neighbors = receiver
                        .neighbors()
                        .map(|peer| peer.to_string())
                        .collect::<BTreeSet<_>>();
                    *neighbors_task.write().await = current_neighbors;
                } else {
                    let message = "timed out waiting for initial topic join".to_string();
                    *last_error_task.lock().await = Some(message.clone());
                    *transport_last_error.lock().await =
                        Some(format!("topic join pending: {message}"));
                }
            }
            while let Some(event) = receiver.next().await {
                match event {
                    Ok(GossipEvent::Received(message)) => {
                        joined_task_state.store(true, Ordering::SeqCst);
                        joined_task_notify.notify_waiters();
                        let current_neighbors = receiver
                            .neighbors()
                            .map(|peer| peer.to_string())
                            .collect::<BTreeSet<_>>();
                        *neighbors_task.write().await = current_neighbors;
                        *last_received_at_task.lock().await = Some(Utc::now().timestamp_millis());
                        if let Ok(parsed) = serde_json::from_slice::<GossipHint>(&message.content) {
                            *last_error_task.lock().await = None;
                            *transport_last_error.lock().await = None;
                            let _ = outbound.send(HintEnvelope {
                                hint: parsed,
                                received_at: Utc::now().timestamp_millis(),
                                source_peer: message.delivered_from.to_string(),
                            });
                        } else {
                            *last_error_task.lock().await =
                                Some("failed to decode hint payload".to_string());
                        }
                    }
                    Ok(GossipEvent::NeighborUp(peer_id)) => {
                        joined_task_state.store(true, Ordering::SeqCst);
                        joined_task_notify.notify_waiters();
                        let mut guard = neighbors_task.write().await;
                        let first_direct_peer = guard.is_empty();
                        guard.insert(peer_id.to_string());
                        if first_direct_peer {
                            info!(
                                topic = %topic_name,
                                peer_id = %peer_id,
                                "gossip topic established direct peer"
                            );
                        }
                        *last_error_task.lock().await = None;
                        *transport_last_error.lock().await = None;
                    }
                    Ok(GossipEvent::NeighborDown(peer_id)) => {
                        let mut guard = neighbors_task.write().await;
                        guard.remove(peer_id.to_string().as_str());
                    }
                    Ok(GossipEvent::Lagged) => {}
                    Err(error) => {
                        let message = format!("gossip receiver closed: {error}");
                        *last_error_task.lock().await = Some(message.clone());
                        *transport_last_error.lock().await = Some(message);
                        break;
                    }
                }
            }
        });

        self.subscribed_topics.lock().await.insert(topic.0.clone());
        self.topic_states.lock().await.insert(
            topic.0.clone(),
            HintTopicState {
                sender: Arc::new(Mutex::new(sender)),
                broadcaster: broadcaster.clone(),
                bootstrap_peer_ids,
                neighbors,
                last_received_at,
                last_error,
                _receiver_task: task,
            },
        );

        Ok(broadcaster)
    }

    fn stream_from_sender(sender: &broadcast::Sender<HintEnvelope>) -> HintStream {
        let stream =
            BroadcastStream::new(sender.subscribe()).filter_map(|event| async move { event.ok() });
        Box::pin(stream)
    }

    pub async fn shutdown(&self) {
        let topics = self
            .subscribed_topics
            .lock()
            .await
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        for topic in topics {
            self.remove_topic_state(topic.as_str()).await;
        }
    }

    pub(crate) async fn hint_subscribe_hints_impl(&self, topic: &TopicId) -> Result<HintStream> {
        let hint_topic = TopicId::new(format!("hint/{}", topic.as_str()));
        let sender = self.ensure_hint_topic(&hint_topic).await?;
        Ok(Self::stream_from_sender(&sender))
    }

    pub(crate) async fn hint_unsubscribe_hints_impl(&self, topic: &TopicId) -> Result<()> {
        let hint_topic = TopicId::new(format!("hint/{}", topic.as_str()));
        self.remove_topic_state(hint_topic.as_str()).await;
        Ok(())
    }

    pub(crate) async fn hint_publish_hint_impl(
        &self,
        topic: &TopicId,
        hint: GossipHint,
    ) -> Result<()> {
        let hint_topic = TopicId::new(format!("hint/{}", topic.as_str()));
        let _ = self.ensure_hint_topic(&hint_topic).await?;
        let states = self.topic_states.lock().await;
        let state = states
            .get(hint_topic.as_str())
            .ok_or_else(|| anyhow!("missing hint topic sender"))?;
        let sender = state.sender.lock().await;
        let payload = serde_json::to_vec(&hint)?;
        if let Err(error) = sender.broadcast(payload.into()).await {
            let message = format!("failed to broadcast gossip hint: {error}");
            *state.last_error.lock().await = Some(message.clone());
            *self.last_error.lock().await = Some(message.clone());
            return Err(anyhow!(message));
        }
        *state.last_error.lock().await = None;
        *self.last_error.lock().await = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topic_warmup_retry_delay_backs_off_and_caps() {
        assert_eq!(
            topic_warmup_retry_delay(0, false),
            Duration::from_millis(250)
        );
        assert_eq!(
            topic_warmup_retry_delay(1, false),
            Duration::from_millis(500)
        );
        assert_eq!(topic_warmup_retry_delay(2, false), Duration::from_secs(1));
        assert_eq!(topic_warmup_retry_delay(3, false), Duration::from_secs(2));
        assert_eq!(topic_warmup_retry_delay(4, false), Duration::from_secs(5));
        assert_eq!(topic_warmup_retry_delay(12, false), Duration::from_secs(5));
    }

    #[test]
    fn relay_topic_warmup_retry_delay_backs_off_more_slowly() {
        assert_eq!(topic_warmup_retry_delay(0, true), Duration::from_secs(1));
        assert_eq!(topic_warmup_retry_delay(1, true), Duration::from_secs(2));
        assert_eq!(topic_warmup_retry_delay(2, true), Duration::from_secs(4));
        assert_eq!(topic_warmup_retry_delay(3, true), Duration::from_secs(8));
        assert_eq!(topic_warmup_retry_delay(4, true), Duration::from_secs(10));
        assert_eq!(topic_warmup_retry_delay(12, true), Duration::from_secs(10));
    }

    #[test]
    fn warmup_coordinator_coalesces_same_peer_dials() {
        let coordinator = TopicWarmupCoordinator::default();

        assert!(coordinator.try_mark_peer_in_flight_for_test("peer"));
        assert!(!coordinator.try_mark_peer_in_flight_for_test("peer"));

        coordinator.clear_in_flight_for_test("peer");
        assert!(coordinator.try_mark_peer_in_flight_for_test("peer"));
    }

    #[test]
    fn warmup_in_flight_guard_clears_on_drop() {
        let coordinator = TopicWarmupCoordinator::default();
        let guard = coordinator
            .try_mark_peer_in_flight("peer".to_string())
            .expect("first warmup marks peer");

        assert!(!coordinator.try_mark_peer_in_flight_for_test("peer"));
        drop(guard);
        assert!(coordinator.try_mark_peer_in_flight_for_test("peer"));
    }
}
