use super::*;

impl DesktopRuntime {
    pub async fn create_post(&self, request: CreatePostRequest) -> Result<String> {
        let attachments = request
            .attachments
            .into_iter()
            .map(pending_attachment_from_request)
            .collect::<Result<Vec<_>>>()?;
        self.app_service
            .create_post_with_attachments_in_channel(
                request.topic.as_str(),
                request.channel_ref,
                request.content.as_str(),
                request.reply_to.as_deref(),
                attachments,
            )
            .await
    }

    pub async fn create_repost(&self, request: CreateRepostRequest) -> Result<String> {
        self.app_service
            .create_repost(
                request.topic.as_str(),
                request.source_topic.as_str(),
                request.source_object_id.as_str(),
                request.commentary.as_deref(),
            )
            .await
    }

    pub async fn toggle_reaction(
        &self,
        request: ToggleReactionRequest,
    ) -> Result<ReactionStateView> {
        self.app_service
            .toggle_reaction(
                request.target_topic_id.as_str(),
                request.target_object_id.as_str(),
                reaction_key_from_request(request.reaction_key)?,
                request.channel_ref,
            )
            .await
    }

    pub async fn list_my_custom_reaction_assets(&self) -> Result<Vec<CustomReactionAssetView>> {
        self.app_service.list_my_custom_reaction_assets().await
    }

    pub async fn list_recent_reactions(
        &self,
        request: ListRecentReactionsRequest,
    ) -> Result<Vec<RecentReactionView>> {
        self.app_service
            .list_recent_reactions(request.limit.unwrap_or(8))
            .await
    }

    pub async fn create_custom_reaction_asset(
        &self,
        request: CreateCustomReactionAssetRequest,
    ) -> Result<CustomReactionAssetView> {
        let upload = request.upload;
        let raw = BASE64_STANDARD
            .decode(upload.data_base64.as_bytes())
            .context("failed to decode custom reaction upload")?;
        let normalized =
            normalize_custom_reaction_upload(raw, upload.mime.as_str(), &request.crop_rect)?;
        self.app_service
            .create_custom_reaction_asset(CreateCustomReactionAssetInput {
                search_key: request.search_key,
                mime: normalized.mime,
                bytes: normalized.bytes,
                width: 128,
                height: 128,
            })
            .await
    }

    pub async fn list_bookmarked_custom_reactions(
        &self,
    ) -> Result<Vec<BookmarkedCustomReactionView>> {
        self.app_service.list_bookmarked_custom_reactions().await
    }

    pub async fn bookmark_custom_reaction(
        &self,
        request: BookmarkCustomReactionRequest,
    ) -> Result<BookmarkedCustomReactionView> {
        self.app_service
            .bookmark_custom_reaction(CustomReactionAssetSnapshotV1 {
                asset_id: request.asset_id,
                owner_pubkey: request.owner_pubkey.into(),
                blob_hash: BlobHash::new(request.blob_hash),
                search_key: request.search_key,
                mime: request.mime,
                bytes: request.bytes,
                width: request.width,
                height: request.height,
            })
            .await
    }

    pub async fn remove_bookmarked_custom_reaction(
        &self,
        request: RemoveBookmarkedCustomReactionRequest,
    ) -> Result<()> {
        self.app_service
            .remove_bookmarked_custom_reaction(request.asset_id.as_str())
            .await
    }

    pub async fn list_bookmarked_posts(&self) -> Result<Vec<BookmarkedPostView>> {
        self.app_service.list_bookmarked_posts().await
    }

    pub async fn bookmark_post(&self, request: BookmarkPostRequest) -> Result<BookmarkedPostView> {
        self.app_service
            .bookmark_post(request.topic.as_str(), request.object_id.as_str())
            .await
    }

    pub async fn remove_bookmarked_post(&self, request: RemoveBookmarkedPostRequest) -> Result<()> {
        self.app_service
            .remove_bookmarked_post(request.object_id.as_str())
            .await
    }

    pub async fn list_timeline(&self, request: ListTimelineRequest) -> Result<TimelineView> {
        self.app_service
            .list_timeline_scoped(
                request.topic.as_str(),
                request.scope,
                request.cursor,
                request.limit.unwrap_or(50),
            )
            .await
    }

    pub async fn list_thread(&self, request: ListThreadRequest) -> Result<TimelineView> {
        self.app_service
            .list_thread(
                request.topic.as_str(),
                request.thread_id.as_str(),
                request.cursor,
                request.limit.unwrap_or(50),
            )
            .await
    }

    pub async fn list_profile_timeline(
        &self,
        request: ListProfileTimelineRequest,
    ) -> Result<TimelineView> {
        self.app_service
            .list_profile_timeline(
                request.pubkey.as_str(),
                request.cursor,
                request.limit.unwrap_or(50),
            )
            .await
    }

    pub async fn get_my_profile(&self) -> Result<Profile> {
        self.app_service.get_my_profile().await
    }

    pub async fn set_my_profile(&self, request: SetMyProfileRequest) -> Result<Profile> {
        self.app_service
            .set_my_profile(ProfileInput {
                name: request.name,
                display_name: request.display_name,
                about: request.about,
                picture: request.picture,
                picture_upload: request
                    .picture_upload
                    .map(pending_attachment_from_request)
                    .transpose()?,
                clear_picture: request.clear_picture,
            })
            .await
    }

    pub async fn follow_author(&self, request: AuthorRequest) -> Result<AuthorSocialView> {
        self.app_service
            .follow_author(request.pubkey.as_str())
            .await
    }

    pub async fn unfollow_author(&self, request: AuthorRequest) -> Result<AuthorSocialView> {
        self.app_service
            .unfollow_author(request.pubkey.as_str())
            .await
    }

    pub async fn get_author_social_view(&self, request: AuthorRequest) -> Result<AuthorSocialView> {
        self.app_service
            .get_author_social_view(request.pubkey.as_str())
            .await
    }

    pub async fn mute_author(&self, request: AuthorRequest) -> Result<AuthorSocialView> {
        self.app_service.mute_author(request.pubkey.as_str()).await
    }

    pub async fn unmute_author(&self, request: AuthorRequest) -> Result<AuthorSocialView> {
        self.app_service
            .unmute_author(request.pubkey.as_str())
            .await
    }

    pub async fn list_social_connections(
        &self,
        request: ListSocialConnectionsRequest,
    ) -> Result<Vec<AuthorSocialView>> {
        self.app_service.list_social_connections(request.kind).await
    }
}
