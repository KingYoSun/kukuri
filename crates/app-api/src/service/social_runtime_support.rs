use super::*;

impl AppService {
    pub(crate) async fn build_author_social_view(
        &self,
        author_pubkey: &str,
    ) -> Result<AuthorSocialView> {
        let profile = self.store.get_profile(author_pubkey).await?;
        let relationship = self
            .projection_store
            .get_author_relationship(self.current_author_pubkey().as_str(), author_pubkey)
            .await?;
        let muted = self
            .projection_store
            .get_muted_author(author_pubkey)
            .await?
            .is_some();
        Ok(author_social_view_from_parts(
            author_pubkey,
            profile.as_ref(),
            relationship.as_ref(),
            muted,
        ))
    }

    pub(crate) async fn rebuild_author_relationships(&self) -> Result<()> {
        rebuild_author_relationships_with_services(
            self.store.as_ref(),
            self.projection_store.as_ref(),
            self.current_author_pubkey().as_str(),
        )
        .await?;
        self.reconcile_direct_message_subscriptions().await
    }

    pub(crate) async fn restart_direct_message_subscriptions(&self) -> Result<()> {
        let existing_peers = self
            .direct_message_subscriptions
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for peer_pubkey in existing_peers {
            stop_direct_message_subscription_with_services(
                self.direct_message_subscriptions.as_ref(),
                self.hint_transport.as_ref(),
                self.keys.as_ref(),
                peer_pubkey.as_str(),
            )
            .await?;
        }
        self.reconcile_direct_message_subscriptions().await
    }

    pub(crate) async fn current_muted_author_pubkeys(&self) -> Result<BTreeSet<String>> {
        Ok(self
            .projection_store
            .list_muted_authors()
            .await?
            .into_iter()
            .map(|row| row.author_pubkey)
            .collect())
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
            let subscriptions = self.author_subscriptions.lock().await;
            match subscriptions.get(author_pubkey.as_str()) {
                Some(handle) if !handle.is_finished() => return Ok(()),
                Some(_) => Some(author_pubkey.to_string()),
                None => None,
            }
        };
        if let Some(stale_key) = stale_key {
            self.author_subscriptions
                .lock()
                .await
                .remove(stale_key.as_str());
        }

        self.spawn_author_subscription(author_pubkey.as_str()).await
    }

    pub(crate) async fn restart_author_subscription(&self, author_pubkey: &str) -> Result<()> {
        let author_pubkey = normalize_author_pubkey(author_pubkey)?;
        if let Some(handle) = self
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
            let mut deadlines = self.replica_sync_restart_deadlines.lock().await;
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
        let store = Arc::clone(&self.store);
        let projection_store = Arc::clone(&self.projection_store);
        let docs_sync = Arc::clone(&self.docs_sync);
        let blob_service = Arc::clone(&self.blob_service);
        let hint_transport = Arc::clone(&self.hint_transport);
        let transport = Arc::clone(&self.transport);
        let keys = Arc::clone(&self.keys);
        let last_sync = Arc::clone(&self.last_sync_ts);
        let direct_message_subscriptions = Arc::clone(&self.direct_message_subscriptions);
        let author_key = normalize_author_pubkey(author_pubkey)?;
        let local_author_pubkey = self.current_author_pubkey();
        let replica = author_replica_id(author_key.as_str());
        docs_sync.open_replica(&replica).await?;
        let mut doc_stream = docs_sync.subscribe_replica(&replica).await?;
        let author_key_for_task = author_key.clone();
        let handle = tokio::spawn(async move {
            let notification_baseline = match snapshot_follow_notification_baseline_with_policy(
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
            match hydrate_author_state_with_services_with_policy(
                docs_sync.as_ref(),
                store.as_ref(),
                projection_store.as_ref(),
                local_author_pubkey.as_str(),
                author_key_for_task.as_str(),
                DocFetchPolicy::LocalOnly,
            )
            .await
            {
                Ok(initial_count) if initial_count > 0 => {
                    *last_sync.lock().await = Some(Utc::now().timestamp_millis());
                    schedule_direct_message_reconcile_with_services(
                        Arc::clone(&store),
                        Arc::clone(&projection_store),
                        Arc::clone(&blob_service),
                        Arc::clone(&hint_transport),
                        Arc::clone(&transport),
                        Arc::clone(&keys),
                        Arc::clone(&last_sync),
                        Arc::clone(&direct_message_subscriptions),
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
            let recovery_store = Arc::clone(&store);
            let recovery_projection_store = Arc::clone(&projection_store);
            let recovery_docs_sync = Arc::clone(&docs_sync);
            let recovery_blob_service = Arc::clone(&blob_service);
            let recovery_hint_transport = Arc::clone(&hint_transport);
            let recovery_transport = Arc::clone(&transport);
            let recovery_keys = Arc::clone(&keys);
            let recovery_last_sync = Arc::clone(&last_sync);
            let recovery_direct_message_subscriptions = Arc::clone(&direct_message_subscriptions);
            let recovery_local_author_pubkey = local_author_pubkey.clone();
            let recovery_author_pubkey = author_key_for_task.clone();
            tokio::spawn(async move {
                match tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    hydrate_author_state_with_services(
                        recovery_docs_sync.as_ref(),
                        recovery_store.as_ref(),
                        recovery_projection_store.as_ref(),
                        recovery_local_author_pubkey.as_str(),
                        recovery_author_pubkey.as_str(),
                    ),
                )
                .await
                {
                    Ok(Ok(initial_count)) if initial_count > 0 => {
                        *recovery_last_sync.lock().await = Some(Utc::now().timestamp_millis());
                        schedule_direct_message_reconcile_with_services(
                            Arc::clone(&recovery_store),
                            Arc::clone(&recovery_projection_store),
                            Arc::clone(&recovery_blob_service),
                            Arc::clone(&recovery_hint_transport),
                            Arc::clone(&recovery_transport),
                            Arc::clone(&recovery_keys),
                            Arc::clone(&recovery_last_sync),
                            Arc::clone(&recovery_direct_message_subscriptions),
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
                        if let Ok(count) = hydrate_author_state_with_services(
                            docs_sync.as_ref(),
                            store.as_ref(),
                            projection_store.as_ref(),
                            local_author_pubkey.as_str(),
                            author_key_for_task.as_str(),
                        ).await
                        && count > 0
                        {
                            *last_sync.lock().await = Some(Utc::now().timestamp_millis());
                            schedule_direct_message_reconcile_with_services(
                                Arc::clone(&store),
                                Arc::clone(&projection_store),
                                Arc::clone(&blob_service),
                                Arc::clone(&hint_transport),
                                Arc::clone(&transport),
                                Arc::clone(&keys),
                                Arc::clone(&last_sync),
                                Arc::clone(&direct_message_subscriptions),
                                local_author_pubkey.clone(),
                                author_key_for_task.clone(),
                            );
                        }
                    }
                    else => break,
                }
            }
        });
        self.author_subscriptions
            .lock()
            .await
            .insert(author_key, handle);
        Ok(())
    }
}
