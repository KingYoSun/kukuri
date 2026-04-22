use super::*;

impl DesktopRuntime {
    pub async fn create_private_channel(
        &self,
        request: CreatePrivateChannelRequest,
    ) -> Result<JoinedPrivateChannelView> {
        let channel = self
            .app_service
            .create_private_channel(CreatePrivateChannelInput {
                topic_id: TopicId::new(request.topic),
                label: request.label,
                audience_kind: request.audience_kind,
            })
            .await?;
        self.persist_private_channel_capabilities_from_app().await?;
        Ok(channel)
    }

    pub async fn export_private_channel_invite(
        &self,
        request: ExportPrivateChannelInviteRequest,
    ) -> Result<String> {
        self.app_service
            .export_private_channel_invite(
                request.topic.as_str(),
                request.channel_id.as_str(),
                request.expires_at,
            )
            .await
    }

    pub async fn import_private_channel_invite(
        &self,
        request: ImportPrivateChannelInviteRequest,
    ) -> Result<PrivateChannelInvitePreview> {
        let preview = self
            .app_service
            .import_private_channel_invite(request.token.as_str())
            .await?;
        self.persist_private_channel_capabilities_from_app().await?;
        Ok(preview)
    }

    pub async fn export_channel_access_token(
        &self,
        request: ExportChannelAccessTokenRequest,
    ) -> Result<ChannelAccessTokenExport> {
        self.app_service
            .export_channel_access_token(
                request.topic.as_str(),
                request.channel_id.as_str(),
                request.expires_at,
            )
            .await
    }

    pub async fn import_channel_access_token(
        &self,
        request: ImportChannelAccessTokenRequest,
    ) -> Result<ChannelAccessTokenPreview> {
        let preview = self
            .app_service
            .import_channel_access_token(request.token.as_str())
            .await?;
        self.persist_private_channel_capabilities_from_app().await?;
        Ok(preview)
    }

    pub async fn preview_channel_access_token(
        &self,
        request: PreviewChannelAccessTokenRequest,
    ) -> Result<ChannelAccessTokenPreview> {
        self.app_service
            .preview_channel_access_token(request.token.as_str())
            .await
    }

    pub async fn export_friend_only_grant(
        &self,
        request: ExportFriendOnlyGrantRequest,
    ) -> Result<String> {
        self.app_service
            .export_friend_only_grant(
                request.topic.as_str(),
                request.channel_id.as_str(),
                request.expires_at,
            )
            .await
    }

    pub async fn import_friend_only_grant(
        &self,
        request: ImportFriendOnlyGrantRequest,
    ) -> Result<FriendOnlyGrantPreview> {
        let preview = self
            .app_service
            .import_friend_only_grant(request.token.as_str())
            .await?;
        self.persist_private_channel_capabilities_from_app().await?;
        Ok(preview)
    }

    pub async fn export_friend_plus_share(
        &self,
        request: ExportFriendPlusShareRequest,
    ) -> Result<String> {
        self.app_service
            .export_friend_plus_share(
                request.topic.as_str(),
                request.channel_id.as_str(),
                request.expires_at,
            )
            .await
    }

    pub async fn import_friend_plus_share(
        &self,
        request: ImportFriendPlusShareRequest,
    ) -> Result<FriendPlusSharePreview> {
        let preview = self
            .app_service
            .import_friend_plus_share(request.token.as_str())
            .await?;
        self.persist_private_channel_capabilities_from_app().await?;
        Ok(preview)
    }

    pub async fn freeze_private_channel(
        &self,
        request: FreezePrivateChannelRequest,
    ) -> Result<JoinedPrivateChannelView> {
        let view = self
            .app_service
            .freeze_private_channel(request.topic.as_str(), request.channel_id.as_str())
            .await?;
        self.persist_private_channel_capabilities_from_app().await?;
        Ok(view)
    }

    pub async fn rotate_private_channel(
        &self,
        request: RotatePrivateChannelRequest,
    ) -> Result<JoinedPrivateChannelView> {
        let view = self
            .app_service
            .rotate_private_channel(request.topic.as_str(), request.channel_id.as_str())
            .await?;
        self.persist_private_channel_capabilities_from_app().await?;
        Ok(view)
    }

    pub async fn list_joined_private_channels(
        &self,
        request: ListJoinedPrivateChannelsRequest,
    ) -> Result<Vec<JoinedPrivateChannelView>> {
        let items = self
            .app_service
            .list_joined_private_channels(request.topic.as_str())
            .await?;
        self.persist_private_channel_capabilities_from_app().await?;
        Ok(items)
    }

    pub async fn update_game_room(&self, request: UpdateGameRoomRequest) -> Result<()> {
        self.app_service
            .update_game_room(
                request.topic.as_str(),
                request.room_id.as_str(),
                UpdateGameRoomInput {
                    status: request.status,
                    phase_label: request.phase_label,
                    scores: request.scores,
                },
            )
            .await
    }

    pub async fn import_peer_ticket(&self, request: ImportPeerTicketRequest) -> Result<()> {
        self.app_service
            .import_peer_ticket(request.ticket.as_str())
            .await
    }

    pub async fn set_discovery_seeds(
        &self,
        request: SetDiscoverySeedsRequest,
    ) -> Result<DiscoveryConfig> {
        let mut next_config = self.discovery_config.lock().await.clone();
        if next_config.env_locked {
            bail!("discovery configuration is locked by environment variables");
        }
        next_config.seed_peers = parse_seed_entries(&request.seed_entries)?;
        save_discovery_config(&self.db_path, &next_config.stored())?;
        *self.discovery_config.lock().await = next_config.clone();
        self.apply_effective_seed_peers().await?;
        Ok(next_config)
    }

    pub async fn unsubscribe_topic(&self, request: UnsubscribeTopicRequest) -> Result<()> {
        self.app_service
            .unsubscribe_topic(request.topic.as_str())
            .await
    }

    pub async fn local_peer_ticket(&self) -> Result<Option<String>> {
        self.app_service.peer_ticket().await
    }

    pub async fn get_blob_preview_url(
        &self,
        request: GetBlobPreviewRequest,
    ) -> Result<Option<String>> {
        self.app_service
            .blob_preview_data_url(request.hash.as_str(), request.mime.as_str())
            .await
    }

    pub async fn get_blob_media_payload(
        &self,
        request: GetBlobMediaRequest,
    ) -> Result<Option<BlobMediaPayload>> {
        if request.hash.trim().is_empty() {
            tracing::warn!(mime = %request.mime, "blob media payload request skipped because hash was blank");
            return Ok(None);
        }
        self.app_service
            .blob_media_payload(request.hash.as_str(), request.mime.as_str())
            .await
    }
}
