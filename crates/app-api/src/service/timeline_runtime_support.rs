use super::*;

impl AppService {
    pub(crate) async fn ensure_topic_subscription(&self, topic_id: &str) -> Result<()> {
        if self.is_topic_gossip_disabled(topic_id).await {
            return Ok(());
        }
        let stale_key = {
            let subscriptions = self.subscriptions.lock().await;
            match subscriptions.get(topic_id) {
                Some(handle) if !handle.is_finished() => return Ok(()),
                Some(_) => Some(topic_id.to_string()),
                None => None,
            }
        };
        if let Some(stale_key) = stale_key {
            self.subscriptions.lock().await.remove(stale_key.as_str());
        }

        self.spawn_topic_subscription(topic_id).await
    }

    pub(crate) async fn has_topic_subscription(&self, topic_id: &str) -> bool {
        self.subscriptions
            .lock()
            .await
            .get(topic_id)
            .is_some_and(|handle| !handle.is_finished())
    }

    pub(crate) async fn should_restart_after_empty_result(&self, key: &str) -> bool {
        !self
            .empty_recovery_candidates
            .lock()
            .await
            .insert(key.to_string())
    }

    pub(crate) async fn clear_empty_result_restart_marker(&self, key: &str) {
        self.empty_recovery_candidates.lock().await.remove(key);
    }

    pub(crate) async fn restart_topic_subscription(&self, topic_id: &str) -> Result<()> {
        if let Some(handle) = self.subscriptions.lock().await.remove(topic_id) {
            handle.abort();
        }
        self.hint_transport
            .unsubscribe_hints(&TopicId::new(topic_id))
            .await?;
        self.spawn_topic_subscription(topic_id).await
    }

    pub(crate) async fn spawn_topic_subscription(&self, topic_id: &str) -> Result<()> {
        self.spawn_subscription_task(
            topic_id,
            None,
            topic_replica_id(topic_id),
            TopicId::new(topic_id),
            None,
        )
        .await
    }

    pub(crate) async fn ingest_event(
        &self,
        replica: &ReplicaId,
        envelope: KukuriEnvelope,
        _stored_blob: Option<StoredBlob>,
        attachments: Vec<(AssetRole, StoredBlob)>,
    ) -> Result<()> {
        self.store.put_envelope(envelope.clone()).await?;
        let mut object = envelope
            .to_post_object()?
            .ok_or_else(|| anyhow::anyhow!("expected timeline envelope"))?;
        if object.object_kind != "repost" {
            object.attachments = attachments
                .iter()
                .map(|(role, stored)| kukuri_core::AssetRef {
                    hash: stored.hash.clone(),
                    mime: stored.mime.clone(),
                    bytes: stored.bytes,
                    role: role.clone(),
                })
                .collect();
        }
        let content = match &object.payload_ref {
            PayloadRef::InlineText { text } => Some(text.clone()),
            PayloadRef::BlobText { hash, .. } => self
                .blob_service
                .fetch_blob(hash)
                .await?
                .map(|bytes| String::from_utf8_lossy(&bytes).to_string()),
        };
        persist_post_object(
            self.docs_sync.as_ref(),
            replica,
            object.clone(),
            envelope.clone(),
        )
        .await?;
        if let Err(error) = self.docs_sync.restart_replica_sync(replica).await {
            warn!(
                replica_id = %replica.as_str(),
                error = %error,
                "failed to restart replica sync after local timeline write"
            );
        }
        ProjectionStore::put_object_projection(
            self.projection_store.as_ref(),
            projection_row_from_header(&object, content, replica),
        )
        .await?;
        if let PayloadRef::BlobText { hash, .. } = &object.payload_ref {
            ProjectionStore::mark_blob_status(
                self.projection_store.as_ref(),
                hash,
                BlobCacheStatus::Available,
            )
            .await?;
        }
        for (_, attachment) in attachments {
            ProjectionStore::mark_blob_status(
                self.projection_store.as_ref(),
                &attachment.hash,
                BlobCacheStatus::Available,
            )
            .await?;
        }
        *self.last_sync_ts.lock().await = Some(Utc::now().timestamp_millis());
        Ok(())
    }

