use super::*;

impl AppService {
    pub(crate) async fn build_author_social_view(
        &self,
        author_pubkey: &str,
    ) -> Result<AuthorSocialView> {
        let profile = self.services.store.get_profile(author_pubkey).await?;
        let relationship = self
            .services
            .projection_store
            .get_author_relationship(self.current_author_pubkey().as_str(), author_pubkey)
            .await?;
        let muted = self
            .services
            .projection_store
            .get_muted_author(author_pubkey)
            .await?
            .is_some();
        let local_author = self.current_author_pubkey();
        let blocking = self
            .services
            .store
            .list_block_edges_by_subject(local_author.as_str())
            .await?
            .into_iter()
            .any(|edge| {
                edge.target_pubkey.as_str() == author_pubkey
                    && edge.status == BlockEdgeStatus::Active
            });
        let blocked_by = self
            .services
            .store
            .list_block_edges_by_target(local_author.as_str())
            .await?
            .into_iter()
            .any(|edge| {
                edge.subject_pubkey.as_str() == author_pubkey
                    && edge.status == BlockEdgeStatus::Active
            });
        let mut view = author_social_view_from_parts(
            author_pubkey,
            profile.as_ref(),
            relationship.as_ref(),
            muted,
            blocking,
            blocked_by,
        );
        view.provenance = self
            .content_provenance_view("profile", author_pubkey, "author_docs")
            .await?;
        Ok(view)
    }

    pub(crate) async fn rebuild_author_relationships(&self) -> Result<()> {
        rebuild_author_relationships(
            self.services.store.as_ref(),
            self.services.projection_store.as_ref(),
            self.current_author_pubkey().as_str(),
        )
        .await?;
        self.reconcile_direct_message_subscriptions().await
    }

    pub(crate) async fn restart_direct_message_subscriptions(&self) -> Result<()> {
        let existing_peers = self
            .subscription_registry
            .direct_message_subscriptions
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for peer_pubkey in existing_peers {
            stop_direct_message_subscription(
                self.subscription_registry
                    .direct_message_subscriptions
                    .as_ref(),
                &self.services,
                peer_pubkey.as_str(),
            )
            .await?;
        }
        self.reconcile_direct_message_subscriptions().await
    }

    pub(crate) async fn current_muted_author_pubkeys(&self) -> Result<BTreeSet<String>> {
        Ok(self
            .services
            .projection_store
            .list_muted_authors()
            .await?
            .into_iter()
            .map(|row| row.author_pubkey)
            .collect())
    }

    pub(crate) async fn authors_blocked_either_direction(
        &self,
        left_pubkey: &str,
        right_pubkey: &str,
    ) -> Result<bool> {
        let left_blocks_right = self
            .services
            .store
            .list_block_edges_by_subject(left_pubkey)
            .await?
            .into_iter()
            .any(|edge| {
                edge.target_pubkey.as_str() == right_pubkey
                    && edge.status == BlockEdgeStatus::Active
            });
        if left_blocks_right {
            return Ok(true);
        }
        Ok(self
            .services
            .store
            .list_block_edges_by_subject(right_pubkey)
            .await?
            .into_iter()
            .any(|edge| {
                edge.target_pubkey.as_str() == left_pubkey && edge.status == BlockEdgeStatus::Active
            }))
    }

    pub(crate) async fn owner_blocks_visitor(
        &self,
        owner_pubkey: &str,
        visitor_pubkey: &str,
    ) -> Result<bool> {
        Ok(self
            .services
            .store
            .list_block_edges_by_subject(owner_pubkey)
            .await?
            .into_iter()
            .any(|edge| {
                edge.target_pubkey.as_str() == visitor_pubkey
                    && edge.status == BlockEdgeStatus::Active
            }))
    }

    pub(crate) async fn ensure_author_subscriptions_for_rows(
        &self,
        rows: &[ObjectProjectionRow],
    ) -> Result<()> {
        let mut author_pubkeys = BTreeSet::new();
        for row in rows {
            author_pubkeys.insert(row.author_pubkey.clone());
            if let Some(repost_of) = row.repost_of.as_ref() {
                author_pubkeys.insert(repost_of.source_author_pubkey.as_str().to_string());
            }
        }
        for author_pubkey in author_pubkeys {
            self.ensure_author_subscription(author_pubkey.as_str())
                .await?;
        }
        Ok(())
    }

