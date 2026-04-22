use super::direct_messages_delivery_support::DirectMessageHintServices;
use super::*;

impl AppService {
    pub(crate) async fn direct_message_send_enabled(&self, peer_pubkey: &str) -> Result<bool> {
        Ok(self
            .projection_store
            .get_author_relationship(self.current_author_pubkey().as_str(), peer_pubkey)
            .await?
            .as_ref()
            .is_some_and(|relationship| relationship.mutual))
    }

    pub(crate) async fn reconcile_direct_message_subscriptions(&self) -> Result<()> {
        reconcile_direct_message_subscriptions_with_services(
            self.store.as_ref(),
            Arc::clone(&self.projection_store),
            Arc::clone(&self.blob_service),
            Arc::clone(&self.hint_transport),
            Arc::clone(&self.transport),
            Arc::clone(&self.keys),
            Arc::clone(&self.last_sync_ts),
            Arc::clone(&self.direct_message_subscriptions),
            self.current_author_pubkey().as_str(),
        )
        .await
    }

    pub(crate) async fn direct_message_status_view(
        &self,
        peer_pubkey: &str,
    ) -> Result<DirectMessageStatusView> {
        let dm_id = direct_message_id_for_participants(
            &Pubkey::from(self.current_author_pubkey()),
            &Pubkey::from(peer_pubkey),
        );
        let send_enabled = self.direct_message_send_enabled(peer_pubkey).await?;
        let peer_count = if send_enabled {
            self.direct_message_topic_peer_count(peer_pubkey).await?
        } else {
            0
        };
        let pending_outbox_count = self
            .projection_store
            .list_direct_message_outbox()
            .await?
            .into_iter()
            .filter(|row| row.peer_pubkey == peer_pubkey)
            .count();
        Ok(DirectMessageStatusView {
            peer_pubkey: peer_pubkey.to_string(),
            dm_id,
            mutual: send_enabled,
            send_enabled,
            peer_count,
            pending_outbox_count,
        })
    }

    pub(crate) async fn ensure_direct_message_conversation_row(
        &self,
        peer_pubkey: &str,
    ) -> Result<()> {
        if self
            .projection_store
            .get_direct_message_conversation_by_peer(peer_pubkey)
            .await?
            .is_some()
        {
            return Ok(());
        }
        let dm_id = direct_message_id_for_participants(
            &Pubkey::from(self.current_author_pubkey()),
            &Pubkey::from(peer_pubkey),
        );
        self.projection_store
            .upsert_direct_message_conversation(DirectMessageConversationRow {
                dm_id,
                peer_pubkey: peer_pubkey.to_string(),
                updated_at: Utc::now().timestamp_millis(),
                last_message_at: None,
                last_message_id: None,
                last_message_preview: None,
            })
            .await
    }

    pub(crate) async fn refresh_direct_message_conversation(
        &self,
        peer_pubkey: &str,
    ) -> Result<()> {
        let dm_id = direct_message_id_for_participants(
            &Pubkey::from(self.current_author_pubkey()),
            &Pubkey::from(peer_pubkey),
        );
        let existing = self
            .projection_store
            .get_direct_message_conversation_by_peer(peer_pubkey)
            .await?;
        let page = self
            .projection_store
            .list_direct_message_messages(dm_id.as_str(), None, 1)
            .await?;
        let (updated_at, last_message_at, last_message_id, last_message_preview) =
            if let Some(message) = page.items.first() {
                (
                    message.created_at,
                    Some(message.created_at),
                    Some(message.message_id.clone()),
                    Some(direct_message_preview(message)),
                )
            } else if let Some(existing) = existing.as_ref() {
                (existing.updated_at, None, None, None)
            } else if self.direct_message_send_enabled(peer_pubkey).await? {
                (Utc::now().timestamp_millis(), None, None, None)
            } else {
                return Ok(());
            };
        self.projection_store
            .upsert_direct_message_conversation(DirectMessageConversationRow {
                dm_id,
                peer_pubkey: peer_pubkey.to_string(),
                updated_at,
                last_message_at,
                last_message_id,
                last_message_preview,
            })
            .await
    }