    pub(crate) async fn resolve_parent_object(
        &self,
        object_id: &EnvelopeId,
    ) -> Result<Option<KukuriEnvelope>> {
        if let Some(envelope) = self.store.get_envelope(object_id).await? {
            return Ok(Some(envelope));
        }

        let Some(projection) =
            ProjectionStore::get_object_projection(self.projection_store.as_ref(), object_id)
                .await?
        else {
            return Ok(None);
        };

        let object_kind = projection.object_kind.as_str();
        let mut tags = vec![
            vec!["topic".into(), projection.topic_id.clone()],
            vec!["object".into(), object_kind.to_string()],
        ];
        if projection.channel_id != PUBLIC_CHANNEL_ID {
            tags.push(vec!["channel".into(), projection.channel_id.clone()]);
        }

        Ok(Some(KukuriEnvelope {
            id: projection.object_id,
            pubkey: projection.author_pubkey.into(),
            created_at: projection.created_at,
            kind: object_kind.into(),
            tags,
            content: serde_json::to_string(&kukuri_core::KukuriPostEnvelopeContentV1 {
                object_kind: object_kind.into(),
                topic_id: TopicId::new(projection.topic_id.clone()),
                channel_id: channel_id_from_storage(projection.channel_id.as_str()),
                payload_ref: projection.payload_ref.clone(),
                attachments: Vec::new(),
                media_manifest_refs: Vec::new(),
                visibility: if projection.channel_id == PUBLIC_CHANNEL_ID {
                    ObjectVisibility::Public
                } else {
                    ObjectVisibility::Private
                },
                reply_to: projection.reply_to_object_id.clone(),
                root_id: projection.root_object_id.clone(),
                repost_of: projection.repost_of.clone(),
            })?,
            sig: String::new(),
        }))
    }

    pub(crate) async fn ensure_scope_subscriptions(
        &self,
        topic_id: &str,
        scope: &TimelineScope,
    ) -> Result<()> {
        self.ensure_topic_subscription(topic_id).await?;
        match scope {
            TimelineScope::Public => Ok(()),
            TimelineScope::AllJoined => {
                self.ensure_joined_private_channel_subscriptions(topic_id)
                    .await
            }
            TimelineScope::Channel { channel_id } => {
                self.ensure_private_channel_access(topic_id, channel_id)
                    .await?;
                self.ensure_private_channel_subscription(topic_id, channel_id.as_str())
                    .await
            }
        }
    }

    pub(crate) async fn scope_needs_current_private_epoch_hydration(
        &self,
        topic_id: &str,
        scope: &TimelineScope,
        page: &Page<ObjectProjectionRow>,
    ) -> bool {
        let TimelineScope::Channel { channel_id } = scope else {
            return false;
        };
        let Some(state) = self
            .joined_private_channel_state(topic_id, channel_id.as_str())
            .await
        else {
            return false;
        };
        if state.archived_epochs.is_empty() {
            return false;
        }
        let current_replica = current_private_channel_replica_id(&state);
        !page
            .items
            .iter()
            .any(|item| item.source_replica_id == current_replica)
    }

    pub(crate) async fn allowed_channel_ids_for_scope(
        &self,
        topic_id: &str,
        scope: &TimelineScope,
    ) -> Result<BTreeSet<String>> {
        let mut allowed = BTreeSet::new();
        match scope {
            TimelineScope::Public => {
                allowed.insert(PUBLIC_CHANNEL_ID.to_string());
            }
            TimelineScope::AllJoined => {
                allowed.insert(PUBLIC_CHANNEL_ID.to_string());
                for state in self.joined_private_channel_states_for_topic(topic_id).await {
                    allowed.insert(state.channel_id.as_str().to_string());
                }
            }
            TimelineScope::Channel { channel_id } => {
                self.ensure_private_channel_access(topic_id, channel_id)
                    .await?;
                allowed.insert(channel_id.as_str().to_string());
            }
        }
        Ok(allowed)
    }

