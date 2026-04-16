use crate::service::*;

impl AppService {
    pub async fn create_private_channel(
        &self,
        input: CreatePrivateChannelInput,
    ) -> Result<JoinedPrivateChannelView> {
        self.ensure_topic_subscription(input.topic_id.as_str())
            .await?;
        let label = input.label.trim();
        if label.is_empty() {
            anyhow::bail!("private channel label is required");
        }
        let now = Utc::now().timestamp_millis();
        let owner_pubkey = self.current_author_pubkey();
        let channel_id = ChannelId::new(format!(
            "channel-{}-{}",
            now,
            short_id_suffix(owner_pubkey.as_str())
        ));
        let current_epoch_id =
            initial_private_channel_epoch_id(&input.audience_kind, now, owner_pubkey.as_str());
        let current_epoch_secret_hex = generate_keys().export_secret_hex();
        let state = JoinedPrivateChannelState {
            topic_id: input.topic_id.as_str().to_string(),
            channel_id: channel_id.clone(),
            label: label.to_string(),
            creator_pubkey: owner_pubkey.clone(),
            owner_pubkey: owner_pubkey.clone(),
            joined_via_pubkey: None,
            audience_kind: input.audience_kind.clone(),
            current_epoch_id: current_epoch_id.clone(),
            current_epoch_secret_hex: current_epoch_secret_hex.clone(),
            archived_epochs: Vec::new(),
        };
        self.register_joined_private_channel(state.clone()).await?;
        let metadata = PrivateChannelMetadataDocV1 {
            channel_id: channel_id.clone(),
            topic_id: input.topic_id.clone(),
            label: label.to_string(),
            creator_pubkey: Pubkey::from(state.creator_pubkey.clone()),
            created_at: now,
            audience_kind: input.audience_kind.clone(),
            owner_pubkey: Pubkey::from(owner_pubkey.clone()),
        };
        persist_private_channel_metadata(
            self.docs_sync.as_ref(),
            &current_private_channel_replica_id(&state),
            &metadata,
        )
        .await?;
        persist_private_channel_policy(
            self.docs_sync.as_ref(),
            self.keys.as_ref(),
            &PrivateChannelPolicyDocV1 {
                channel_id: channel_id.clone(),
                topic_id: input.topic_id.clone(),
                audience_kind: input.audience_kind.clone(),
                owner_pubkey: Pubkey::from(owner_pubkey.clone()),
                epoch_id: current_epoch_id,
                sharing_state: ChannelSharingState::Open,
                rotated_at: None,
                previous_epoch_id: None,
            },
            &current_private_channel_replica_id(&state),
        )
        .await?;
        persist_private_channel_participant(
            self.docs_sync.as_ref(),
            self.keys.as_ref(),
            &PrivateChannelParticipantDocV1 {
                channel_id,
                topic_id: input.topic_id,
                epoch_id: state.current_epoch_id.clone(),
                participant_pubkey: Pubkey::from(owner_pubkey),
                joined_at: now,
                is_owner: true,
                join_mode: Some(PrivateChannelJoinMode::OwnerSeed),
                sponsor_pubkey: None,
                share_token_id: None,
            },
            &current_private_channel_replica_id(&state),
        )
        .await?;
        self.joined_private_channel_view_for_state(&state).await
    }

    pub async fn export_private_channel_invite(
        &self,
        topic_id: &str,
        channel_id: &str,
        expires_at: Option<i64>,
    ) -> Result<String> {
        let state = self
            .private_channel_state_for_owner_action(
                topic_id,
                &ChannelId::new(channel_id),
                PrivateChannelOwnerAction::Share,
            )
            .await?;
        if state.audience_kind != ChannelAudienceKind::InviteOnly {
            anyhow::bail!(
                "private channel invite export is only available for invite-only channels"
            );
        }
        build_private_channel_invite_token(
            self.keys.as_ref(),
            PrivateChannelInviteTokenParams {
                topic: &TopicId::new(topic_id),
                channel_id: &state.channel_id,
                channel_label: state.label.as_str(),
                owner_pubkey: &Pubkey::from(state.owner_pubkey.clone()),
                epoch_id: state.current_epoch_id.as_str(),
                namespace_secret_hex: state.current_epoch_secret_hex.as_str(),
                expires_at,
            },
        )
    }

