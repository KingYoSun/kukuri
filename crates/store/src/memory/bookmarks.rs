use super::*;

impl MemoryStore {
    pub(super) async fn projection_mark_blob_status_impl(
        &self,
        hash: &BlobHash,
        status: BlobCacheStatus,
    ) -> Result<()> {
        self.blob_statuses
            .write()
            .await
            .insert(hash.as_str().to_string(), status);
        Ok(())
    }

    pub(super) async fn projection_mark_blob_statuses_impl(
        &self,
        rows: Vec<(BlobHash, BlobCacheStatus)>,
    ) -> Result<()> {
        let mut statuses = self.blob_statuses.write().await;
        for (hash, status) in rows {
            statuses.insert(hash.as_str().to_string(), status);
        }
        Ok(())
    }

    pub(super) async fn projection_upsert_reaction_cache_impl(
        &self,
        row: ReactionProjectionRow,
    ) -> Result<()> {
        self.reaction_projection_rows.write().await.insert(
            (
                row.source_replica_id.as_str().to_string(),
                row.target_object_id.as_str().to_string(),
                row.reaction_id.as_str().to_string(),
            ),
            row,
        );
        Ok(())
    }

    pub(super) async fn projection_get_reaction_cache_impl(
        &self,
        source_replica_id: &ReplicaId,
        target_object_id: &EnvelopeId,
        reaction_id: &EnvelopeId,
    ) -> Result<Option<ReactionProjectionRow>> {
        Ok(self
            .reaction_projection_rows
            .read()
            .await
            .get(&(
                source_replica_id.as_str().to_string(),
                target_object_id.as_str().to_string(),
                reaction_id.as_str().to_string(),
            ))
            .cloned())
    }

    pub(super) async fn projection_list_reaction_cache_for_target_impl(
        &self,
        source_replica_id: &ReplicaId,
        target_object_id: &EnvelopeId,
    ) -> Result<Vec<ReactionProjectionRow>> {
        let mut items = self
            .reaction_projection_rows
            .read()
            .await
            .values()
            .filter(|row| {
                row.source_replica_id == *source_replica_id
                    && row.target_object_id == *target_object_id
            })
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            left.normalized_reaction_key
                .cmp(&right.normalized_reaction_key)
                .then_with(|| left.reaction_id.cmp(&right.reaction_id))
        });
        Ok(items)
    }

    pub(super) async fn projection_list_reaction_cache_for_targets_impl(
        &self,
        source_replica_id: &ReplicaId,
        target_object_ids: &[EnvelopeId],
    ) -> Result<HashMap<String, Vec<ReactionProjectionRow>>> {
        let target_ids = target_object_ids
            .iter()
            .map(|target_object_id| target_object_id.as_str().to_string())
            .collect::<HashSet<_>>();
        let mut grouped = HashMap::<String, Vec<ReactionProjectionRow>>::new();
        for row in self.reaction_projection_rows.read().await.values() {
            if row.source_replica_id == *source_replica_id
                && target_ids.contains(row.target_object_id.as_str())
            {
                grouped
                    .entry(row.target_object_id.as_str().to_string())
                    .or_default()
                    .push(row.clone());
            }
        }
        for rows in grouped.values_mut() {
            rows.sort_by(|left, right| {
                left.normalized_reaction_key
                    .cmp(&right.normalized_reaction_key)
                    .then_with(|| left.reaction_id.cmp(&right.reaction_id))
            });
        }
        Ok(grouped)
    }

    pub(super) async fn projection_list_recent_reaction_cache_by_author_impl(
        &self,
        author_pubkey: &str,
    ) -> Result<Vec<ReactionProjectionRow>> {
        let mut items = self
            .reaction_projection_rows
            .read()
            .await
            .values()
            .filter(|row| row.author_pubkey == author_pubkey)
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| right.reaction_id.cmp(&left.reaction_id))
        });
        Ok(items)
    }

    pub(super) async fn projection_put_bookmarked_custom_reaction_impl(
        &self,
        row: BookmarkedCustomReactionRow,
    ) -> Result<()> {
        self.bookmarked_custom_reactions
            .write()
            .await
            .insert(row.asset_id.clone(), row);
        Ok(())
    }

    pub(super) async fn projection_list_bookmarked_custom_reactions_impl(
        &self,
    ) -> Result<Vec<BookmarkedCustomReactionRow>> {
        let mut items = self
            .bookmarked_custom_reactions
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .bookmarked_at
                .cmp(&left.bookmarked_at)
                .then_with(|| right.asset_id.cmp(&left.asset_id))
        });
        Ok(items)
    }

    pub(super) async fn projection_remove_bookmarked_custom_reaction_impl(
        &self,
        asset_id: &str,
    ) -> Result<()> {
        self.bookmarked_custom_reactions
            .write()
            .await
            .remove(asset_id);
        Ok(())
    }

    pub(super) async fn projection_put_bookmarked_post_impl(
        &self,
        row: BookmarkedPostRow,
    ) -> Result<()> {
        self.bookmarked_posts
            .write()
            .await
            .insert(row.source_object_id.as_str().to_string(), row);
        Ok(())
    }

    pub(super) async fn projection_list_bookmarked_posts_impl(
        &self,
    ) -> Result<Vec<BookmarkedPostRow>> {
        let mut items = self
            .bookmarked_posts
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .bookmarked_at
                .cmp(&left.bookmarked_at)
                .then_with(|| right.source_object_id.cmp(&left.source_object_id))
        });
        Ok(items)
    }

    pub(super) async fn projection_remove_bookmarked_post_impl(
        &self,
        source_object_id: &EnvelopeId,
    ) -> Result<()> {
        self.bookmarked_posts
            .write()
            .await
            .remove(source_object_id.as_str());
        Ok(())
    }
}