    pub(crate) async fn hydrate_scope_projection(
        &self,
        topic_id: &str,
        scope: &TimelineScope,
    ) -> Result<usize> {
        let mut hydrated = hydrate_topic_state_with_services_with_policy(
            self.docs_sync.as_ref(),
            self.blob_service.as_ref(),
            self.projection_store.as_ref(),
            topic_id,
            DocFetchPolicy::LocalOnly,
        )
        .await?;
        match scope {
            TimelineScope::Public => {}
            TimelineScope::AllJoined => {
                for state in self.joined_private_channel_states_for_topic(topic_id).await {
                    for replica in
                        private_channel_epoch_capabilities(&state)
                            .into_iter()
                            .map(|epoch| {
                                private_channel_replica_for_epoch(
                                    state.channel_id.as_str(),
                                    epoch.epoch_id.as_str(),
                                )
                            })
                    {
                        hydrated += hydrate_subscription_state_with_services_with_policy(
                            self.docs_sync.as_ref(),
                            self.blob_service.as_ref(),
                            self.projection_store.as_ref(),
                            topic_id,
                            &replica,
                            DocFetchPolicy::LocalOnly,
                        )
                        .await?;
                    }
                }
            }
            TimelineScope::Channel { channel_id } => {
                self.ensure_private_channel_access(topic_id, channel_id)
                    .await?;
                if let Some(state) = self
                    .joined_private_channel_state(topic_id, channel_id.as_str())
                    .await
                {
                    for replica in
                        private_channel_epoch_capabilities(&state)
                            .into_iter()
                            .map(|epoch| {
                                private_channel_replica_for_epoch(
                                    state.channel_id.as_str(),
                                    epoch.epoch_id.as_str(),
                                )
                            })
                    {
                        hydrated += hydrate_subscription_state_with_services_with_policy(
                            self.docs_sync.as_ref(),
                            self.blob_service.as_ref(),
                            self.projection_store.as_ref(),
                            topic_id,
                            &replica,
                            DocFetchPolicy::LocalOnly,
                        )
                        .await?;
                    }
                }
            }
        }
        Ok(hydrated)
    }

    pub(crate) async fn maybe_restart_scope_replica_sync(
        &self,
        topic_id: &str,
        scope: &TimelineScope,
    ) {
        self.maybe_restart_replica_sync(topic_id, &topic_replica_id(topic_id))
            .await;
        match scope {
            TimelineScope::Public => {}
            TimelineScope::AllJoined => {
                for state in self.joined_private_channel_states_for_topic(topic_id).await {
                    self.maybe_restart_private_channel_subscription(
                        topic_id,
                        state.channel_id.as_str(),
                    )
                    .await;
                    for replica in
                        private_channel_epoch_capabilities(&state)
                            .into_iter()
                            .map(|epoch| {
                                private_channel_replica_for_epoch(
                                    state.channel_id.as_str(),
                                    epoch.epoch_id.as_str(),
                                )
                            })
                    {
                        self.maybe_restart_replica_sync(topic_id, &replica).await;
                    }
                }
            }
            TimelineScope::Channel { channel_id } => {
                if let Some(state) = self
                    .joined_private_channel_state(topic_id, channel_id.as_str())
                    .await
                {
                    self.maybe_restart_private_channel_subscription(topic_id, channel_id.as_str())
                        .await;
                    for replica in
                        private_channel_epoch_capabilities(&state)
                            .into_iter()
                            .map(|epoch| {
                                private_channel_replica_for_epoch(
                                    state.channel_id.as_str(),
                                    epoch.epoch_id.as_str(),
                                )
                            })
                    {
                        self.maybe_restart_replica_sync(topic_id, &replica).await;
                    }
                }
            }
        }
    }

    pub(crate) async fn maybe_restart_replica_sync(&self, topic_id: &str, replica: &ReplicaId) {
        maybe_restart_replica_sync_with_cooldown(
            self.docs_sync.as_ref(),
            &self.replica_sync_restart_deadlines,
            topic_id,
            replica,
        )
        .await;
    }

    pub(crate) async fn maybe_restart_private_channel_subscription(
        &self,
        topic_id: &str,
        channel_id: &str,
    ) {
        let key = format!("private-channel:{topic_id}:{channel_id}");
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
            .restart_private_channel_subscription(topic_id, channel_id)
            .await
        {
            warn!(
                topic = %topic_id,
                channel_id = %channel_id,
                error = %error,
                "failed to restart private channel subscription"
            );
        }
    }

    pub(crate) async fn maybe_restart_topic_subscription(&self, topic_id: &str) {
        let key = format!("topic-subscription:{topic_id}");
        let now = Utc::now().timestamp();
        {
            let mut deadlines = self.replica_sync_restart_deadlines.lock().await;
            let next_due_at = deadlines.get(key.as_str()).copied().unwrap_or_default();
            if next_due_at > now {
                return;
            }
            deadlines.insert(key, now.saturating_add(REPLICA_SYNC_RESTART_RETRY_SECONDS));
        }
        if let Err(error) = self.restart_topic_subscription(topic_id).await {
            warn!(
                topic = %topic_id,
                error = %error,
                "failed to restart topic subscription"
            );
        }
    }

