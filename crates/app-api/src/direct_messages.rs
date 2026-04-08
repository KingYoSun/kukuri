use crate::service::*;

impl AppService {
    pub async fn resume_direct_message_state(&self) -> Result<()> {
        let mut peers = self
            .projection_store
            .list_direct_message_conversations()
            .await?
            .into_iter()
            .map(|row| row.peer_pubkey)
            .collect::<BTreeSet<_>>();
        for row in self.projection_store.list_direct_message_outbox().await? {
            peers.insert(row.peer_pubkey);
        }
        peers.extend(
            current_mutual_direct_message_peers_with_services(
                self.store.as_ref(),
                self.current_author_pubkey().as_str(),
            )
            .await?,
        );
        for peer_pubkey in peers {
            self.ensure_author_subscription(peer_pubkey.as_str())
                .await?;
        }
        self.rebuild_author_relationships().await?;
        Ok(())
    }

    pub async fn open_direct_message(
        &self,
        peer_pubkey: &str,
    ) -> Result<DirectMessageConversationView> {
        let peer_pubkey = normalize_author_pubkey(peer_pubkey)?;
        self.ensure_author_subscription(peer_pubkey.as_str())
            .await?;
        self.rebuild_author_relationships().await?;
        let existing = self
            .projection_store
            .get_direct_message_conversation_by_peer(peer_pubkey.as_str())
            .await?;
        let can_send = self
            .direct_message_send_enabled(peer_pubkey.as_str())
            .await?;
        if existing.is_none() && !can_send {
            anyhow::bail!("direct message requires a mutual relationship");
        }
        if can_send {
            self.ensure_direct_message_subscription(peer_pubkey.as_str())
                .await?;
        }
        self.ensure_direct_message_conversation_row(peer_pubkey.as_str())
            .await?;
        self.direct_message_conversation_view(peer_pubkey.as_str())
            .await
    }

