use super::*;

pub(crate) fn initial_topic_join_timeout() -> Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        Duration::from_secs(180)
    } else {
        Duration::from_secs(15)
    }
}
fn direct_warmup_addr(endpoint_addr: &EndpointAddr) -> EndpointAddr {
    if endpoint_addr.relay_urls().next().is_some() {
        return endpoint_addr.clone();
    }
    let direct_addrs = endpoint_addr
        .ip_addrs()
        .copied()
        .map(TransportAddr::Ip)
        .collect::<Vec<_>>();
    if direct_addrs.is_empty() {
        endpoint_addr.clone()
    } else {
        EndpointAddr::from_parts(endpoint_addr.id, direct_addrs)
    }
}
pub(crate) fn topic_to_gossip_id(topic: &TopicId) -> GossipTopicId {
    let hash = blake3::hash(topic.as_str().as_bytes());
    GossipTopicId::from_bytes(*hash.as_bytes())
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
            tokio::spawn(async move {
                let join_deadline = tokio::time::Instant::now() + initial_topic_join_timeout();
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
                    for peer in &join_endpoint_addrs {
                        let warmup_addr = direct_warmup_addr(peer);
                        if let Ok(connection) = endpoint.connect(warmup_addr, GOSSIP_ALPN).await {
                            let _ = gossip.handle_connection(connection).await;
                        }
                    }
                    sleep(Duration::from_millis(100)).await;
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

        let task = tokio::spawn(async move {
            if imported_count > 0 {
                let join_timeout = initial_topic_join_timeout();
                let warmup_task = tokio::spawn(async move {
                    let join_deadline = tokio::time::Instant::now() + join_timeout;
                    loop {
                        for peer in &warm_bootstrap_peers {
                            let warmup_addr = direct_warmup_addr(peer);
                            if let Ok(connection) =
                                warm_endpoint.connect(warmup_addr, GOSSIP_ALPN).await
                            {
                                let _ = warm_gossip.handle_connection(connection).await;
                            }
                        }
                        if tokio::time::Instant::now() >= join_deadline {
                            return;
                        }
                        sleep(Duration::from_millis(100)).await;
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