    pub async fn import_private_channel_invite(
        &self,
        token: &str,
    ) -> Result<PrivateChannelInvitePreview> {
        let preview = parse_private_channel_invite_token(token)?;
        if let Some(expires_at) = preview.expires_at
            && expires_at < Utc::now().timestamp_millis()
        {
            anyhow::bail!("private channel invite is expired");
        }
        self.ensure_topic_subscription(preview.topic_id.as_str())
            .await?;
        let replica = private_channel_replica_for_epoch(
            preview.channel_id.as_str(),
            preview.epoch_id.as_str(),
        );
        self.docs_sync
            .register_private_replica_secret(&replica, preview.namespace_secret_hex.as_str())
            .await?;
        let import_result = async {
            let (metadata, policy, participants) = wait_for_private_channel_epoch_snapshot(
                self.docs_sync.as_ref(),
                &replica,
                "invite-only channel replica sync",
            )
            .await?;
            if policy.audience_kind != ChannelAudienceKind::InviteOnly {
                anyhow::bail!("invite-only replica audience must be invite_only");
            }
            if policy.sharing_state != ChannelSharingState::Open {
                anyhow::bail!("invite-only access token is no longer open for import");
            }
            if policy.epoch_id != preview.epoch_id {
                anyhow::bail!("invite-only access token epoch does not match the current policy");
            }
            if !participants.iter().any(|participant| {
                participant.participant_pubkey == policy.owner_pubkey
                    && participant.epoch_id == policy.epoch_id
                    && participant.is_owner
            }) {
                anyhow::bail!("invite-only channel owner is not an active participant");
            }
            let local_pubkey = Pubkey::from(self.current_author_pubkey());
            if !participants.iter().any(|participant| {
                participant.participant_pubkey == local_pubkey
                    && participant.epoch_id == policy.epoch_id
            }) {
                persist_private_channel_participant(
                    self.docs_sync.as_ref(),
                    self.keys.as_ref(),
                    &PrivateChannelParticipantDocV1 {
                        channel_id: metadata.channel_id.clone(),
                        topic_id: metadata.topic_id.clone(),
                        epoch_id: policy.epoch_id.clone(),
                        participant_pubkey: local_pubkey,
                        joined_at: Utc::now().timestamp_millis(),
                        is_owner: false,
                        join_mode: Some(PrivateChannelJoinMode::InviteToken),
                        sponsor_pubkey: Some(preview.inviter_pubkey.clone()),
                        share_token_id: None,
                    },
                    &replica,
                )
                .await?;
            }
            let next_state = merged_private_channel_state_from_epoch_join(
                self.joined_private_channel_state(
                    preview.topic_id.as_str(),
                    preview.channel_id.as_str(),
                )
                .await,
                preview.topic_id.as_str(),
                preview.channel_id.clone(),
                preview.channel_label.as_str(),
                metadata.creator_pubkey.as_str(),
                preview.owner_pubkey.as_str(),
                Some(preview.inviter_pubkey.as_str()),
                ChannelAudienceKind::InviteOnly,
                preview.epoch_id.as_str(),
                preview.namespace_secret_hex.as_str(),
            );
            self.register_joined_private_channel(next_state).await?;
            Ok::<(), anyhow::Error>(())
        }
        .await;
        if import_result.is_err() {
            let _ = self.docs_sync.remove_private_replica_secret(&replica).await;
        }
        import_result?;
        Ok(preview)
    }

    pub async fn export_channel_access_token(
        &self,
        topic_id: &str,
        channel_id: &str,
        expires_at: Option<i64>,
    ) -> Result<ChannelAccessTokenExport> {
        let Some(state) = self
            .joined_private_channel_state(topic_id, channel_id)
            .await
        else {
            anyhow::bail!("private channel is not joined");
        };
        let (kind, token) = match state.audience_kind {
            ChannelAudienceKind::InviteOnly => (
                ChannelAccessTokenKind::Invite,
                self.export_private_channel_invite(topic_id, channel_id, expires_at)
                    .await?,
            ),
            ChannelAudienceKind::FriendOnly => (
                ChannelAccessTokenKind::Grant,
                self.export_friend_only_grant(topic_id, channel_id, expires_at)
                    .await?,
            ),
            ChannelAudienceKind::FriendPlus => (
                ChannelAccessTokenKind::Share,
                self.export_friend_plus_share(topic_id, channel_id, expires_at)
                    .await?,
            ),
        };
        Ok(ChannelAccessTokenExport { kind, token })
    }