    pub(crate) async fn maybe_restart_scope_subscription(
        &self,
        topic_id: &str,
        scope: &TimelineScope,
    ) {
        self.maybe_restart_topic_subscription(topic_id).await;
        match scope {
            TimelineScope::Public => {}
            TimelineScope::AllJoined => {
                for state in self.joined_private_channel_states_for_topic(topic_id).await {
                    self.maybe_restart_private_channel_subscription(
                        topic_id,
                        state.channel_id.as_str(),
                    )
                    .await;
                }
            }
            TimelineScope::Channel { channel_id } => {
                self.maybe_restart_private_channel_subscription(topic_id, channel_id.as_str())
                    .await;
            }
        }
    }

    pub(crate) async fn page_to_view(
        &self,
        page: Page<ObjectProjectionRow>,
    ) -> Result<TimelineView> {
        let local_author = self.current_author_pubkey();
        let mut author_pubkeys = BTreeSet::new();
        let mut targets_by_replica = BTreeMap::<String, Vec<EnvelopeId>>::new();
        for row in &page.items {
            author_pubkeys.insert(row.author_pubkey.clone());
            if let Some(repost_of) = row.repost_of.as_ref() {
                author_pubkeys.insert(repost_of.source_author_pubkey.as_str().to_string());
            }
            targets_by_replica
                .entry(row.source_replica_id.as_str().to_string())
                .or_default()
                .push(row.object_id.clone());
        }

        let author_pubkeys = author_pubkeys.into_iter().collect::<Vec<_>>();
        let profiles = self.store.get_profiles(&author_pubkeys).await?;
        let relationships = self
            .projection_store
            .list_author_relationships(local_author.as_str(), &author_pubkeys)
            .await?;
        let mut reactions_by_target = HashMap::<String, Vec<ReactionProjectionRow>>::new();
        for (replica_id, object_ids) in targets_by_replica {
            let grouped = self
                .projection_store
                .list_reaction_cache_for_targets(&ReplicaId::new(replica_id.clone()), &object_ids)
                .await?;
            for (object_id, rows) in grouped {
                reactions_by_target.insert(format!("{replica_id}:{object_id}"), rows);
            }
        }

        let mut items = Vec::with_capacity(page.items.len());
        for row in page.items {
            items.push(
                self.row_to_view_with_cache(row, &profiles, &relationships, &reactions_by_target)
                    .await?,
            );
        }
        Ok(TimelineView {
            items,
            next_cursor: page.next_cursor,
        })
    }

