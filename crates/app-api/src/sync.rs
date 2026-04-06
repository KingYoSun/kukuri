use crate::service::*;

impl AppService {
    pub async fn get_sync_status(&self) -> Result<SyncStatus> {
        let PeerSnapshot {
            connected,
            peer_count,
            connected_peers,
            configured_peers,
            subscribed_topics,
            pending_events,
            status_detail,
            last_error,
            topic_diagnostics,
        } = self.transport.peers().await?;
        let subscribed_topics = normalize_topics(subscribed_topics);
        let topic_diagnostics = normalize_topic_diagnostics(topic_diagnostics);
        let assist_peer_ids = self.assisted_peer_ids().await?;
        let effective_connected_peer_ids =
            merge_peer_ids(connected_peers.clone(), assist_peer_ids.clone());
        let discovery = self.get_discovery_status().await?;

        Ok(SyncStatus {
            connected: connected || !assist_peer_ids.is_empty(),
            last_sync_ts: *self.last_sync_ts.lock().await,
            peer_count: peer_count.max(effective_connected_peer_ids.len()),
            pending_events,
            status_detail: effective_sync_status_detail(
                status_detail.as_str(),
                connected_peers.len(),
                assist_peer_ids.len(),
                subscribed_topics.len(),
            ),
            last_error,
            configured_peers,
            subscribed_topics,
            topic_diagnostics: topic_diagnostics
                .into_iter()
                .map(|diagnostic| {
                    let gossip_peer_count = diagnostic.connected_peers.len();
                    TopicSyncStatus {
                        topic: diagnostic.topic,
                        joined: diagnostic.joined || !assist_peer_ids.is_empty(),
                        peer_count: diagnostic.peer_count.max(
                            merge_peer_ids(
                                diagnostic.connected_peers.clone(),
                                assist_peer_ids.clone(),
                            )
                            .len(),
                        ),
                        connected_peers: diagnostic.connected_peers,
                        assist_peer_ids: assist_peer_ids.clone(),
                        configured_peer_ids: diagnostic.configured_peer_ids,
                        missing_peer_ids: diagnostic.missing_peer_ids,
                        last_received_at: diagnostic.last_received_at,
                        status_detail: effective_topic_status_detail(
                            diagnostic.status_detail.as_str(),
                            gossip_peer_count,
                            assist_peer_ids.len(),
                        ),
                        last_error: diagnostic.last_error,
                    }
                })
                .collect(),
            local_author_pubkey: self.current_author_pubkey(),
            discovery,
        })
    }

    pub async fn get_discovery_status(&self) -> Result<DiscoveryStatus> {
        let DiscoverySnapshot {
            mode,
            connect_mode,
            env_locked,
            configured_seed_peer_ids,
            bootstrap_seed_peer_ids,
            manual_ticket_peer_ids,
            connected_peer_ids,
            local_endpoint_id,
            last_discovery_error,
        } = self.transport.discovery().await?;
        let assist_peer_ids = self.assisted_peer_ids().await?;
        Ok(DiscoveryStatus {
            mode,
            connect_mode,
            env_locked,
            configured_seed_peer_ids,
            bootstrap_seed_peer_ids,
            manual_ticket_peer_ids,
            connected_peer_ids,
            assist_peer_ids,
            local_endpoint_id,
            last_discovery_error,
        })
    }

    pub async fn import_peer_ticket(&self, ticket: &str) -> Result<()> {
        self.transport.import_ticket(ticket).await?;
        self.docs_sync.import_peer_ticket(ticket).await?;
        self.blob_service.import_peer_ticket(ticket).await?;
        let existing_topics = self
            .subscriptions
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for topic in existing_topics {
            self.restart_topic_subscription(topic.as_str()).await?;
        }
        let existing_private_topics = self
            .joined_private_channels
            .lock()
            .await
            .values()
            .map(|state| {
                (
                    state.topic_id.clone(),
                    state.channel_id.as_str().to_string(),
                )
            })
            .collect::<Vec<_>>();
        for (topic_id, channel_id) in existing_private_topics {
            self.restart_private_channel_subscription(topic_id.as_str(), channel_id.as_str())
                .await?;
        }
        let existing_authors = self
            .author_subscriptions
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for author in existing_authors {
            self.restart_author_subscription(author.as_str()).await?;
        }
        self.restart_direct_message_subscriptions().await?;
        Ok(())
    }

    pub async fn set_discovery_seeds(
        &self,
        mode: DiscoveryMode,
        env_locked: bool,
        configured_seed_peers: Vec<SeedPeer>,
        bootstrap_seed_peers: Vec<SeedPeer>,
    ) -> Result<()> {
        let effective_seed_peers =
            merge_seed_peers(configured_seed_peers.clone(), bootstrap_seed_peers.clone());
        self.transport
            .configure_discovery(
                mode,
                env_locked,
                configured_seed_peers,
                bootstrap_seed_peers,
            )
            .await?;
        self.docs_sync
            .set_seed_peers(effective_seed_peers.clone())
            .await?;
        self.blob_service
            .set_seed_peers(effective_seed_peers)
            .await?;
        let existing_topics = self
            .subscriptions
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for topic in existing_topics {
            self.restart_topic_subscription(topic.as_str()).await?;
        }
        let existing_private_topics = self
            .joined_private_channels
            .lock()
            .await
            .values()
            .map(|state| {
                (
                    state.topic_id.clone(),
                    state.channel_id.as_str().to_string(),
                )
            })
            .collect::<Vec<_>>();
        for (topic_id, channel_id) in existing_private_topics {
            self.restart_private_channel_subscription(topic_id.as_str(), channel_id.as_str())
                .await?;
        }
        let existing_authors = self
            .author_subscriptions
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for author in existing_authors {
            self.restart_author_subscription(author.as_str()).await?;
        }
        self.restart_direct_message_subscriptions().await?;
        Ok(())
    }

    pub async fn unsubscribe_topic(&self, topic_id: &str) -> Result<()> {
        if let Some(handle) = self.subscriptions.lock().await.remove(topic_id) {
            handle.abort();
        }
        let private_keys = self
            .private_channel_subscriptions
            .lock()
            .await
            .keys()
            .filter(|key| key.starts_with(&format!("{topic_id}::")))
            .cloned()
            .collect::<Vec<_>>();
        for key in private_keys {
            if let Some(handle) = self
                .private_channel_subscriptions
                .lock()
                .await
                .remove(key.as_str())
            {
                handle.abort();
            }
            let mut parts = key.splitn(3, "::");
            let _ = parts.next();
            if let Some(channel_id) = parts.next() {
                self.hint_transport
                    .unsubscribe_hints(&private_channel_hint_topic(channel_id))
                    .await?;
            }
        }
        let keys_to_remove = self
            .live_presence_tasks
            .lock()
            .await
            .keys()
            .filter(|key| key.starts_with(&format!("{topic_id}::")))
            .cloned()
            .collect::<Vec<_>>();
        for key in keys_to_remove {
            let mut parts = key.splitn(3, "::");
            let _ = parts.next();
            let channel_id = parts.next().unwrap_or(PUBLIC_CHANNEL_ID).to_string();
            let session_id = parts.next().unwrap_or_default().to_string();
            self.stop_live_presence_task(topic_id, channel_id.as_str(), session_id.as_str())
                .await;
        }
        self.hint_transport
            .unsubscribe_hints(&TopicId::new(topic_id))
            .await
    }

    pub async fn peer_ticket(&self) -> Result<Option<String>> {
        self.transport.export_ticket().await
    }
}