    pub async fn import_channel_access_token(
        &self,
        token: &str,
    ) -> Result<ChannelAccessTokenPreview> {
        if let Ok(preview) = self.preview_channel_access_token(token).await {
            match preview.kind {
                ChannelAccessTokenKind::Invite => {
                    let preview = self.import_private_channel_invite(token).await?;
                    return Ok(ChannelAccessTokenPreview {
                        kind: ChannelAccessTokenKind::Invite,
                        topic_id: preview.topic_id.as_str().to_string(),
                        channel_id: preview.channel_id.as_str().to_string(),
                        channel_label: preview.channel_label,
                        owner_pubkey: preview.owner_pubkey.as_str().to_string(),
                        inviter_pubkey: Some(preview.inviter_pubkey.as_str().to_string()),
                        sponsor_pubkey: None,
                        epoch_id: preview.epoch_id,
                    });
                }
                ChannelAccessTokenKind::Grant => {
                    let preview = self.import_friend_only_grant(token).await?;
                    return Ok(ChannelAccessTokenPreview {
                        kind: ChannelAccessTokenKind::Grant,
                        topic_id: preview.topic_id.as_str().to_string(),
                        channel_id: preview.channel_id.as_str().to_string(),
                        channel_label: preview.channel_label,
                        owner_pubkey: preview.owner_pubkey.as_str().to_string(),
                        inviter_pubkey: None,
                        sponsor_pubkey: Some(preview.owner_pubkey.as_str().to_string()),
                        epoch_id: preview.epoch_id,
                    });
                }
                ChannelAccessTokenKind::Share => {
                    let preview = self.import_friend_plus_share(token).await?;
                    return Ok(ChannelAccessTokenPreview {
                        kind: ChannelAccessTokenKind::Share,
                        topic_id: preview.topic_id.as_str().to_string(),
                        channel_id: preview.channel_id.as_str().to_string(),
                        channel_label: preview.channel_label,
                        owner_pubkey: preview.owner_pubkey.as_str().to_string(),
                        inviter_pubkey: None,
                        sponsor_pubkey: Some(preview.sponsor_pubkey.as_str().to_string()),
                        epoch_id: preview.epoch_id,
                    });
                }
            }
        }
        anyhow::bail!("unrecognized private channel access token")
    }

    pub async fn preview_channel_access_token(
        &self,
        token: &str,
    ) -> Result<ChannelAccessTokenPreview> {
        if parse_private_channel_invite_token(token).is_ok() {
            let preview = parse_private_channel_invite_token(token)?;
            return Ok(ChannelAccessTokenPreview {
                kind: ChannelAccessTokenKind::Invite,
                topic_id: preview.topic_id.as_str().to_string(),
                channel_id: preview.channel_id.as_str().to_string(),
                channel_label: preview.channel_label,
                owner_pubkey: preview.owner_pubkey.as_str().to_string(),
                inviter_pubkey: Some(preview.inviter_pubkey.as_str().to_string()),
                sponsor_pubkey: None,
                epoch_id: preview.epoch_id,
            });
        }
        if parse_friend_only_grant_token(token).is_ok() {
            let preview = parse_friend_only_grant_token(token)?;
            return Ok(ChannelAccessTokenPreview {
                kind: ChannelAccessTokenKind::Grant,
                topic_id: preview.topic_id.as_str().to_string(),
                channel_id: preview.channel_id.as_str().to_string(),
                channel_label: preview.channel_label,
                owner_pubkey: preview.owner_pubkey.as_str().to_string(),
                inviter_pubkey: None,
                sponsor_pubkey: Some(preview.owner_pubkey.as_str().to_string()),
                epoch_id: preview.epoch_id,
            });
        }
        if parse_friend_plus_share_token(token).is_ok() {
            let preview = parse_friend_plus_share_token(token)?;
            return Ok(ChannelAccessTokenPreview {
                kind: ChannelAccessTokenKind::Share,
                topic_id: preview.topic_id.as_str().to_string(),
                channel_id: preview.channel_id.as_str().to_string(),
                channel_label: preview.channel_label,
                owner_pubkey: preview.owner_pubkey.as_str().to_string(),
                inviter_pubkey: None,
                sponsor_pubkey: Some(preview.sponsor_pubkey.as_str().to_string()),
                epoch_id: preview.epoch_id,
            });
        }
        anyhow::bail!("unrecognized private channel access token")
    }