    pub(crate) async fn row_to_view_with_cache(
        &self,
        row: ObjectProjectionRow,
        profiles: &HashMap<String, Profile>,
        relationships: &HashMap<String, AuthorRelationshipProjectionRow>,
        reactions_by_target: &HashMap<String, Vec<ReactionProjectionRow>>,
    ) -> Result<PostView> {
        let profile = profiles.get(row.author_pubkey.as_str());
        let relationship = relationships.get(row.author_pubkey.as_str());
        let repost_commentary = normalize_repost_commentary(row.content.clone());
        let content_status = if row.object_kind == "repost" {
            BlobViewStatus::Available
        } else {
            blob_view_status_for_payload(self.blob_service.as_ref(), &row.payload_ref).await?
        };
        let attachments = self.attachment_views_for_projection_row(&row).await?;
        let repost_of = match row.repost_of.clone() {
            Some(snapshot) => Some(
                self.repost_snapshot_to_view_with_profiles(snapshot, profiles)
                    .await?,
            ),
            None => None,
        };
        let reply_preview = self
            .reply_preview_for_object_id(
                row.reply_to_object_id.as_ref(),
                Some(&row.source_replica_id),
                profiles,
            )
            .await?;
        let audience_label = self
            .audience_label_for_storage(row.topic_id.as_str(), row.channel_id.as_str())
            .await;
        let reaction_state = reaction_state_view_from_rows(
            &row.source_replica_id,
            &row.object_id,
            reactions_by_target
                .get(reaction_cache_key(&row.source_replica_id, &row.object_id).as_str())
                .cloned()
                .unwrap_or_default(),
            self.current_author_pubkey().as_str(),
        );

        Ok(PostView {
            object_id: row.object_id.0.clone(),
            envelope_id: row.source_envelope_id.0.clone(),
            author_pubkey: row.author_pubkey.clone(),
            author_name: profile.and_then(|profile| profile.name.clone()),
            author_display_name: profile.and_then(|profile| profile.display_name.clone()),
            author_picture: profile.and_then(|profile| profile.picture.clone()),
            author_picture_asset: profile
                .and_then(|profile| profile_asset_view_from_ref(profile.picture_asset.as_ref())),
            following: relationship.is_some_and(|value| value.following),
            followed_by: relationship.is_some_and(|value| value.followed_by),
            mutual: relationship.is_some_and(|value| value.mutual),
            friend_of_friend: relationship.is_some_and(|value| value.friend_of_friend),
            content: row.content.unwrap_or_else(|| "[blob pending]".to_string()),
            content_status,
            attachments,
            created_at: row.created_at,
            reply_to: row.reply_to_object_id.clone().map(|id| id.0),
            reply_preview,
            root_id: row.root_object_id.clone().map(|id| id.0),
            object_kind: row.object_kind.clone(),
            published_topic_id: Some(row.topic_id.clone()),
            origin_topic_id: Some(row.topic_id.clone()),
            repost_of,
            repost_commentary: repost_commentary.clone(),
            is_threadable: row.object_kind != "repost" || repost_commentary.is_some(),
            channel_id: channel_id_for_view(row.channel_id.as_str()),
            audience_label,
            reaction_summary: reaction_state.reaction_summary,
            my_reactions: reaction_state.my_reactions,
        })
    }

    pub(crate) async fn hydrate_reply_preview_row(
        &self,
        object_id: &EnvelopeId,
        source_replica_id: Option<&ReplicaId>,
    ) -> Result<Option<ObjectProjectionRow>> {
        if let Some(row) = self
            .projection_store
            .get_object_projection(object_id)
            .await?
        {
            return Ok(Some(row));
        }
        let Some(source_replica_id) = source_replica_id else {
            return Ok(None);
        };
        let source_key = stable_key("objects", &format!("{}/state", object_id.as_str()));
        let Some(header) = fetch_post_object_for_projection(
            self.docs_sync.as_ref(),
            source_replica_id,
            source_key.as_str(),
        )
        .await?
        else {
            return Ok(None);
        };
        let content = match &header.payload_ref {
            PayloadRef::InlineText { text } => Some(text.clone()),
            PayloadRef::BlobText { hash, .. } => {
                fetch_projection_blob_text(self.blob_service.as_ref(), hash).await
            }
        };
        let row = projection_row_from_header(&header, content, source_replica_id);
        self.projection_store
            .put_object_projection(row.clone())
            .await?;
        Ok(Some(row))
    }

    pub(crate) async fn reply_preview_for_object_id(
        &self,
        object_id: Option<&EnvelopeId>,
        source_replica_id: Option<&ReplicaId>,
        profiles: &HashMap<String, Profile>,
    ) -> Result<Option<ReplyPreviewView>> {
        let Some(object_id) = object_id else {
            return Ok(None);
        };
        let Some(row) = self
            .hydrate_reply_preview_row(object_id, source_replica_id)
            .await?
        else {
            return Ok(None);
        };
        let attachments = self.attachment_views_for_projection_row(&row).await?;
        let profile = match profiles.get(row.author_pubkey.as_str()) {
            Some(profile) => Some(profile.clone()),
            None => self.store.get_profile(row.author_pubkey.as_str()).await?,
        };
        Ok(Some(ReplyPreviewView {
            object_id: row.object_id.0.clone(),
            topic: row.topic_id.clone(),
            author: ReplyPreviewAuthorView {
                pubkey: row.author_pubkey.clone(),
                name: profile.as_ref().and_then(|value| value.name.clone()),
                display_name: profile
                    .as_ref()
                    .and_then(|value| value.display_name.clone()),
                picture: profile.as_ref().and_then(|value| value.picture.clone()),
                picture_asset: profile
                    .as_ref()
                    .and_then(|value| profile_asset_view_from_ref(value.picture_asset.as_ref())),
            },
            content: row.content.unwrap_or_else(|| "[blob pending]".to_string()),
            attachments,
            root_id: row.root_object_id.map(|id| id.0),
            reply_to: row.reply_to_object_id.map(|id| id.0),
        }))
    }