    pub(crate) async fn ensure_author_subscription(&self, author_pubkey: &str) -> Result<()> {
        let author_pubkey = normalize_author_pubkey(author_pubkey)?;
        let stale_key = {
            let subscriptions = self.subscription_registry.author_subscriptions.lock().await;
            match subscriptions.get(author_pubkey.as_str()) {
                Some(handle) if !handle.is_finished() => return Ok(()),
                Some(_) => Some(author_pubkey.to_string()),
                None => None,
            }
        };
        if let Some(stale_key) = stale_key {
            self.subscription_registry
                .author_subscriptions
                .lock()
                .await
                .remove(stale_key.as_str());
        }

        self.spawn_author_subscription(author_pubkey.as_str()).await
    }

    pub(crate) async fn restart_author_subscription(&self, author_pubkey: &str) -> Result<()> {
        let author_pubkey = normalize_author_pubkey(author_pubkey)?;
        if let Some(handle) = self
            .subscription_registry
            .author_subscriptions
            .lock()
            .await
            .remove(author_pubkey.as_str())
        {
            handle.abort();
        }
        self.spawn_author_subscription(author_pubkey.as_str()).await
    }

    pub(crate) async fn maybe_restart_author_subscription(&self, author_pubkey: &str) {
        let Ok(author_pubkey) = normalize_author_pubkey(author_pubkey) else {
            return;
        };
        let key = format!("author-subscription:{author_pubkey}");
        let now = Utc::now().timestamp();
        {
            let mut deadlines = self
                .subscription_registry
                .replica_sync_restart_deadlines
                .lock()
                .await;
            let next_due_at = deadlines.get(key.as_str()).copied().unwrap_or_default();
            if next_due_at > now {
                return;
            }
            deadlines.insert(key, now.saturating_add(REPLICA_SYNC_RESTART_RETRY_SECONDS));
        }
        if let Err(error) = self
            .restart_author_subscription(author_pubkey.as_str())
            .await
        {
            warn!(
                author_pubkey = %author_pubkey,
                error = %error,
                "failed to restart author subscription"
            );
        }
    }