    pub(crate) async fn direct_message_conversation_view(
        &self,
        peer_pubkey: &str,
    ) -> Result<DirectMessageConversationView> {
        let conversation = self
            .projection_store
            .get_direct_message_conversation_by_peer(peer_pubkey)
            .await?
            .ok_or_else(|| anyhow::anyhow!("direct message conversation is not initialized"))?;
        let profile = self.store.get_profile(peer_pubkey).await?;
        let status = self.direct_message_status_view(peer_pubkey).await?;
        Ok(DirectMessageConversationView {
            dm_id: conversation.dm_id,
            peer_pubkey: peer_pubkey.to_string(),
            peer_name: profile.as_ref().and_then(|value| value.name.clone()),
            peer_display_name: profile
                .as_ref()
                .and_then(|value| value.display_name.clone()),
            peer_picture: profile.as_ref().and_then(|value| value.picture.clone()),
            peer_picture_asset: profile_asset_view_from_ref(
                profile
                    .as_ref()
                    .and_then(|value| value.picture_asset.as_ref()),
            ),
            updated_at: conversation.updated_at,
            last_message_at: conversation.last_message_at,
            last_message_id: conversation.last_message_id,
            last_message_preview: conversation.last_message_preview,
            status,
        })
    }

    pub(crate) async fn direct_message_message_view(
        &self,
        row: DirectMessageMessageRow,
    ) -> Result<DirectMessageMessageView> {
        Ok(DirectMessageMessageView {
            dm_id: row.dm_id,
            message_id: row.message_id,
            sender_pubkey: row.sender_pubkey,
            recipient_pubkey: row.recipient_pubkey,
            created_at: row.created_at,
            text: row.text.unwrap_or_default(),
            reply_to_message_id: row.reply_to_message_id,
            attachments: direct_message_attachment_views(
                self.blob_service.as_ref(),
                row.attachment_manifest.as_ref(),
            )
            .await?,
            outgoing: row.outgoing,
            delivered: row.acked_at.is_some() || !row.outgoing,
        })
    }

    pub(crate) async fn notification_view_from_row(
        &self,
        row: NotificationRow,
    ) -> Result<NotificationView> {
        let object_id = row.object_id.clone();
        let thread_root_object_id = if let Some(object_id) = object_id.as_ref() {
            self.projection_store
                .get_object_projection(object_id)
                .await?
                .map(|projection| {
                    projection
                        .root_object_id
                        .unwrap_or(projection.object_id)
                        .as_str()
                        .to_string()
                })
        } else {
            None
        };
        let profile = self.store.get_profile(row.actor_pubkey.as_str()).await?;
        Ok(NotificationView {
            notification_id: row.notification_id,
            kind: row.kind,
            actor_pubkey: row.actor_pubkey,
            actor_name: profile.as_ref().and_then(|value| value.name.clone()),
            actor_display_name: profile
                .as_ref()
                .and_then(|value| value.display_name.clone()),
            actor_picture: profile.as_ref().and_then(|value| value.picture.clone()),
            actor_picture_asset: profile_asset_view_from_ref(
                profile
                    .as_ref()
                    .and_then(|value| value.picture_asset.as_ref()),
            ),
            source_envelope_id: row
                .source_envelope_id
                .map(|value| value.as_str().to_string()),
            source_replica_id: row
                .source_replica_id
                .map(|value| value.as_str().to_string()),
            topic_id: row.topic_id,
            channel_id: row.channel_id,
            object_id: object_id.map(|value| value.as_str().to_string()),
            thread_root_object_id,
            dm_id: row.dm_id,
            message_id: row.message_id,
            preview_text: row.preview_text,
            created_at: row.created_at,
            received_at: row.received_at,
            read_at: row.read_at,
        })
    }

    pub(crate) async fn notification_status_view(&self) -> Result<NotificationStatusView> {
        Ok(NotificationStatusView {
            unread_count: self.projection_store.count_unread_notifications().await?,
        })
    }