    pub(crate) async fn attachment_views_for_projection_row(
        &self,
        row: &ObjectProjectionRow,
    ) -> Result<Vec<AttachmentView>> {
        if row.object_kind == "repost" {
            return Ok(Vec::new());
        }
        if !row.attachments.is_empty() || row.projection_version >= 2 {
            return attachment_views_from_refs(self.blob_service.as_ref(), &row.attachments).await;
        }

        let post_object = fetch_post_object_for_projection(
            self.docs_sync.as_ref(),
            &row.source_replica_id,
            row.source_key.as_str(),
        )
        .await?;
        if let Some(post_object) = post_object {
            return attachment_views(self.blob_service.as_ref(), &post_object).await;
        }
        Ok(Vec::new())
    }

    pub(crate) async fn bookmarked_post_view_from_row(
        &self,
        row: BookmarkedPostRow,
    ) -> Result<BookmarkedPostView> {
        let profile = self.store.get_profile(row.author_pubkey.as_str()).await?;
        let relationship = self
            .projection_store
            .get_author_relationship(
                self.current_author_pubkey().as_str(),
                row.author_pubkey.as_str(),
            )
            .await?;
        let content_status = if row.object_kind == "repost" {
            BlobViewStatus::Available
        } else {
            blob_view_status_for_payload(self.blob_service.as_ref(), &row.payload_ref).await?
        };
        let attachments = if row.object_kind == "repost" {
            Vec::new()
        } else {
            attachment_views_from_refs(self.blob_service.as_ref(), &row.attachments).await?
        };
        let repost_commentary = normalize_repost_commentary(row.content.clone());
        let repost_of = match row.repost_of.clone() {
            Some(snapshot) => Some(self.repost_snapshot_to_view(snapshot).await?),
            None => None,
        };
        let audience_label = self
            .audience_label_for_storage(row.topic_id.as_str(), row.channel_id.as_str())
            .await;
        let reaction_state = self
            .reaction_state_for_target(&row.source_replica_id, &row.source_object_id)
            .await?;
        let empty_profiles = HashMap::new();
        let reply_preview = self
            .reply_preview_for_object_id(
                row.reply_to_object_id.as_ref(),
                Some(&row.source_replica_id),
                &empty_profiles,
            )
            .await?;

        Ok(BookmarkedPostView {
            bookmarked_at: row.bookmarked_at,
            post: PostView {
                object_id: row.source_object_id.as_str().to_string(),
                envelope_id: row.source_envelope_id.as_str().to_string(),
                author_pubkey: row.author_pubkey.clone(),
                author_name: profile.as_ref().and_then(|profile| profile.name.clone()),
                author_display_name: profile
                    .as_ref()
                    .and_then(|profile| profile.display_name.clone()),
                author_picture: profile.as_ref().and_then(|profile| profile.picture.clone()),
                author_picture_asset: profile.as_ref().and_then(|profile| {
                    profile_asset_view_from_ref(profile.picture_asset.as_ref())
                }),
                following: relationship.as_ref().is_some_and(|value| value.following),
                followed_by: relationship.as_ref().is_some_and(|value| value.followed_by),
                mutual: relationship.as_ref().is_some_and(|value| value.mutual),
                friend_of_friend: relationship
                    .as_ref()
                    .is_some_and(|value| value.friend_of_friend),
                object_kind: row.object_kind.clone(),
                content: row.content.unwrap_or_else(|| "[blob pending]".to_string()),
                content_status,
                attachments,
                created_at: row.created_at,
                reply_to: row.reply_to_object_id.map(|id| id.0),
                reply_preview,
                root_id: row.root_object_id.map(|id| id.0),
                published_topic_id: Some(row.topic_id.clone()),
                origin_topic_id: Some(row.topic_id.clone()),
                repost_of,
                repost_commentary: repost_commentary.clone(),
                is_threadable: row.object_kind != "repost" || repost_commentary.is_some(),
                channel_id: channel_id_for_view(row.channel_id.as_str()),
                audience_label,
                reaction_summary: reaction_state.reaction_summary,
                my_reactions: reaction_state.my_reactions,
            },
        })
    }

