use super::*;

#[async_trait]
impl DirectMessageStore for MemoryStore {
    async fn upsert_direct_message_conversation(
        &self,
        row: DirectMessageConversationRow,
    ) -> Result<()> {
        self.direct_message_conversations
            .write()
            .await
            .insert(row.dm_id.clone(), row);
        Ok(())
    }

    async fn get_direct_message_conversation_by_peer(
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

    async fn get_direct_message_conversation_by_dm_id(
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

    async fn list_direct_message_conversations(&self) -> Result<Vec<DirectMessageConversationRow>> {
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

    async fn put_direct_message_message(&self, row: DirectMessageMessageRow) -> Result<()> {
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

    async fn get_direct_message_message(
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

    async fn list_direct_message_messages(
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

    async fn set_direct_message_acked_at(
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

    async fn put_direct_message_outbox(&self, row: DirectMessageOutboxRow) -> Result<()> {
        self.direct_message_outbox_rows
            .write()
            .await
            .insert((row.dm_id.clone(), row.message_id.clone()), row);
        Ok(())
    }

    async fn get_direct_message_outbox(
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

    async fn list_direct_message_outbox(&self) -> Result<Vec<DirectMessageOutboxRow>> {
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

    async fn touch_direct_message_outbox_attempt(
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

    async fn remove_direct_message_outbox(&self, dm_id: &str, message_id: &str) -> Result<()> {
        self.direct_message_outbox_rows
            .write()
            .await
            .remove(&(dm_id.to_string(), message_id.to_string()));
        Ok(())
    }

    async fn put_direct_message_tombstone(&self, row: DirectMessageTombstoneRow) -> Result<()> {
        self.direct_message_tombstones
            .write()
            .await
            .insert((row.dm_id.clone(), row.message_id.clone()), row);
        Ok(())
    }

    async fn list_direct_message_tombstones(
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

    async fn has_direct_message_tombstone(&self, dm_id: &str, message_id: &str) -> Result<bool> {
        Ok(self
            .direct_message_tombstones
            .read()
            .await
            .contains_key(&(dm_id.to_string(), message_id.to_string())))
    }

    async fn delete_direct_message_message_local(
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

    async fn clear_direct_message_local(&self, dm_id: &str) -> Result<()> {
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
