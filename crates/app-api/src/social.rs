use crate::service::*;

impl AppService {
    pub async fn warm_social_graph(&self) -> Result<()> {
        let local_author = self.current_author_pubkey();
        self.ensure_author_subscription(local_author.as_str())
            .await?;
        self.rebuild_author_relationships().await?;
        for edge in self
            .store
            .list_follow_edges_by_subject(local_author.as_str())
            .await?
        {
            if edge.status == FollowEdgeStatus::Active {
                self.ensure_author_subscription(edge.target_pubkey.as_str())
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn get_my_profile(&self) -> Result<Profile> {
        let local_author = self.current_author_pubkey();
        self.ensure_author_subscription(local_author.as_str())
            .await?;
        Ok(self
            .store
            .get_profile(local_author.as_str())
            .await?
            .unwrap_or(Profile {
                pubkey: Pubkey::from(local_author),
                ..Profile::default()
            }))
    }

    pub async fn set_my_profile(&self, input: ProfileInput) -> Result<Profile> {
        let author_pubkey = Pubkey::from(self.current_author_pubkey());
        let current_profile = self.get_my_profile().await?;
        let picture = if input.clear_picture || input.picture_upload.is_some() {
            normalize_optional_text(input.picture)
        } else {
            normalize_optional_text(input.picture).or(current_profile.picture.clone())
        };
        let picture_asset = if input.clear_picture {
            None
        } else if let Some(upload) = input.picture_upload {
            let stored = self
                .blob_service
                .put_blob(upload.bytes, upload.mime.as_str())
                .await?;
            Some(kukuri_core::AssetRef {
                hash: stored.hash,
                mime: stored.mime,
                bytes: stored.bytes,
                role: AssetRole::ProfileAvatar,
            })
        } else {
            current_profile.picture_asset.clone()
        };
        let envelope = build_profile_envelope(
            self.keys.as_ref(),
            &KukuriProfileEnvelopeContentV1 {
                author_pubkey: author_pubkey.clone(),
                name: normalize_optional_text(input.name),
                display_name: normalize_optional_text(input.display_name),
                about: normalize_optional_text(input.about),
                picture,
                picture_asset,
            },
        )?;
        let profile = parse_profile(&envelope)?
            .ok_or_else(|| anyhow::anyhow!("failed to parse profile envelope"))?;
        self.store.put_envelope(envelope.clone()).await?;
        self.projection_store
            .upsert_profile_cache(profile.clone())
            .await?;
        persist_profile_doc(self.docs_sync.as_ref(), &profile, &envelope).await?;
        self.rebuild_author_relationships().await?;
        *self.last_sync_ts.lock().await = Some(Utc::now().timestamp_millis());
        Ok(profile)
    }

    pub async fn follow_author(&self, pubkey: &str) -> Result<AuthorSocialView> {
        let target_pubkey = Pubkey::from(normalize_author_pubkey(pubkey)?);
        let envelope = build_follow_edge_envelope(
            self.keys.as_ref(),
            &target_pubkey,
            FollowEdgeStatus::Active,
        )?;
        let edge = parse_follow_edge(&envelope)?
            .ok_or_else(|| anyhow::anyhow!("failed to parse follow edge"))?;
        self.store.put_envelope(envelope.clone()).await?;
        persist_follow_edge_doc(self.docs_sync.as_ref(), &edge, &envelope).await?;
        self.ensure_author_subscription(target_pubkey.as_str())
            .await?;
        self.rebuild_author_relationships().await?;
        *self.last_sync_ts.lock().await = Some(Utc::now().timestamp_millis());
        self.build_author_social_view(target_pubkey.as_str()).await
    }

    pub async fn unfollow_author(&self, pubkey: &str) -> Result<AuthorSocialView> {
        let target_pubkey = Pubkey::from(normalize_author_pubkey(pubkey)?);
        let envelope = build_follow_edge_envelope(
            self.keys.as_ref(),
            &target_pubkey,
            FollowEdgeStatus::Revoked,
        )?;
        let edge = parse_follow_edge(&envelope)?
            .ok_or_else(|| anyhow::anyhow!("failed to parse follow edge"))?;
        self.store.put_envelope(envelope.clone()).await?;
        persist_follow_edge_doc(self.docs_sync.as_ref(), &edge, &envelope).await?;
        self.ensure_author_subscription(target_pubkey.as_str())
            .await?;
        self.rebuild_author_relationships().await?;
        *self.last_sync_ts.lock().await = Some(Utc::now().timestamp_millis());
        self.build_author_social_view(target_pubkey.as_str()).await
    }

    pub async fn get_author_social_view(&self, pubkey: &str) -> Result<AuthorSocialView> {
        let author_pubkey = normalize_author_pubkey(pubkey)?;
        self.ensure_author_subscription(author_pubkey.as_str())
            .await?;
        self.maybe_restart_author_subscription(author_pubkey.as_str())
            .await;
        self.rebuild_author_relationships().await?;
        self.build_author_social_view(author_pubkey.as_str()).await
    }

    pub async fn mute_author(&self, pubkey: &str) -> Result<AuthorSocialView> {
        let author_pubkey = normalize_author_pubkey(pubkey)?;
        self.ensure_author_subscription(author_pubkey.as_str())
            .await?;
        self.projection_store
            .put_muted_author(MutedAuthorRow {
                author_pubkey: author_pubkey.clone(),
                muted_at: Utc::now().timestamp_millis(),
            })
            .await?;
        self.build_author_social_view(author_pubkey.as_str()).await
    }

    pub async fn unmute_author(&self, pubkey: &str) -> Result<AuthorSocialView> {
        let author_pubkey = normalize_author_pubkey(pubkey)?;
        self.ensure_author_subscription(author_pubkey.as_str())
            .await?;
        self.projection_store
            .remove_muted_author(author_pubkey.as_str())
            .await?;
        self.build_author_social_view(author_pubkey.as_str()).await
    }

    pub async fn list_social_connections(
        &self,
        kind: SocialConnectionKind,
    ) -> Result<Vec<AuthorSocialView>> {
        let local_author_pubkey = self.current_author_pubkey();
        let pubkeys = match kind {
            SocialConnectionKind::Following => self
                .store
                .list_follow_edges_by_subject(local_author_pubkey.as_str())
                .await?
                .into_iter()
                .filter(|edge| edge.status == FollowEdgeStatus::Active)
                .map(|edge| edge.target_pubkey.as_str().to_string())
                .collect::<BTreeSet<_>>(),
            SocialConnectionKind::Followed => self
                .store
                .list_follow_edges_by_target(local_author_pubkey.as_str())
                .await?
                .into_iter()
                .filter(|edge| edge.status == FollowEdgeStatus::Active)
                .map(|edge| edge.subject_pubkey.as_str().to_string())
                .collect::<BTreeSet<_>>(),
            SocialConnectionKind::Muted => self
                .projection_store
                .list_muted_authors()
                .await?
                .into_iter()
                .map(|row| row.author_pubkey)
                .collect::<BTreeSet<_>>(),
        };
        let mut items = Vec::with_capacity(pubkeys.len());
        for author_pubkey in pubkeys {
            items.push(
                self.build_author_social_view(author_pubkey.as_str())
                    .await?,
            );
        }
        items.sort_by(author_social_view_sort_key);
        Ok(items)
    }
}
