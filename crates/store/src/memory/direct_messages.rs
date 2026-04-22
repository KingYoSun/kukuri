use super::*;

impl MemoryStore {
    pub(super) async fn projection_upsert_direct_message_conversation_impl(
        &self,
        row: DirectMessageConversationRow,
    ) -> Result<()> {
        self.direct_message_conversations
            .write()
            .await
            .insert(row.dm_id.clone(), row);
        Ok(())
    }

    pub(super) async fn projection_get_direct_message_conversation_by_peer_impl(
        &self,
        peer_pubkey: &str,
    ) -> Result<Option<DirectMessageConversationRow>> {
        Ok(self
            .direct_message_conversations
            .read()
            .await
            .values()
            .find(|row| row.peer_pubkey == peer_pubkey)
            .cloned())
    }

    pub(super) async fn projection_get_direct_message_conversation_by_dm_id_impl(
        &self,
        dm_id: &str,
    ) -> Result<Option<DirectMessageConversationRow>> {
        Ok(self
            .direct_message_conversations
            .read()
            .await
            .get(dm_id)
            .cloned())
    }

    pub(super) async fn projection_list_direct_message_conversations_impl(
        &self,
    ) -> Result<Vec<DirectMessageConversationRow>> {
        let mut items = self
            .direct_message_conversations
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| right.dm_id.cmp(&left.dm_id))
        });
        Ok(items)
    }

    pub(super) async fn projection_put_direct_message_message_impl(
        &self,
        row: DirectMessageMessageRow,
    ) -> Result<()> {
        if self
            .direct_message_tombstones
            .read()
            .await
            .contains_key(&(row.dm_id.clone(), row.message_id.clone()))
        {
            return Ok(());
        }
        self.direct_message_rows
            .write()
            .await
            .insert((row.dm_id.clone(), row.message_id.clone()), row);
        Ok(())
    }

    pub(super) async fn projection_get_direct_message_message_impl(
        &self,
        dm_id: &str,
        message_id: &str,
    ) -> Result<Option<DirectMessageMessageRow>> {
        Ok(self
            .direct_message_rows
            .read()
            .await
            .get(&(dm_id.to_string(), message_id.to_string()))
            .cloned())
    }

    pub(super) async fn projection_list_direct_message_messages_impl(
        &self,
        dm_id: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<DirectMessageMessageRow>> {
        let mut items = self
            .direct_message_rows
            .read()
            .await
            .values()
            .filter(|row| row.dm_id == dm_id)
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.message_id.cmp(&left.message_id))
        });
        Ok(apply_desc_direct_message_cursor(items, cursor, limit))
    }

    pub(super) async fn projection_set_direct_message_acked_at_impl(
        &self,
        dm_id: &str,
        message_id: &str,
        acked_at: i64,
    ) -> Result<()> {
        if let Some(row) = self
            .direct_message_rows
            .write()
            .await
            .get_mut(&(dm_id.to_string(), message_id.to_string()))
        {
            row.acked_at = Some(acked_at);
        }
        Ok(())
    }

    pub(super) async fn projection_put_direct_message_outbox_impl(
        &self,
        row: DirectMessageOutboxRow,
    ) -> Result<()> {
        self.direct_message_outbox_rows
            .write()
            .await
            .insert((row.dm_id.clone(), row.message_id.clone()), row);
        Ok(())
    }

    pub(super) async fn projection_get_direct_message_outbox_impl(
        &self,
        dm_id: &str,
        message_id: &str,
    ) -> Result<Option<DirectMessageOutboxRow>> {
        Ok(self
            .direct_message_outbox_rows
            .read()
            .await
            .get(&(dm_id.to_string(), message_id.to_string()))
            .cloned())
    }

    pub(super) async fn projection_list_direct_message_outbox_impl(
        &self,
    ) -> Result<Vec<DirectMessageOutboxRow>> {
        let mut items = self
            .direct_message_outbox_rows
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.message_id.cmp(&right.message_id))
        });
        Ok(items)
    }

    pub(super) async fn projection_touch_direct_message_outbox_attempt_impl(
        &self,
        dm_id: &str,
        message_id: &str,
        attempted_at: i64,
    ) -> Result<()> {
        if let Some(row) = self
            .direct_message_outbox_rows
            .write()
            .await
            .get_mut(&(dm_id.to_string(), message_id.to_string()))
        {
            row.last_attempt_at = Some(attempted_at);
        }
        Ok(())
    }

    pub(super) async fn projection_remove_direct_message_outbox_impl(
        &self,
        dm_id: &str,
        message_id: &str,
    ) -> Result<()> {
        self.direct_message_outbox_rows
            .write()
            .await
            .remove(&(dm_id.to_string(), message_id.to_string()));
        Ok(())
    }

    pub(super) async fn projection_put_direct_message_tombstone_impl(
        &self,
        row: DirectMessageTombstoneRow,
    ) -> Result<()> {
        self.direct_message_tombstones
            .write()
            .await
            .insert((row.dm_id.clone(), row.message_id.clone()), row);
        Ok(())
    }

    pub(super) async fn projection_list_direct_message_tombstones_impl(
        &self,
        dm_id: &str,
    ) -> Result<Vec<DirectMessageTombstoneRow>> {
        let mut items = self
            .direct_message_tombstones
            .read()
            .await
            .values()
            .filter(|row| row.dm_id == dm_id)
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .deleted_at
                .cmp(&left.deleted_at)
                .then_with(|| right.message_id.cmp(&left.message_id))
        });
        Ok(items)
    }

    pub(super) async fn projection_has_direct_message_tombstone_impl(
        &self,
        dm_id: &str,
        message_id: &str,
    ) -> Result<bool> {
        Ok(self
            .direct_message_tombstones
            .read()
            .await
            .contains_key(&(dm_id.to_string(), message_id.to_string())))
    }

    pub(super) async fn projection_delete_direct_message_message_local_impl(
        &self,
        dm_id: &str,
        message_id: &str,
    ) -> Result<()> {
        self.direct_message_rows
            .write()
            .await
            .remove(&(dm_id.to_string(), message_id.to_string()));
        self.direct_message_outbox_rows
            .write()
            .await
            .remove(&(dm_id.to_string(), message_id.to_string()));
        Ok(())
    }

    pub(super) async fn projection_clear_direct_message_local_impl(
        &self,
        dm_id: &str,
    ) -> Result<()> {
        self.direct_message_rows
            .write()
            .await
            .retain(|(row_dm_id, _), _| row_dm_id != dm_id);
        self.direct_message_outbox_rows
            .write()
            .await
            .retain(|(row_dm_id, _), _| row_dm_id != dm_id);
        self.direct_message_conversations
            .write()
            .await
            .remove(dm_id);
        Ok(())
    }
}
