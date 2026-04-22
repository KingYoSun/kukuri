use super::*;

impl MemoryStore {
    pub(super) async fn projection_put_object_projection_impl(
        &self,
        row: ObjectProjectionRow,
    ) -> Result<()> {
        self.put_object_projections(vec![row]).await
    }

    pub(super) async fn projection_put_object_projections_impl(
        &self,
        rows: Vec<ObjectProjectionRow>,
    ) -> Result<()> {
        let mut projections = self.object_projection_rows.write().await;
        for row in rows {
            projections.insert(row.object_id.clone(), row);
        }
        Ok(())
    }

    pub(super) async fn projection_get_object_projection_impl(
        &self,
        object_id: &EnvelopeId,
    ) -> Result<Option<ObjectProjectionRow>> {
        Ok(self
            .object_projection_rows
            .read()
            .await
            .get(object_id)
            .cloned())
    }

    pub(super) async fn projection_list_topic_timeline_impl(
        &self,
        topic_id: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<ObjectProjectionRow>> {
        let mut items = self
            .object_projection_rows
            .read()
            .await
            .values()
            .filter(|row| row.topic_id == topic_id)
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.object_id.cmp(&left.object_id))
        });
        Ok(apply_desc_projection_cursor(items, cursor, limit))
    }

    pub(super) async fn projection_list_topic_timeline_filtered_impl(
        &self,
        topic_id: &str,
        allowed_channels: &BTreeSet<String>,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<ObjectProjectionRow>> {
        let mut items = self
            .object_projection_rows
            .read()
            .await
            .values()
            .filter(|row| {
                row.topic_id == topic_id && allowed_channels.contains(row.channel_id.as_str())
            })
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.object_id.cmp(&left.object_id))
        });
        Ok(apply_desc_projection_cursor(items, cursor, limit))
    }

    pub(super) async fn projection_list_thread_impl(
        &self,
        topic_id: &str,
        thread_root_object_id: &EnvelopeId,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<ObjectProjectionRow>> {
        let mut items = self
            .object_projection_rows
            .read()
            .await
            .values()
            .filter(|row| {
                row.topic_id == topic_id
                    && (row.object_id == *thread_root_object_id
                        || row
                            .root_object_id
                            .as_ref()
                            .is_some_and(|root| root == thread_root_object_id))
            })
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            let left_root = left.object_id == *thread_root_object_id;
            let right_root = right.object_id == *thread_root_object_id;
            left_root
                .cmp(&right_root)
                .reverse()
                .then_with(|| left.created_at.cmp(&right.created_at))
                .then_with(|| left.object_id.cmp(&right.object_id))
        });
        Ok(apply_asc_projection_cursor(items, cursor, limit))
    }

    pub(super) async fn projection_list_thread_filtered_impl(
        &self,
        topic_id: &str,
        thread_root_object_id: &EnvelopeId,
        allowed_channel: Option<&str>,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<ObjectProjectionRow>> {
        let mut items = self
            .object_projection_rows
            .read()
            .await
            .values()
            .filter(|row| {
                row.topic_id == topic_id
                    && allowed_channel.is_none_or(|channel_id| row.channel_id == channel_id)
                    && (row.object_id == *thread_root_object_id
                        || row
                            .root_object_id
                            .as_ref()
                            .is_some_and(|root| root == thread_root_object_id))
            })
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            let left_root = left.object_id == *thread_root_object_id;
            let right_root = right.object_id == *thread_root_object_id;
            left_root
                .cmp(&right_root)
                .reverse()
                .then_with(|| left.created_at.cmp(&right.created_at))
                .then_with(|| left.object_id.cmp(&right.object_id))
        });
        Ok(apply_asc_projection_cursor(items, cursor, limit))
    }

    pub(super) async fn projection_rebuild_object_projections_impl(
        &self,
        rows: Vec<ObjectProjectionRow>,
    ) -> Result<()> {
        let mut guard = self.object_projection_rows.write().await;
        guard.clear();
        for row in rows {
            guard.insert(row.object_id.clone(), row);
        }
        self.live_session_rows.write().await.clear();
        self.game_room_rows.write().await.clear();
        self.live_presence.write().await.clear();
        self.reaction_projection_rows.write().await.clear();
        Ok(())
    }
}