    pub async fn export_friend_only_grant(
        &self,
        topic_id: &str,
        channel_id: &str,
        expires_at: Option<i64>,
    ) -> Result<String> {
        let state = self
            .private_channel_state_for_owner_action(
                topic_id,
                &ChannelId::new(channel_id),
                PrivateChannelOwnerAction::Share,
            )
            .await?;
        if state.audience_kind != ChannelAudienceKind::FriendOnly {
            anyhow::bail!("friend-only grant export is only available for friends channels");
        }
        if state.owner_pubkey != self.current_author_pubkey() {
            anyhow::bail!("only the channel owner can create friend-only grants");
        }
        let diagnostics = self.private_channel_diagnostics(&state).await?;
        if diagnostics.sharing_state != ChannelSharingState::Open {
            anyhow::bail!("friend-only grant export is disabled while sharing is frozen");
        }
        build_friend_only_grant_token(
            self.keys.as_ref(),
            &TopicId::new(topic_id),
            &state.channel_id,
            state.label.as_str(),
            state.current_epoch_id.as_str(),
            state.current_epoch_secret_hex.as_str(),
            expires_at,
        )
    }

    pub async fn import_friend_only_grant(&self, token: &str) -> Result<FriendOnlyGrantPreview> {
        let preview = parse_friend_only_grant_token(token)?;
        if let Some(expires_at) = preview.expires_at
            && expires_at < Utc::now().timestamp_millis()
        {
            anyhow::bail!("friend-only grant is expired");
        }
        self.ensure_topic_subscription(preview.topic_id.as_str())
            .await?;
        self.ensure_author_subscription(preview.owner_pubkey.as_str())
            .await?;
        self.rebuild_author_relationships().await?;
        let relationship = self
            .projection_store
            .get_author_relationship(
                self.current_author_pubkey().as_str(),
                preview.owner_pubkey.as_str(),
            )
            .await?;
        if !relationship.as_ref().is_some_and(|value| value.mutual) {
            anyhow::bail!(
                "friend-only grant import requires a mutual relationship with the channel owner"
            );
        }

        let replica = private_channel_epoch_replica_id(
            preview.channel_id.as_str(),
            preview.epoch_id.as_str(),
        );
        self.docs_sync
            .register_private_replica_secret(&replica, preview.namespace_secret_hex.as_str())
            .await?;
        let import_result = async {
            let (metadata, policy, participants) = wait_for_private_channel_epoch_snapshot(
                self.docs_sync.as_ref(),
                &replica,
                "friend-only channel replica sync",
            )
            .await?;
            if policy.audience_kind != ChannelAudienceKind::FriendOnly {
                anyhow::bail!("friend-only grant replica audience must be friend_only");
            }
            if policy.sharing_state != ChannelSharingState::Open {
                anyhow::bail!("friend-only grant is no longer open for import");
            }
            if policy.epoch_id != preview.epoch_id {
                anyhow::bail!("friend-only grant epoch does not match the current policy");
            }
            if !participants.iter().any(|participant| {
                participant.participant_pubkey == policy.owner_pubkey
                    && participant.epoch_id == policy.epoch_id
                    && participant.is_owner
            }) {
                anyhow::bail!("friend-only grant owner is not an active participant");
            }
            let joined_at = Utc::now().timestamp_millis();
            persist_private_channel_participant(
                self.docs_sync.as_ref(),
                self.keys.as_ref(),
                &PrivateChannelParticipantDocV1 {
                    channel_id: metadata.channel_id.clone(),
                    topic_id: metadata.topic_id.clone(),
                    epoch_id: policy.epoch_id.clone(),
                    participant_pubkey: Pubkey::from(self.current_author_pubkey()),
                    joined_at,
                    is_owner: false,
                    join_mode: Some(PrivateChannelJoinMode::FriendOnlyGrant),
                    sponsor_pubkey: Some(policy.owner_pubkey.clone()),
                    share_token_id: None,
                },
                &replica,
            )
            .await?;
            let next_state = merged_private_channel_state_from_epoch_join(
                self.joined_private_channel_state(
                    preview.topic_id.as_str(),
                    preview.channel_id.as_str(),
                )
                .await,
                preview.topic_id.as_str(),
                preview.channel_id.clone(),
                preview.channel_label.as_str(),
                metadata.creator_pubkey.as_str(),
                preview.owner_pubkey.as_str(),
                Some(preview.owner_pubkey.as_str()),
                ChannelAudienceKind::FriendOnly,
                preview.epoch_id.as_str(),
                preview.namespace_secret_hex.as_str(),
            );
            self.register_joined_private_channel(next_state).await?;
            Ok::<(), anyhow::Error>(())
        }
        .await;
        if import_result.is_err() {
            let _ = self.docs_sync.remove_private_replica_secret(&replica).await;
        }
        import_result?;
        Ok(preview)
    }