    pub(crate) async fn spawn_author_subscription(&self, author_pubkey: &str) -> Result<()> {
        let services = self.services.clone();
        let last_sync = Arc::clone(&self.last_sync_ts);
        let notification_inserted = Arc::clone(&self.notification_inserted_notify);
        let direct_message_subscriptions =
            Arc::clone(&self.subscription_registry.direct_message_subscriptions);
        let author_key = normalize_author_pubkey(author_pubkey)?;
        let local_author_pubkey = self.current_author_pubkey();
        let replica = author_replica_id(author_key.as_str());
        services.docs_sync.open_replica(&replica).await?;
        let mut doc_stream = services.docs_sync.subscribe_replica(&replica).await?;
        let author_key_for_task = author_key.clone();
        let handle = tokio::spawn(async move {
            let store = &services.store;
            let projection_store = &services.projection_store;
            let docs_sync = &services.docs_sync;
            let blob_service = &services.blob_service;
            let notification_baseline = match snapshot_follow_notification_baseline(
                docs_sync.as_ref(),
                &replica,
                DocFetchPolicy::LocalOnly,
            )
            .await
            {
                Ok(baseline) => baseline,
                Err(error) => {
                    warn!(
                        author_pubkey = %author_key_for_task,
                        error = %error,
                        "failed to snapshot local follow baseline for author bootstrap"
                    );
                    NotificationDocEventBaseline::default()
                }
            };
            match hydrate_author_state(
                &services,
                local_author_pubkey.as_str(),
                author_key_for_task.as_str(),
                DocFetchPolicy::LocalOnly,
            )
            .await
            {
                Ok(initial_count) if initial_count > 0 => {
                    *last_sync.lock().await = Some(Utc::now().timestamp_millis());
                    schedule_direct_message_reconcile(
                        services.clone(),
                        Arc::clone(&last_sync),
                        Arc::clone(&direct_message_subscriptions),
                        Arc::clone(&notification_inserted),
                        local_author_pubkey.clone(),
                        author_key_for_task.clone(),
                    );
                }
                Ok(_) => {}
                Err(error) => {
                    warn!(
                        author_pubkey = %author_key_for_task,
                        error = %error,
                        "failed to hydrate local author cache during bootstrap"
                    );
                }
            }
            let recovery_services = services.clone();
            let recovery_last_sync = Arc::clone(&last_sync);
            let recovery_notification_inserted = Arc::clone(&notification_inserted);
            let recovery_direct_message_subscriptions = Arc::clone(&direct_message_subscriptions);
            let recovery_local_author_pubkey = local_author_pubkey.clone();
            let recovery_author_pubkey = author_key_for_task.clone();
            tokio::spawn(async move {
                match tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    hydrate_author_state(
                        &recovery_services,
                        recovery_local_author_pubkey.as_str(),
                        recovery_author_pubkey.as_str(),
                        DocFetchPolicy::LocalThenRemote,
                    ),
                )
                .await
                {
                    Ok(Ok(initial_count)) if initial_count > 0 => {
                        *recovery_last_sync.lock().await = Some(Utc::now().timestamp_millis());
                        schedule_direct_message_reconcile(
                            recovery_services,
                            Arc::clone(&recovery_last_sync),
                            Arc::clone(&recovery_direct_message_subscriptions),
                            Arc::clone(&recovery_notification_inserted),
                            recovery_local_author_pubkey,
                            recovery_author_pubkey,
                        );
                    }
                    Ok(Ok(_)) => {}
                    Ok(Err(error)) => {
                        warn!(
                            author_pubkey = %recovery_author_pubkey,
                            error = %error,
                            "failed to hydrate remote author cache during bootstrap recovery"
                        );
                    }
                    Err(_) => {
                        warn!(
                            author_pubkey = %recovery_author_pubkey,
                            "timed out hydrating remote author cache during bootstrap recovery"
                        );
                    }
                }
            });
            loop {
                tokio::select! {
                    Some(event) = doc_stream.next() => {
                        if event.is_err() {
                            continue;
                        }
                        if let Ok(event) = event.as_ref() {
                            if let Some(source_peer) = event.source_peer.as_deref() {
                                if let Err(error) = docs_sync.learn_peer(source_peer).await {
                                    warn!(
                                        author_pubkey = %author_key_for_task,
                                        source_peer = %source_peer,
                                        error = %error,
                                        "failed to learn docs peer from author sync event"
                                    );
                                }
                                if let Err(error) = blob_service.learn_peer(source_peer).await {
                                    warn!(
                                        author_pubkey = %author_key_for_task,
                                        source_peer = %source_peer,
                                        error = %error,
                                        "failed to learn blob peer from author sync event"
                                    );
                                }
                            }
                            match AppService::maybe_create_notification_for_remote_follow_event(
                                store.as_ref(),
                                projection_store.as_ref(),
                                docs_sync.as_ref(),
                                local_author_pubkey.as_str(),
                                author_key_for_task.as_str(),
                                &notification_baseline,
                                event,
                            ).await {
                                Ok(true) => {
                                    *last_sync.lock().await = Some(Utc::now().timestamp_millis());
                                    notification_inserted.notify_waiters();
                                }
                                Ok(false) => {}
                                Err(error) => {
                                    warn!(
                                        author_pubkey = %author_key_for_task,
                                        key = %event.key,
                                        error = %error,
                                        "failed to create notification from remote follow event"
                                    );
                                }
                            }
                        }
                        if let Ok(count) = hydrate_author_state(
                            &services,
                            local_author_pubkey.as_str(),
                            author_key_for_task.as_str(),
                            DocFetchPolicy::LocalThenRemote,
                        ).await
                        && count > 0
                        {
                            *last_sync.lock().await = Some(Utc::now().timestamp_millis());
                            schedule_direct_message_reconcile(
                                services.clone(),
                                Arc::clone(&last_sync),
                                Arc::clone(&direct_message_subscriptions),
                                Arc::clone(&notification_inserted),
                                local_author_pubkey.clone(),
                                author_key_for_task.clone(),
                            );
                        }
                    }
                    else => break,
                }
            }
        });
        self.subscription_registry
            .author_subscriptions
            .lock()
            .await
            .insert(author_key, handle);
        Ok(())
    }
}
