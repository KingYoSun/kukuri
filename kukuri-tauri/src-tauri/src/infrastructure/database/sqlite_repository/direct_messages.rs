use super::SqliteRepository;
use crate::application::ports::repositories::{
    DirectMessageCursor, DirectMessageListDirection, DirectMessagePageRaw, DirectMessageRepository,
};
use crate::domain::entities::{DirectMessage, MessageDirection, NewDirectMessage};
use crate::shared::error::AppError;
use async_trait::async_trait;
use sqlx::sqlite::SqliteRow;
use sqlx::{Acquire, FromRow, Row};
use sqlx::{QueryBuilder, Sqlite};

use super::queries::{
    INSERT_DIRECT_MESSAGE, MARK_DIRECT_MESSAGE_DELIVERED_BY_CLIENT_ID, SELECT_DIRECT_MESSAGE_BY_ID,
};

#[derive(Debug, Clone)]
struct DirectMessageRow {
    id: i64,
    owner_npub: String,
    conversation_npub: String,
    sender_npub: String,
    recipient_npub: String,
    event_id: Option<String>,
    client_message_id: Option<String>,
    payload_cipher_base64: String,
    created_at: i64,
    delivered: i64,
    direction: String,
}

impl<'r> FromRow<'r, SqliteRow> for DirectMessageRow {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            owner_npub: row.try_get("owner_npub")?,
            conversation_npub: row.try_get("conversation_npub")?,
            sender_npub: row.try_get("sender_npub")?,
            recipient_npub: row.try_get("recipient_npub")?,
            event_id: row.try_get("event_id")?,
            client_message_id: row.try_get("client_message_id")?,
            payload_cipher_base64: row.try_get("payload_cipher_base64")?,
            created_at: row.try_get("created_at")?,
            delivered: row.try_get("delivered")?,
            direction: row.try_get("direction")?,
        })
    }
}

impl From<DirectMessageRow> for DirectMessage {
    fn from(row: DirectMessageRow) -> Self {
        let direction =
            MessageDirection::from_str(&row.direction).unwrap_or(MessageDirection::Outbound);
        let delivered = row.delivered != 0;
        DirectMessage::new(
            row.id,
            row.owner_npub,
            row.conversation_npub,
            row.sender_npub,
            row.recipient_npub,
            row.event_id,
            row.client_message_id,
            row.payload_cipher_base64,
            row.created_at,
            delivered,
            direction,
        )
    }
}

#[async_trait]
impl DirectMessageRepository for SqliteRepository {
    async fn insert_direct_message(
        &self,
        message: &NewDirectMessage,
    ) -> Result<DirectMessage, AppError> {
        let mut conn = self.pool.get_pool().acquire().await?;
        let mut tx = conn.begin().await?;

        sqlx::query(INSERT_DIRECT_MESSAGE)
            .bind(&message.owner_npub)
            .bind(&message.conversation_npub)
            .bind(&message.sender_npub)
            .bind(&message.recipient_npub)
            .bind(&message.event_id)
            .bind(&message.client_message_id)
            .bind(&message.payload_cipher_base64)
            .bind(message.created_at.timestamp_millis())
            .bind(if message.delivered { 1 } else { 0 })
            .bind(message.direction.as_str())
            .execute(&mut *tx)
            .await?;

        let inserted_id: i64 = sqlx::query_scalar("SELECT last_insert_rowid()")
            .fetch_one(&mut *tx)
            .await?;

        let row = sqlx::query_as::<_, DirectMessageRow>(SELECT_DIRECT_MESSAGE_BY_ID)
            .bind(inserted_id)
            .fetch_one(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(row.into())
    }

    async fn list_direct_messages(
        &self,
        owner_npub: &str,
        conversation_npub: &str,
        cursor: Option<DirectMessageCursor>,
        limit: usize,
        direction: DirectMessageListDirection,
    ) -> Result<DirectMessagePageRaw, AppError> {
        let fetch_limit = limit.saturating_add(1);

        let mut builder = QueryBuilder::<Sqlite>::new(
            "SELECT id,
                    owner_npub,
                    conversation_npub,
                    sender_npub,
                    recipient_npub,
                    event_id,
                    client_message_id,
                    payload_cipher_base64,
                    created_at,
                    delivered,
                    direction
             FROM direct_messages
             WHERE owner_npub = ",
        );

        builder.push_bind(owner_npub);
        builder.push(" AND conversation_npub = ");
        builder.push_bind(conversation_npub);

        if let Some(cur) = cursor.as_ref() {
            match direction {
                DirectMessageListDirection::Backward => {
                    builder.push(" AND (created_at < ");
                    builder.push_bind(cur.created_at);
                    builder.push(" OR (created_at = ");
                    builder.push_bind(cur.created_at);
                    builder.push(" AND IFNULL(event_id, '') < ");
                    builder.push_bind(cur.event_id.clone().unwrap_or_default());
                    builder.push("))");
                }
                DirectMessageListDirection::Forward => {
                    builder.push(" AND (created_at > ");
                    builder.push_bind(cur.created_at);
                    builder.push(" OR (created_at = ");
                    builder.push_bind(cur.created_at);
                    builder.push(" AND IFNULL(event_id, '') > ");
                    builder.push_bind(cur.event_id.clone().unwrap_or_default());
                    builder.push("))");
                }
            }
        }

        builder.push(" ORDER BY created_at ");
        match direction {
            DirectMessageListDirection::Backward => {
                builder.push("DESC, IFNULL(event_id, '') DESC");
            }
            DirectMessageListDirection::Forward => {
                builder.push("ASC, IFNULL(event_id, '') ASC");
            }
        }

        builder.push(" LIMIT ");
        builder.push_bind(fetch_limit as i64);

        let query = builder.build_query_as::<DirectMessageRow>();
        let mut rows = query.fetch_all(self.pool.get_pool()).await?;

        let has_more = rows.len() > limit;
        if has_more {
            rows.truncate(limit);
        }

        let next_cursor = rows.last().map(|row| {
            let dm: DirectMessage = row.clone().into();
            dm.cursor()
        });

        let items = rows.into_iter().map(Into::into).collect();

        Ok(DirectMessagePageRaw {
            items,
            next_cursor,
            has_more,
        })
    }

    async fn mark_delivered_by_client_id(
        &self,
        owner_npub: &str,
        client_message_id: &str,
        event_id: Option<String>,
        delivered: bool,
    ) -> Result<(), AppError> {
        let mut conn = self.pool.get_pool().acquire().await?;
        sqlx::query(MARK_DIRECT_MESSAGE_DELIVERED_BY_CLIENT_ID)
            .bind(owner_npub)
            .bind(client_message_id)
            .bind(event_id.as_deref())
            .bind(if delivered { 1 } else { 0 })
            .execute(&mut *conn)
            .await?;
        Ok(())
    }
}