    pub(crate) async fn profile_post_to_view(&self, profile_post: ProfilePost) -> Result<PostView> {
        let profile = self
            .store
            .get_profile(profile_post.author_pubkey.as_str())
            .await?;
        let relationship = self
            .projection_store
            .get_author_relationship(
                self.current_author_pubkey().as_str(),
                profile_post.author_pubkey.as_str(),
            )
            .await?;
        let empty_profiles = HashMap::new();
        let source_replica_id = topic_replica_id(profile_post.published_topic_id.as_str());
        let reply_preview = self
            .reply_preview_for_object_id(
                profile_post.reply_to_object_id.as_ref(),
                Some(&source_replica_id),
                &empty_profiles,
            )
            .await?;

        Ok(PostView {
            object_id: profile_post.object_id.0.clone(),
            envelope_id: profile_post.object_id.0.clone(),
            author_pubkey: profile_post.author_pubkey.as_str().to_string(),
            author_name: profile.as_ref().and_then(|value| value.name.clone()),
            author_display_name: profile
                .as_ref()
                .and_then(|value| value.display_name.clone()),
            author_picture: profile.as_ref().and_then(|value| value.picture.clone()),
            author_picture_asset: profile
                .as_ref()
                .and_then(|value| profile_asset_view_from_ref(value.picture_asset.as_ref())),
            following: relationship.as_ref().is_some_and(|value| value.following),
            followed_by: relationship.as_ref().is_some_and(|value| value.followed_by),
            mutual: relationship.as_ref().is_some_and(|value| value.mutual),
            friend_of_friend: relationship
                .as_ref()
                .is_some_and(|value| value.friend_of_friend),
            object_kind: profile_post.object_kind,
            content: profile_post.content,
            content_status: BlobViewStatus::Available,
            attachments: attachment_views_from_refs(
                self.blob_service.as_ref(),
                &profile_post.attachments,
            )
            .await?,
            created_at: profile_post.created_at,
            reply_to: profile_post.reply_to_object_id.map(|id| id.0),
            reply_preview,
            root_id: profile_post.root_id.map(|id| id.0),
            published_topic_id: Some(profile_post.published_topic_id.as_str().to_string()),
            origin_topic_id: Some(profile_post.published_topic_id.as_str().to_string()),
            repost_of: None,
            repost_commentary: None,
            is_threadable: true,
            channel_id: None,
            audience_label: "Public".into(),
            reaction_summary: Vec::new(),
            my_reactions: Vec::new(),
        })
    }

    pub(crate) async fn profile_repost_to_view(
        &self,
        profile_repost: ProfileRepost,
    ) -> Result<PostView> {
        let profile = self
            .store
            .get_profile(profile_repost.author_pubkey.as_str())
            .await?;
        let relationship = self
            .projection_store
            .get_author_relationship(
                self.current_author_pubkey().as_str(),
                profile_repost.author_pubkey.as_str(),
            )
            .await?;

        Ok(PostView {
            object_id: profile_repost.object_id.0.clone(),
            envelope_id: profile_repost.envelope_id.0.clone(),
            author_pubkey: profile_repost.author_pubkey.as_str().to_string(),
            author_name: profile.as_ref().and_then(|value| value.name.clone()),
            author_display_name: profile
                .as_ref()
                .and_then(|value| value.display_name.clone()),
            author_picture: profile.as_ref().and_then(|value| value.picture.clone()),
            author_picture_asset: profile
                .as_ref()
                .and_then(|value| profile_asset_view_from_ref(value.picture_asset.as_ref())),
            following: relationship.as_ref().is_some_and(|value| value.following),
            followed_by: relationship.as_ref().is_some_and(|value| value.followed_by),
            mutual: relationship.as_ref().is_some_and(|value| value.mutual),
            friend_of_friend: relationship
                .as_ref()
                .is_some_and(|value| value.friend_of_friend),
            object_kind: "repost".into(),
            content: profile_repost.commentary.clone().unwrap_or_default(),
            content_status: BlobViewStatus::Available,
            attachments: Vec::new(),
            created_at: profile_repost.created_at,
            reply_to: None,
            reply_preview: None,
            root_id: None,
            published_topic_id: Some(profile_repost.published_topic_id.as_str().to_string()),
            origin_topic_id: Some(profile_repost.published_topic_id.as_str().to_string()),
            repost_of: Some(
                self.repost_snapshot_to_view(profile_repost.repost_of)
                    .await?,
            ),
            repost_commentary: profile_repost.commentary.clone(),
            is_threadable: profile_repost.commentary.is_some(),
            channel_id: None,
            audience_label: "Public".into(),
            reaction_summary: Vec::new(),
            my_reactions: Vec::new(),
        })
    }