    pub async fn export_friend_plus_share(
        &self,
        topic_id: &str,
        channel_id: &str,
        expires_at: Option<i64>,
    ) -> Result<String> {
        let state = self
            .private_channel_state_for_owner_action(
                topic_id,
                &ChannelId::new(channel_id),
                PrivateChannelOwnerAction::Share,
            )
            .await?;
        if state.audience_kind != ChannelAudienceKind::FriendPlus {
            anyhow::bail!("friend-plus share export is only available for friends+ channels");
        }
        let replica = current_private_channel_replica_id(&state);
        let Some(policy) =
            fetch_private_channel_policy_from_replica(self.docs_sync.as_ref(), &replica).await?
        else {
            anyhow::bail!("friend-plus channel policy is missing");
        };
        if policy.sharing_state != ChannelSharingState::Open {
            anyhow::bail!("friend-plus share export is disabled while sharing is frozen");
        }
        let participants =
            fetch_private_channel_participants_from_replica(self.docs_sync.as_ref(), &replica)
                .await?;
        let local_author = self.current_author_pubkey();
        if !participants.iter().any(|participant| {
            participant.epoch_id == state.current_epoch_id
                && participant.participant_pubkey.as_str() == local_author
        }) {
            anyhow::bail!("only active participants can create friend-plus shares");
        }
        let effective_expires_at =
            expires_at.or_else(|| Some(Utc::now().timestamp_millis() + 24 * 60 * 60 * 1000));
        build_friend_plus_share_token(
            self.keys.as_ref(),
            &TopicId::new(topic_id),
            &state.channel_id,
            state.label.as_str(),
            &Pubkey::from(state.owner_pubkey.clone()),
            state.current_epoch_id.as_str(),
            state.current_epoch_secret_hex.as_str(),
            effective_expires_at,
        )
    }

