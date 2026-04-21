use crate::service::*;

impl AppService {
    pub async fn get_sync_status(&self) -> Result<SyncStatus> {
        let PeerSnapshot {
            connected,
            peer_count,
            connected_peers: _,
            configured_peers,
            subscribed_topics,
            pending_events,
            status_detail,
            last_error,
            topic_diagnostics,
        } = self.transport.peers().await?;
        let subscribed_topics = normalize_topics(subscribed_topics);
        let topic_diagnostics = normalize_topic_diagnostics(topic_diagnostics);
        let docs_assist_peer_ids = self.docs_assisted_peer_ids().await?;
        let discovery = self.get_discovery_status().await?;
        let mut effective_delivery_state = if connected {
            DeliveryState::Live
        } else {
            DeliveryState::Offline
        };
        let mut effective_last_docs_activity_at = None;
        let mut effective_topic_diagnostics = Vec::with_capacity(topic_diagnostics.len());

        for diagnostic in topic_diagnostics {
            let delivery = self
                .public_topic_delivery_status(diagnostic.topic.as_str())
                .await;
            let last_docs_activity_at = delivery.and_then(|status| status.last_docs_activity_at);
            let delivery_state = delivery_state_for_topic(
                diagnostic.connected_peers.len(),
                docs_assist_peer_ids.len(),
                last_docs_activity_at,
            );
            effective_delivery_state =
                combine_delivery_states(effective_delivery_state, delivery_state);
            effective_last_docs_activity_at =
                merge_optional_timestamp(effective_last_docs_activity_at, last_docs_activity_at);
            effective_topic_diagnostics.push(TopicSyncStatus {
                topic: diagnostic.topic,
                joined: diagnostic.joined,
                delivery_state,
                peer_count: diagnostic.peer_count,
                connected_peers: diagnostic.connected_peers,
                docs_assist_peer_ids: docs_assist_peer_ids.clone(),
                configured_peer_ids: diagnostic.configured_peer_ids,
                missing_peer_ids: diagnostic.missing_peer_ids,
                last_received_at: diagnostic.last_received_at,
                last_docs_activity_at,
                status_detail: effective_topic_status_detail(
                    diagnostic.status_detail.as_str(),
                    delivery_state,
                    docs_assist_peer_ids.len(),
                ),
                last_error: diagnostic.last_error,
            });
        }

        if effective_delivery_state == DeliveryState::Offline
            && !docs_assist_peer_ids.is_empty()
            && !subscribed_topics.is_empty()
        {
            effective_delivery_state = if effective_last_docs_activity_at.is_some() {
                DeliveryState::DurableReady
            } else {
                DeliveryState::DurableRecovering
            };
        }

        Ok(SyncStatus {
            connected,
            delivery_state: effective_delivery_state,
            last_sync_ts: *self.last_sync_ts.lock().await,
            peer_count,
            pending_events,
            status_detail: effective_sync_status_detail(
                status_detail.as_str(),
                effective_delivery_state,
                docs_assist_peer_ids.len(),
                subscribed_topics.len(),
            ),
            last_error,
            configured_peers,
            subscribed_topics,
            topic_diagnostics: effective_topic_diagnostics,
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
        let docs_assist_peer_ids = self.docs_assisted_peer_ids().await?;
        let blob_assist_peer_ids = self.blob_assisted_peer_ids().await?;
        Ok(DiscoveryStatus {
            mode,
            connect_mode,
            env_locked,
            configured_seed_peer_ids,
            bootstrap_seed_peer_ids,
            manual_ticket_peer_ids,
            connected_peer_ids,
            docs_assist_peer_ids,
            blob_assist_peer_ids,
            local_endpoint_id,
            last_discovery_error,
        })
    }

    pub async fn import_peer_ticket(&self, ticket: &str) -> Result<()> {
        self.transport.import_ticket(ticket).await?;
        self.docs_sync.import_peer_ticket(ticket).await?;
        self.blob_service.import_peer_ticket(ticket).await?;
        self.restart_active_subscriptions().await?;
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
        self.restart_active_subscriptions().await?;
        Ok(())
    }

    pub async fn unsubscribe_topic(&self, topic_id: &str) -> Result<()> {
        if let Some(handle) = self.subscriptions.lock().await.remove(topic_id) {
            handle.abort();
        }
        self.clear_public_topic_delivery(topic_id).await;
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