    pub(crate) async fn repost_snapshot_to_view(
        &self,
        snapshot: RepostSourceSnapshotV1,
    ) -> Result<RepostSourceView> {
        let profiles = self
            .store
            .get_profiles(&[snapshot.source_author_pubkey.as_str().to_string()])
            .await?;
        self.repost_snapshot_to_view_with_profiles(snapshot, &profiles)
            .await
    }

    pub(crate) async fn repost_snapshot_to_view_with_profiles(
        &self,
        snapshot: RepostSourceSnapshotV1,
        profiles: &HashMap<String, Profile>,
    ) -> Result<RepostSourceView> {
        let source_profile = profiles.get(snapshot.source_author_pubkey.as_str());
        Ok(RepostSourceView {
            source_object_id: snapshot.source_object_id.as_str().to_string(),
            source_topic_id: snapshot.source_topic_id.as_str().to_string(),
            source_author_pubkey: snapshot.source_author_pubkey.as_str().to_string(),
            source_author_name: source_profile.and_then(|value| value.name.clone()),
            source_author_display_name: source_profile.and_then(|value| value.display_name.clone()),
            source_author_picture: source_profile.and_then(|value| value.picture.clone()),
            source_author_picture_asset: source_profile
                .and_then(|value| profile_asset_view_from_ref(value.picture_asset.as_ref())),
            source_object_kind: snapshot.source_object_kind,
            content: snapshot.content,
            attachments: attachment_views_from_refs(
                self.blob_service.as_ref(),
                &snapshot.attachments,
            )
            .await?,
            reply_to: snapshot.reply_to_object_id.map(|id| id.0),
            root_id: snapshot.root_id.map(|id| id.0),
        })
    }
}

/// Upper bounds (in Unicode scalar values) for user-authored text that gets signed
/// into envelopes and replicated to other peers. They bound the gossip/docs payload
/// size and keep a single client from flooding the topic with oversized objects.
pub(crate) const MAX_POST_CONTENT_CHARS: usize = 10_000;
pub(crate) const MAX_REPOST_COMMENTARY_CHARS: usize = 2_000;
pub(crate) const MAX_PROFILE_NAME_CHARS: usize = 64;
pub(crate) const MAX_PROFILE_DISPLAY_NAME_CHARS: usize = 128;
pub(crate) const MAX_PROFILE_ABOUT_CHARS: usize = 2_000;

pub(crate) fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

/// Reject user text whose character count exceeds `max_chars`. Counting `chars()`
/// keeps the limit user-facing (it matches what a person types) while still bounding
/// the byte payload, since a scalar value is at most 4 bytes.
pub(crate) fn ensure_text_within_limit(field: &str, value: &str, max_chars: usize) -> Result<()> {
    let count = value.chars().count();
    if count > max_chars {
        anyhow::bail!("{field} must be at most {max_chars} characters (got {count})");
    }
    Ok(())
}

pub(crate) fn ensure_optional_text_within_limit(
    field: &str,
    value: Option<&str>,
    max_chars: usize,
) -> Result<()> {
    match value {
        Some(value) => ensure_text_within_limit(field, value, max_chars),
        None => Ok(()),
    }
}

pub(crate) fn profile_asset_view_from_ref(
    asset: Option<&kukuri_core::AssetRef>,
) -> Option<ProfileAssetView> {
    asset.map(|asset| ProfileAssetView {
        hash: asset.hash.as_str().to_string(),
        mime: asset.mime.clone(),
        bytes: asset.bytes,
        role: "profile_avatar".into(),
    })
}

pub(crate) fn normalize_repost_commentary(value: Option<String>) -> Option<String> {
    normalize_optional_text(value)
}

pub(crate) fn content_from_payload_ref(payload_ref: &PayloadRef) -> Option<String> {
    match payload_ref {
        PayloadRef::InlineText { text } => Some(text.clone()),
        PayloadRef::BlobText { .. } => None,
    }
}