    pub async fn import_friend_plus_share(&self, token: &str) -> Result<FriendPlusSharePreview> {
        let preview = parse_friend_plus_share_token(token)?;
        self.ensure_topic_subscription(preview.topic_id.as_str())
            .await?;
        self.ensure_author_subscription(preview.sponsor_pubkey.as_str())
            .await?;
        self.rebuild_author_relationships().await?;
        let relationship = self
            .projection_store
            .get_author_relationship(
                self.current_author_pubkey().as_str(),
                preview.sponsor_pubkey.as_str(),
            )
            .await?;
        if !relationship.as_ref().is_some_and(|value| value.mutual) {
            anyhow::bail!(
                "friend-plus share import requires a mutual relationship with the sponsor"
            );
        }

        let replica = private_channel_epoch_replica_id(
            preview.channel_id.as_str(),
            preview.epoch_id.as_str(),
        );
        self.docs_sync
            .register_private_replica_secret(&replica, preview.namespace_secret_hex.as_str())
            .await?;
        let import_result = async {
            let (metadata, policy, _participants) = wait_for_private_channel_epoch_snapshot(
                self.docs_sync.as_ref(),
                &replica,
                "friend-plus channel replica sync",
            )
            .await?;
            let participants =
                fetch_private_channel_participants_from_replica(self.docs_sync.as_ref(), &replica)
                    .await?;
            if policy.audience_kind != ChannelAudienceKind::FriendPlus {
                anyhow::bail!("friend-plus share replica audience must be friend_plus");
            }
            if policy.sharing_state != ChannelSharingState::Open {
                anyhow::bail!("friend-plus share is no longer open for import");
            }
            if policy.epoch_id != preview.epoch_id {
                anyhow::bail!("friend-plus share epoch does not match the current policy");
            }
            let local_author = self.current_author_pubkey();
            if !participants.iter().any(|participant| {
                participant.participant_pubkey.as_str() == local_author
                    && participant.epoch_id == policy.epoch_id
            }) {
                persist_private_channel_participant(
                    self.docs_sync.as_ref(),
                    self.keys.as_ref(),
                    &PrivateChannelParticipantDocV1 {
                        channel_id: metadata.channel_id.clone(),
                        topic_id: metadata.topic_id.clone(),
                        epoch_id: policy.epoch_id.clone(),
                        participant_pubkey: Pubkey::from(local_author),
                        joined_at: Utc::now().timestamp_millis(),
                        is_owner: false,
                        join_mode: Some(PrivateChannelJoinMode::FriendPlusShare),
                        sponsor_pubkey: Some(preview.sponsor_pubkey.clone()),
                        share_token_id: Some(preview.share_token_id.clone()),
                    },
                    &replica,
                )
                .await?;
            }
            let next_state = merged_private_channel_state_from_epoch_join(
                self.joined_private_channel_state(
                    preview.topic_id.as_str(),
                    preview.channel_id.as_str(),
                )
                .await,
                preview.topic_id.as_str(),
                preview.channel_id.clone(),
                preview.channel_label.as_str(),
                metadata.creator_pubkey.as_str(),
                preview.owner_pubkey.as_str(),
                Some(preview.sponsor_pubkey.as_str()),
                ChannelAudienceKind::FriendPlus,
                preview.epoch_id.as_str(),
                preview.namespace_secret_hex.as_str(),
            );
            self.register_joined_private_channel(next_state).await?;
            Ok::<(), anyhow::Error>(())
        }
        .await;
        if import_result.is_err() {
            let _ = self.docs_sync.remove_private_replica_secret(&replica).await;
        }
        import_result?;
        Ok(preview)
    }

    pub async fn freeze_private_channel(
        &self,
        topic_id: &str,
        channel_id: &str,
    ) -> Result<JoinedPrivateChannelView> {
        let Some(state) = self
            .joined_private_channel_state(topic_id, channel_id)
            .await
        else {
            anyhow::bail!("private channel is not joined");
        };
        if state.audience_kind != ChannelAudienceKind::FriendPlus {
            anyhow::bail!("freeze is only available for friend-plus channels");
        }
        if state.owner_pubkey != self.current_author_pubkey() {
            anyhow::bail!("only the channel owner can freeze the channel");
        }
        let current_replica = current_private_channel_replica_id(&state);
        let Some(current_policy) =
            fetch_private_channel_policy_from_replica(self.docs_sync.as_ref(), &current_replica)
                .await?
        else {
            anyhow::bail!("friend-plus channel policy is missing");
        };
        persist_private_channel_policy(
            self.docs_sync.as_ref(),
            self.keys.as_ref(),
            &PrivateChannelPolicyDocV1 {
                sharing_state: ChannelSharingState::Frozen,
                rotated_at: current_policy.rotated_at,
                ..current_policy
            },
            &current_replica,
        )
        .await?;
        self.joined_private_channel_view_for_state(&state).await
    }