    pub(crate) async fn ensure_direct_message_subscription(&self, peer_pubkey: &str) -> Result<()> {
        let peer_pubkey = normalize_author_pubkey(peer_pubkey)?;
        if !self
            .direct_message_send_enabled(peer_pubkey.as_str())
            .await?
        {
            return Ok(());
        }
        let has_active_handle = self
            .direct_message_subscriptions
            .lock()
            .await
            .get(peer_pubkey.as_str())
            .is_some_and(|handle| !handle.is_finished());
        if has_active_handle {
            if self
                .should_restart_stale_direct_message_subscription(peer_pubkey.as_str())
                .await?
            {
                self.restart_direct_message_subscription(peer_pubkey.as_str())
                    .await?;
            }
            return Ok(());
        }
        Self::spawn_direct_message_subscription_with_services(
            Arc::clone(&self.direct_message_subscriptions),
            Arc::clone(&self.projection_store),
            Arc::clone(&self.blob_service),
            Arc::clone(&self.hint_transport),
            Arc::clone(&self.transport),
            Arc::clone(&self.keys),
            Arc::clone(&self.last_sync_ts),
            self.current_author_pubkey().as_str(),
            peer_pubkey.as_str(),
        )
        .await
    }

    pub(crate) async fn restart_direct_message_subscription(
        &self,
        peer_pubkey: &str,
    ) -> Result<()> {
        let peer_pubkey = normalize_author_pubkey(peer_pubkey)?;
        stop_direct_message_subscription_with_services(
            self.direct_message_subscriptions.as_ref(),
            self.hint_transport.as_ref(),
            self.keys.as_ref(),
            peer_pubkey.as_str(),
        )
        .await?;
        Self::spawn_direct_message_subscription_with_services(
            Arc::clone(&self.direct_message_subscriptions),
            Arc::clone(&self.projection_store),
            Arc::clone(&self.blob_service),
            Arc::clone(&self.hint_transport),
            Arc::clone(&self.transport),
            Arc::clone(&self.keys),
            Arc::clone(&self.last_sync_ts),
            self.current_author_pubkey().as_str(),
            peer_pubkey.as_str(),
        )
        .await
    }

    pub(crate) async fn direct_message_topic_snapshot(
        &self,
        peer_pubkey: &str,
    ) -> Result<Option<TopicPeerSnapshot>> {
        let peer_pubkey = normalize_author_pubkey(peer_pubkey)?;
        let topic =
            derive_direct_message_topic(self.keys.as_ref(), &Pubkey::from(peer_pubkey.as_str()))?;
        let hint_topic = format!("hint/{}", topic.as_str());
        Ok(self
            .transport
            .peers()
            .await?
            .topic_diagnostics
            .into_iter()
            .find(|diagnostic| {
                diagnostic.topic == hint_topic || diagnostic.topic == topic.as_str()
            }))
    }