    pub async fn list_direct_messages(&self) -> Result<Vec<DirectMessageConversationView>> {
        let rows = self
            .projection_store
            .list_direct_message_conversations()
            .await?;
        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            // Re-establish DM subscriptions for mutual peers when the conversation list is polled
            // after a restart. This keeps queued outbox delivery progressing even if the UI only
            // reopens the conversation list first.
            self.ensure_direct_message_subscription(row.peer_pubkey.as_str())
                .await?;
            items.push(
                self.direct_message_conversation_view(row.peer_pubkey.as_str())
                    .await?,
            );
        }
        Ok(items)
    }

    pub async fn list_direct_message_messages(
        &self,
        peer_pubkey: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<DirectMessageTimelineView> {
        let peer_pubkey = normalize_author_pubkey(peer_pubkey)?;
        let existing = self
            .projection_store
            .get_direct_message_conversation_by_peer(peer_pubkey.as_str())
            .await?;
        let can_send = self
            .direct_message_send_enabled(peer_pubkey.as_str())
            .await?;
        if existing.is_none() && !can_send {
            anyhow::bail!("direct message requires a mutual relationship");
        }
        if can_send {
            self.ensure_direct_message_subscription(peer_pubkey.as_str())
                .await?;
        }
        self.ensure_direct_message_conversation_row(peer_pubkey.as_str())
            .await?;
        let dm_id = direct_message_id_for_participants(
            &Pubkey::from(self.current_author_pubkey()),
            &Pubkey::from(peer_pubkey.as_str()),
        );
        let page = self
            .projection_store
            .list_direct_message_messages(dm_id.as_str(), cursor, limit)
            .await?;
        let mut items = Vec::with_capacity(page.items.len());
        for row in page.items {
            items.push(self.direct_message_message_view(row).await?);
        }
        Ok(DirectMessageTimelineView {
            items,
            next_cursor: page.next_cursor,
        })
    }

    pub async fn send_direct_message(
        &self,
        peer_pubkey: &str,
        text: Option<&str>,
        reply_to_message_id: Option<&str>,
        attachments: Vec<PendingAttachment>,
    ) -> Result<String> {
        let peer_pubkey = normalize_author_pubkey(peer_pubkey)?;
        self.ensure_author_subscription(peer_pubkey.as_str())
            .await?;
        self.rebuild_author_relationships().await?;
        if !self
            .direct_message_send_enabled(peer_pubkey.as_str())
            .await?
        {
            anyhow::bail!("direct message requires a mutual relationship");
        }
        self.ensure_direct_message_subscription(peer_pubkey.as_str())
            .await?;
        self.send_direct_message_internal(
            peer_pubkey.as_str(),
            text,
            reply_to_message_id,
            attachments,
        )
        .await
    }

    pub async fn delete_direct_message_message(
        &self,
        peer_pubkey: &str,
        message_id: &str,
    ) -> Result<()> {
        let peer_pubkey = normalize_author_pubkey(peer_pubkey)?;
        let message_id = message_id.trim();
        if message_id.is_empty() {
            anyhow::bail!("direct message message_id is required");
        }
        let dm_id = direct_message_id_for_participants(
            &Pubkey::from(self.current_author_pubkey()),
            &Pubkey::from(peer_pubkey.as_str()),
        );
        self.projection_store
            .put_direct_message_tombstone(DirectMessageTombstoneRow {
                dm_id: dm_id.clone(),
                message_id: message_id.to_string(),
                deleted_at: Utc::now().timestamp_millis(),
            })
            .await?;
        self.projection_store
            .delete_direct_message_message_local(dm_id.as_str(), message_id)
            .await?;
        self.refresh_direct_message_conversation(peer_pubkey.as_str())
            .await?;
        Ok(())
    }

    pub async fn clear_direct_message(&self, peer_pubkey: &str) -> Result<()> {
        let peer_pubkey = normalize_author_pubkey(peer_pubkey)?;
        let dm_id = direct_message_id_for_participants(
            &Pubkey::from(self.current_author_pubkey()),
            &Pubkey::from(peer_pubkey.as_str()),
        );
        let deleted_at = Utc::now().timestamp_millis();
        let mut cursor = None;
        loop {
            let page = self
                .projection_store
                .list_direct_message_messages(dm_id.as_str(), cursor.clone(), 500)
                .await?;
            for row in &page.items {
                self.projection_store
                    .put_direct_message_tombstone(DirectMessageTombstoneRow {
                        dm_id: dm_id.clone(),
                        message_id: row.message_id.clone(),
                        deleted_at,
                    })
                    .await?;
            }
            if page.next_cursor.is_none() {
                break;
            }
            cursor = page.next_cursor;
        }
        self.projection_store
            .clear_direct_message_local(dm_id.as_str())
            .await?;
        Ok(())
    }

    pub async fn get_direct_message_status(
        &self,
        peer_pubkey: &str,
    ) -> Result<DirectMessageStatusView> {
        let peer_pubkey = normalize_author_pubkey(peer_pubkey)?;
        self.ensure_author_subscription(peer_pubkey.as_str())
            .await?;
        self.rebuild_author_relationships().await?;
        if self
            .direct_message_send_enabled(peer_pubkey.as_str())
            .await?
        {
            self.ensure_direct_message_subscription(peer_pubkey.as_str())
                .await?;
        }
        self.direct_message_status_view(peer_pubkey.as_str()).await
    }

    pub async fn get_direct_message_topic_status(
        &self,
        peer_pubkey: &str,
    ) -> Result<Option<DirectMessageTopicStatusView>> {
        let peer_pubkey = normalize_author_pubkey(peer_pubkey)?;
        self.ensure_author_subscription(peer_pubkey.as_str())
            .await?;
        self.rebuild_author_relationships().await?;
        if self
            .direct_message_send_enabled(peer_pubkey.as_str())
            .await?
        {
            self.ensure_direct_message_subscription(peer_pubkey.as_str())
                .await?;
        }
        Ok(self
            .direct_message_topic_snapshot(peer_pubkey.as_str())
            .await?
            .map(|diagnostic| DirectMessageTopicStatusView {
                topic: diagnostic.topic,
                joined: diagnostic.joined,
                peer_count: diagnostic.peer_count,
                connected_peers: diagnostic.connected_peers,
                status_detail: diagnostic.status_detail,
                last_error: diagnostic.last_error,
            }))
    }
}