    pub async fn rotate_private_channel(
        &self,
        topic_id: &str,
        channel_id: &str,
    ) -> Result<JoinedPrivateChannelView> {
        let Some(mut state) = self
            .joined_private_channel_state(topic_id, channel_id)
            .await
        else {
            anyhow::bail!("private channel is not joined");
        };
        if !private_channel_is_epoch_aware(&state.audience_kind) {
            anyhow::bail!("rotate is only available for epoch-aware private channels");
        }
        if state.owner_pubkey != self.current_author_pubkey() {
            anyhow::bail!("only the channel owner can rotate the channel");
        }
        let current_replica = current_private_channel_replica_id(&state);
        let current_policy =
            fetch_private_channel_policy_from_replica(self.docs_sync.as_ref(), &current_replica)
                .await?
                .unwrap_or(PrivateChannelPolicyDocV1 {
                    channel_id: state.channel_id.clone(),
                    topic_id: TopicId::new(topic_id),
                    audience_kind: state.audience_kind.clone(),
                    owner_pubkey: Pubkey::from(state.owner_pubkey.clone()),
                    epoch_id: state.current_epoch_id.clone(),
                    sharing_state: ChannelSharingState::Open,
                    rotated_at: None,
                    previous_epoch_id: None,
                });
        let current_participants = fetch_private_channel_participants_from_replica(
            self.docs_sync.as_ref(),
            &current_replica,
        )
        .await?;
        let mut rotation_recipients = BTreeMap::new();
        for participant in active_private_channel_participants(
            &current_participants,
            state.current_epoch_id.as_str(),
        ) {
            if participant.is_owner {
                continue;
            }
            rotation_recipients
                .entry(participant.participant_pubkey.as_str().to_string())
                .or_insert(participant);
        }
        for epoch in &state.archived_epochs {
            let archived_replica =
                private_channel_epoch_replica_id(channel_id, epoch.epoch_id.as_str());
            let archived_participants = fetch_private_channel_participants_from_replica(
                self.docs_sync.as_ref(),
                &archived_replica,
            )
            .await?;
            for participant in
                active_private_channel_participants(&archived_participants, epoch.epoch_id.as_str())
            {
                if participant.is_owner {
                    continue;
                }
                rotation_recipients
                    .entry(participant.participant_pubkey.as_str().to_string())
                    .or_insert(participant);
            }
        }
        persist_private_channel_policy(
            self.docs_sync.as_ref(),
            self.keys.as_ref(),
            &PrivateChannelPolicyDocV1 {
                sharing_state: ChannelSharingState::Frozen,
                rotated_at: Some(Utc::now().timestamp_millis()),
                ..current_policy
            },
            &current_replica,
        )
        .await?;

        let next_epoch_id = next_private_channel_epoch_id(self.current_author_pubkey().as_str());
        let next_secret = generate_keys().export_secret_hex();
        let next_replica = private_channel_epoch_replica_id(channel_id, next_epoch_id.as_str());
        self.docs_sync
            .register_private_replica_secret(&next_replica, next_secret.as_str())
            .await?;
        let metadata = PrivateChannelMetadataDocV1 {
            channel_id: state.channel_id.clone(),
            topic_id: TopicId::new(topic_id),
            label: state.label.clone(),
            creator_pubkey: Pubkey::from(state.creator_pubkey.clone()),
            created_at: Utc::now().timestamp_millis(),
            audience_kind: state.audience_kind.clone(),
            owner_pubkey: Pubkey::from(state.owner_pubkey.clone()),
        };
        persist_private_channel_metadata(self.docs_sync.as_ref(), &next_replica, &metadata).await?;
        persist_private_channel_policy(
            self.docs_sync.as_ref(),
            self.keys.as_ref(),
            &PrivateChannelPolicyDocV1 {
                channel_id: state.channel_id.clone(),
                topic_id: TopicId::new(topic_id),
                audience_kind: state.audience_kind.clone(),
                owner_pubkey: Pubkey::from(state.owner_pubkey.clone()),
                epoch_id: next_epoch_id.clone(),
                sharing_state: ChannelSharingState::Open,
                rotated_at: None,
                previous_epoch_id: Some(state.current_epoch_id.clone()),
            },
            &next_replica,
        )
        .await?;
        persist_private_channel_participant(
            self.docs_sync.as_ref(),
            self.keys.as_ref(),
            &PrivateChannelParticipantDocV1 {
                channel_id: state.channel_id.clone(),
                topic_id: TopicId::new(topic_id),
                epoch_id: next_epoch_id.clone(),
                participant_pubkey: Pubkey::from(state.owner_pubkey.clone()),
                joined_at: Utc::now().timestamp_millis(),
                is_owner: true,
                join_mode: Some(PrivateChannelJoinMode::OwnerSeed),
                sponsor_pubkey: None,
                share_token_id: None,
            },
            &next_replica,
        )
        .await?;
        for participant in rotation_recipients.into_values() {
            if state.audience_kind == ChannelAudienceKind::FriendOnly {
                self.ensure_author_subscription(participant.participant_pubkey.as_str())
                    .await?;
                let relationship = self
                    .projection_store
                    .get_author_relationship(
                        self.current_author_pubkey().as_str(),
                        participant.participant_pubkey.as_str(),
                    )
                    .await?;
                if !relationship.as_ref().is_some_and(|value| value.mutual) {
                    continue;
                }
            }
            let grant_doc = encrypt_private_channel_epoch_handoff_grant(
                self.keys.as_ref(),
                &PrivateChannelEpochHandoffGrantPayloadV1 {
                    channel_id: state.channel_id.clone(),
                    topic_id: TopicId::new(topic_id),
                    owner_pubkey: Pubkey::from(state.owner_pubkey.clone()),
                    recipient_pubkey: participant.participant_pubkey.clone(),
                    old_epoch_id: state.current_epoch_id.clone(),
                    new_epoch_id: next_epoch_id.clone(),
                    new_namespace_secret_hex: next_secret.clone(),
                },
            )?;
            persist_private_channel_rotation_grant(
                self.docs_sync.as_ref(),
                self.keys.as_ref(),
                &grant_doc,
                &current_replica,
            )
            .await?;
        }

        let archived_epoch_id = state.current_epoch_id.clone();
        let archived_secret = state.current_epoch_secret_hex.clone();
        archive_private_channel_epoch(
            &mut state,
            archived_epoch_id.as_str(),
            archived_secret.as_str(),
        );
        state.current_epoch_id = next_epoch_id;
        state.current_epoch_secret_hex = next_secret;
        self.register_joined_private_channel(state.clone()).await?;
        if let Err(error) = self
            .hint_transport
            .publish_hint(
                &channel_hint_topic_for(topic_id, Some(&state.channel_id)),
                GossipHint::TopicObjectsChanged {
                    topic_id: TopicId::new(topic_id),
                    objects: Vec::new(),
                },
            )
            .await
        {
            warn!(
                topic = %topic_id,
                channel_id = %state.channel_id.as_str(),
                epoch_id = %state.current_epoch_id,
                error = %error,
                "failed to publish private channel rotation hint"
            );
        }
        self.joined_private_channel_view_for_state(&state).await
    }

