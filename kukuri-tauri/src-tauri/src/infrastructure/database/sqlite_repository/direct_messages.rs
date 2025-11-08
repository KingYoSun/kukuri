use super::SqliteRepository;
use crate::application::ports::repositories::{
    DirectMessageConversationRecord, DirectMessageCursor, DirectMessageListDirection,
    DirectMessagePageRaw, DirectMessageRepository,
};
use crate::domain::entities::{DirectMessage, MessageDirection, NewDirectMessage};
use crate::shared::error::AppError;
use async_trait::async_trait;
use sqlx::sqlite::SqliteRow;
use sqlx::{Acquire, FromRow, Row};
use sqlx::{QueryBuilder, Sqlite};
use std::str::FromStr;

use super::queries::{
    INSERT_DIRECT_MESSAGE, INSERT_DM_CONVERSATION, MARK_DIRECT_MESSAGE_DELIVERED_BY_CLIENT_ID,
    MARK_DM_CONVERSATION_READ, SELECT_DIRECT_MESSAGE_BY_ID, SELECT_DM_CONVERSATIONS_BY_OWNER,
    UPDATE_DM_CONVERSATION_LAST_MESSAGE,
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
        let direction = row.direction.parse().unwrap_or(MessageDirection::Outbound);
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

#[derive(Debug, Clone)]
struct DirectMessageConversationJoinedRow {
    owner_npub: String,
    conversation_npub: String,
    last_message_id: Option<i64>,
    last_message_created_at: Option<i64>,
    last_read_at: i64,
    unread_count: i64,
    msg_id: Option<i64>,
    msg_owner_npub: Option<String>,
    msg_conversation_npub: Option<String>,
    msg_sender_npub: Option<String>,
    msg_recipient_npub: Option<String>,
    msg_event_id: Option<String>,
    msg_client_message_id: Option<String>,
    msg_payload_cipher_base64: Option<String>,
    msg_created_at: Option<i64>,
    msg_delivered: Option<i64>,
    msg_direction: Option<String>,
}

impl<'r> FromRow<'r, SqliteRow> for DirectMessageConversationJoinedRow {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            owner_npub: row.try_get("owner_npub")?,
            conversation_npub: row.try_get("conversation_npub")?,
            last_message_id: row.try_get("last_message_id")?,
            last_message_created_at: row.try_get("last_message_created_at")?,
            last_read_at: row.try_get("last_read_at")?,
            unread_count: row.try_get("unread_count")?,
            msg_id: row.try_get("msg_id")?,
            msg_owner_npub: row.try_get("msg_owner_npub")?,
            msg_conversation_npub: row.try_get("msg_conversation_npub")?,
            msg_sender_npub: row.try_get("msg_sender_npub")?,
            msg_recipient_npub: row.try_get("msg_recipient_npub")?,
            msg_event_id: row.try_get("msg_event_id")?,
            msg_client_message_id: row.try_get("msg_client_message_id")?,
            msg_payload_cipher_base64: row.try_get("msg_payload_cipher_base64")?,
            msg_created_at: row.try_get("msg_created_at")?,
            msg_delivered: row.try_get("msg_delivered")?,
            msg_direction: row.try_get("msg_direction")?,
        })
    }
}

impl DirectMessageConversationJoinedRow {
    fn into_record(self) -> DirectMessageConversationRecord {
        let last_message = self.build_message();
        DirectMessageConversationRecord {
            owner_npub: self.owner_npub,
            conversation_npub: self.conversation_npub,
            last_message,
            last_read_at: self.last_read_at,
            unread_count: self.unread_count,
        }
    }

    fn build_message(&self) -> Option<DirectMessage> {
        let (
            Some(id),
            Some(owner_npub),
            Some(conversation_npub),
            Some(sender_npub),
            Some(recipient_npub),
            Some(payload),
            Some(created_at),
            Some(direction_str),
        ) = (
            self.msg_id,
            self.msg_owner_npub.clone(),
            self.msg_conversation_npub.clone(),
            self.msg_sender_npub.clone(),
            self.msg_recipient_npub.clone(),
            self.msg_payload_cipher_base64.clone(),
            self.msg_created_at,
            self.msg_direction.clone(),
        )
        else {
            return None;
        };

        let delivered = self.msg_delivered.unwrap_or(1) != 0;
        let direction =
            MessageDirection::from_str(&direction_str).unwrap_or(MessageDirection::Outbound);

        Some(DirectMessage::new(
            id,
            owner_npub,
            conversation_npub,
            sender_npub,
            recipient_npub,
            self.msg_event_id.clone(),
            self.msg_client_message_id.clone(),
            payload,
            created_at,
            delivered,
            direction,
        ))
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

    async fn upsert_conversation_metadata(
        &self,
        owner_npub: &str,
        conversation_npub: &str,
        last_message_id: i64,
        last_message_created_at: i64,
    ) -> Result<(), AppError> {
        let mut conn = self.pool.get_pool().acquire().await?;
        let updated = sqlx::query(UPDATE_DM_CONVERSATION_LAST_MESSAGE)
            .bind(owner_npub)
            .bind(conversation_npub)
            .bind(last_message_id)
            .bind(last_message_created_at)
            .execute(&mut *conn)
            .await?;

        if updated.rows_affected() == 0 {
            sqlx::query(INSERT_DM_CONVERSATION)
                .bind(owner_npub)
                .bind(conversation_npub)
                .bind(last_message_id)
                .bind(last_message_created_at)
                .bind(0_i64)
                .execute(&mut *conn)
                .await?;
        }

        Ok(())
    }

    async fn mark_conversation_as_read(
        &self,
        owner_npub: &str,
        conversation_npub: &str,
        read_at: i64,
    ) -> Result<(), AppError> {
        let mut conn = self.pool.get_pool().acquire().await?;
        let updated = sqlx::query(MARK_DM_CONVERSATION_READ)
            .bind(owner_npub)
            .bind(conversation_npub)
            .bind(read_at)
            .execute(&mut *conn)
            .await?;

        if updated.rows_affected() == 0 {
            sqlx::query(INSERT_DM_CONVERSATION)
                .bind(owner_npub)
                .bind(conversation_npub)
                .bind(None::<i64>)
                .bind(None::<i64>)
                .bind(read_at)
                .execute(&mut *conn)
                .await?;
        }

        Ok(())
    }

    async fn list_direct_message_conversations(
        &self,
        owner_npub: &str,
        limit: usize,
    ) -> Result<Vec<DirectMessageConversationRecord>, AppError> {
        let fetch_limit = limit.max(1);
        let rows = sqlx::query_as::<_, DirectMessageConversationJoinedRow>(
            SELECT_DM_CONVERSATIONS_BY_OWNER,
        )
        .bind(owner_npub)
        .bind(fetch_limit as i64)
        .fetch_all(self.pool.get_pool())
        .await?;

        Ok(rows.into_iter().map(|row| row.into_record()).collect())
    }
}
