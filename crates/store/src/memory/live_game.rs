use super::*;

impl MemoryStore {
    pub(super) async fn projection_upsert_live_session_cache_impl(
        &self,
        row: LiveSessionProjectionRow,
    ) -> Result<()> {
        self.live_session_rows
            .write()
            .await
            .insert(row.session_id.clone(), row);
        Ok(())
    }

    pub(super) async fn projection_list_topic_live_sessions_impl(
        &self,
        topic_id: &str,
    ) -> Result<Vec<LiveSessionProjectionRow>> {
        let presence = self.live_presence.read().await;
        let mut items = self
            .live_session_rows
            .read()
            .await
            .values()
            .filter(|row| row.topic_id == topic_id)
            .cloned()
            .collect::<Vec<_>>();
        for row in &mut items {
            row.viewer_count = if row.status == LiveSessionStatus::Ended {
                0
            } else {
                // sqlite の viewer_count 相関サブクエリ(topic_id, channel_id, session_id
                // が行と一致する live_presence_cache の COUNT)と同義
                presence
                    .iter()
                    .filter(
                        |((presence_topic, presence_channel, presence_session, _), _)| {
                            presence_topic == &row.topic_id
                                && presence_channel == &row.channel_id
                                && presence_session == &row.session_id
                        },
                    )
                    .count()
            };
        }
        items.sort_by(|left, right| {
            right
                .started_at
                .cmp(&left.started_at)
                .then_with(|| right.session_id.cmp(&left.session_id))
        });
        Ok(items)
    }

    pub(super) async fn projection_upsert_game_room_cache_impl(
        &self,
        row: GameRoomProjectionRow,
    ) -> Result<()> {
        self.game_room_rows
            .write()
            .await
            .insert(row.room_id.clone(), row);
        Ok(())
    }

    pub(super) async fn projection_list_topic_game_rooms_impl(
        &self,
        topic_id: &str,
    ) -> Result<Vec<GameRoomProjectionRow>> {
        let mut items = self
            .game_room_rows
            .read()
            .await
            .values()
            .filter(|row| row.topic_id == topic_id)
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| right.room_id.cmp(&left.room_id))
        });
        Ok(items)
    }

    pub(super) async fn projection_upsert_live_presence_impl(
        &self,
        topic_id: &str,
        channel_id: &str,
        session_id: &str,
        author_pubkey: &str,
        expires_at: i64,
        updated_at: i64,
    ) -> Result<()> {
        // キーは sqlite の ON CONFLICT(topic_id, channel_id, session_id, author_pubkey)
        // と同義(topic_id を含めないと別 topic の presence を上書きしてしまう)
        self.live_presence.write().await.insert(
            (
                topic_id.to_string(),
                channel_id.to_string(),
                session_id.to_string(),
                author_pubkey.to_string(),
            ),
            (expires_at, updated_at),
        );
        Ok(())
    }

    pub(super) async fn projection_clear_expired_live_presence_impl(
        &self,
        now_ms: i64,
    ) -> Result<()> {
        self.live_presence
            .write()
            .await
            .retain(|_, (expires_at, _)| *expires_at > now_ms);
        Ok(())
    }

    pub(super) async fn projection_clear_topic_live_presence_impl(
        &self,
        topic_id: &str,
    ) -> Result<()> {
        self.live_presence
            .write()
            .await
            .retain(|(presence_topic, _, _, _), _| presence_topic != topic_id);
        Ok(())
    }
}