    pub async fn restore_private_channel_capability(
        &self,
        capability: PrivateChannelCapability,
    ) -> Result<()> {
        let state = joined_private_channel_state_from_capability(capability)?;
        self.ensure_topic_subscription(state.topic_id.as_str())
            .await?;
        self.register_joined_private_channel(state).await
    }

    pub async fn list_joined_private_channels(
        &self,
        topic_id: &str,
    ) -> Result<Vec<JoinedPrivateChannelView>> {
        self.ensure_topic_subscription(topic_id).await?;
        self.ensure_joined_private_channel_subscriptions(topic_id)
            .await?;
        self.maybe_restart_scope_replica_sync(topic_id, &TimelineScope::AllJoined)
            .await;
        self.maybe_redeem_rotation_grants_for_topic(topic_id)
            .await?;
        let mut items = Vec::new();
        for state in self.joined_private_channel_states_for_topic(topic_id).await {
            items.push(self.joined_private_channel_view_for_state(&state).await?);
        }
        Ok(items)
    }

    pub async fn get_private_channel_capability(
        &self,
        topic_id: &str,
        channel_id: &str,
    ) -> Result<Option<PrivateChannelCapability>> {
        self.maybe_redeem_rotation_grants_for_channel(topic_id, channel_id)
            .await?;
        let Some(state) = self
            .joined_private_channel_state(topic_id, channel_id)
            .await
        else {
            return Ok(None);
        };
        Ok(Some(
            self.private_channel_capability_from_state(&state).await?,
        ))
    }

    pub async fn list_private_channel_capabilities(&self) -> Result<Vec<PrivateChannelCapability>> {
        let states = self
            .joined_private_channels
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let mut items = Vec::with_capacity(states.len());
        for state in states {
            items.push(self.private_channel_capability_from_state(&state).await?);
        }
        Ok(items)
    }
}