    pub(crate) async fn should_restart_stale_direct_message_subscription(
        &self,
        peer_pubkey: &str,
    ) -> Result<bool> {
        let peer_pubkey = normalize_author_pubkey(peer_pubkey)?;
        let Some(snapshot) = self
            .direct_message_topic_snapshot(peer_pubkey.as_str())
            .await?
        else {
            return Ok(false);
        };
        if snapshot.joined || snapshot.peer_count > 0 || snapshot.configured_peer_ids.is_empty() {
            self.direct_message_subscription_restart_deadlines
                .lock()
                .await
                .remove(peer_pubkey.as_str());
            return Ok(false);
        }
        let now = Utc::now().timestamp();
        let mut deadlines = self
            .direct_message_subscription_restart_deadlines
            .lock()
            .await;
        let next_due_at = deadlines
            .get(peer_pubkey.as_str())
            .copied()
            .unwrap_or_default();
        if now < next_due_at {
            return Ok(false);
        }
        deadlines.insert(
            peer_pubkey,
            now.saturating_add(DIRECT_MESSAGE_SUBSCRIPTION_RESTART_RETRY_SECONDS),
        );
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn spawn_direct_message_subscription_with_services(
        direct_message_subscriptions: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
        projection_store: Arc<dyn ProjectionStore>,
        blob_service: Arc<dyn BlobService>,
        hint_transport: Arc<dyn HintTransport>,
        transport: Arc<dyn Transport>,
        keys: Arc<KukuriKeys>,
        last_sync: Arc<Mutex<Option<i64>>>,
        local_author_pubkey: &str,
        peer_pubkey: &str,
    ) -> Result<()> {
        let peer_pubkey = normalize_author_pubkey(peer_pubkey)?;
        {
            let mut subscriptions = direct_message_subscriptions.lock().await;
            if subscriptions
                .get(peer_pubkey.as_str())
                .is_some_and(|handle| !handle.is_finished())
            {
                return Ok(());
            }
            subscriptions.remove(peer_pubkey.as_str());
        }
        let topic =
            derive_direct_message_topic(keys.as_ref(), &Pubkey::from(peer_pubkey.as_str()))?;
        let mut hint_stream = hint_transport.subscribe_hints(&topic).await?;
        let topic_for_task = topic.clone();
        let peer_for_task = peer_pubkey.clone();
        let local_author_pubkey = local_author_pubkey.to_string();
        let task_hint_transport = Arc::clone(&hint_transport);
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(
                DIRECT_MESSAGE_RETRY_INTERVAL_MS,
            ));
            let _ = AppService::flush_direct_message_outbox_for_peer_with_services(
                projection_store.as_ref(),
                task_hint_transport.as_ref(),
                transport.as_ref(),
                local_author_pubkey.as_str(),
                keys.as_ref(),
                peer_for_task.as_str(),
            )
            .await;
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let _ = AppService::flush_direct_message_outbox_for_peer_with_services(
                            projection_store.as_ref(),
                            task_hint_transport.as_ref(),
                            transport.as_ref(),
                            local_author_pubkey.as_str(),
                            keys.as_ref(),
                            peer_for_task.as_str(),
                        ).await;
                    }
                    Some(event) = hint_stream.next() => {
                        if !matches!(
                            &event.hint,
                            GossipHint::DirectMessageFrame { topic_id, .. } | GossipHint::DirectMessageAck { topic_id, .. }
                            if topic_id.as_str() == topic_for_task.as_str()
                        ) {
                            continue;
                        }
                        if let Err(error) = blob_service.learn_peer(event.source_peer.as_str()).await {
                            warn!(
                                peer_pubkey = %peer_for_task,
                                source_peer = %event.source_peer,
                                error = %error,
                                "failed to learn direct message blob peer"
                            );
                        }
                        match AppService::handle_direct_message_hint_with_services(
                            DirectMessageHintServices {
                                projection_store: projection_store.as_ref(),
                                blob_service: blob_service.as_ref(),
                                hint_transport: task_hint_transport.as_ref(),
                                keys: keys.as_ref(),
                                local_author_pubkey: local_author_pubkey.as_str(),
                                peer_pubkey: peer_for_task.as_str(),
                                topic: &topic_for_task,
                            },
                            &event.hint,
                        ).await {
                            Ok(true) => {
                                *last_sync.lock().await = Some(Utc::now().timestamp_millis());
                            }
                            Ok(false) => {}
                            Err(error) => {
                                warn!(
                                    peer_pubkey = %peer_for_task,
                                    error = %error,
                                    "failed to handle direct message hint"
                                );
                            }
                        }
                    }
                    else => {
                        let _ = task_hint_transport.unsubscribe_hints(&topic_for_task).await;
                        break;
                    }
                }
            }
        });
        let mut pending_handle = Some(handle);
        let should_abort_new_handle = {
            let mut subscriptions = direct_message_subscriptions.lock().await;
            if subscriptions
                .get(peer_pubkey.as_str())
                .is_some_and(|existing| !existing.is_finished())
            {
                true
            } else {
                subscriptions.insert(
                    peer_pubkey.clone(),
                    pending_handle
                        .take()
                        .expect("direct message subscription handle must be pending"),
                );
                false
            }
        };
        if should_abort_new_handle {
            pending_handle
                .expect("direct message subscription handle must remain pending")
                .abort();
            hint_transport.unsubscribe_hints(&topic).await?;
        }
        Ok(())
    }
}
