use anyhow::Result;
use cn_core::auth;
use sqlx::Row;
use std::time::Duration;

use crate::config::{RelayRetention, RelayRuntimeConfig};
use crate::AppState;

pub fn spawn_cleanup_loop(state: AppState) {
    tokio::spawn(async move {
        loop {
            let snapshot = state.config.get().await;
            let runtime = RelayRuntimeConfig::from_json(&snapshot.config_json);
            if let Err(err) = cleanup_once(&state, &runtime.retention).await {
                tracing::warn!(error = %err, "relay retention cleanup failed");
            }
            let interval = runtime.retention.cleanup_interval_seconds.max(60);
            tokio::time::sleep(Duration::from_secs(interval)).await;
        }
    });
}

pub(crate) async fn cleanup_once(state: &AppState, retention: &RelayRetention) -> Result<()> {
    expire_events(state).await?;

    if retention.events_days > 0 {
        let mut tx = state.pool.begin().await?;
        let topic_result = sqlx::query(
            "DELETE FROM cn_relay.event_topics          WHERE event_id IN (              SELECT event_id FROM cn_relay.events              WHERE is_current = FALSE                AND ingested_at < NOW() - ($1 * INTERVAL '1 day')          )",
        )
        .bind(retention.events_days)
        .execute(&mut *tx)
        .await?;
        let event_result = sqlx::query(
            "DELETE FROM cn_relay.events          WHERE is_current = FALSE            AND ingested_at < NOW() - ($1 * INTERVAL '1 day')",
        )
        .bind(retention.events_days)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        tracing::info!(
            removed_event_topics = topic_result.rows_affected(),
            removed_events = event_result.rows_affected(),
            "relay retention cleanup: events"
        );
    }

    if retention.dedupe_days > 0 {
        let result = sqlx::query(
            "DELETE FROM cn_relay.event_dedupe          WHERE last_seen_at < NOW() - ($1 * INTERVAL '1 day')",
        )
        .bind(retention.dedupe_days)
        .execute(&state.pool)
        .await?;
        tracing::info!(
            removed_dedupe = result.rows_affected(),
            "relay retention cleanup: dedupe"
        );
    }

    if retention.outbox_days > 0 {
        let result = sqlx::query(
            "DELETE FROM cn_relay.events_outbox          WHERE ingested_at < NOW() - ($1 * INTERVAL '1 day')",
        )
        .bind(retention.outbox_days)
        .execute(&state.pool)
        .await?;
        tracing::info!(
            removed_outbox = result.rows_affected(),
            "relay retention cleanup: outbox"
        );
    }

    if retention.tombstone_days > 0 {
        let now = auth::unix_seconds()? as i64;
        let cutoff = now.saturating_sub(retention.tombstone_days.saturating_mul(86400));
        let result =
            sqlx::query("DELETE FROM cn_relay.deletion_tombstones WHERE requested_at < $1")
                .bind(cutoff)
                .execute(&state.pool)
                .await?;
        tracing::info!(
            removed_tombstones = result.rows_affected(),
            "relay retention cleanup: tombstones"
        );
    }

    Ok(())
}

async fn expire_events(state: &AppState) -> Result<()> {
    loop {
        let expired = expire_events_batch(state).await?;
        if expired == 0 {
            break;
        }
    }
    Ok(())
}

async fn expire_events_batch(state: &AppState) -> Result<usize> {
    let now = auth::unix_seconds()? as i64;
    let mut tx = state.pool.begin().await?;
    let rows = sqlx::query(
        "SELECT event_id, kind, created_at, replaceable_key, addressable_key \
         FROM cn_relay.events \
         WHERE expires_at IS NOT NULL \
           AND expires_at <= $1 \
           AND is_deleted = FALSE \
           AND is_ephemeral = FALSE \
           AND is_current = TRUE \
         ORDER BY created_at ASC, event_id ASC \
         LIMIT 200 \
         FOR UPDATE SKIP LOCKED",
    )
    .bind(now)
    .fetch_all(&mut *tx)
    .await?;
    if rows.is_empty() {
        tx.commit().await?;
        return Ok(0);
    }

    let mut max_seq: Option<i64> = None;
    for row in &rows {
        let event_id: String = row.try_get("event_id")?;
        let kind: i32 = row.try_get("kind")?;
        let created_at: i64 = row.try_get("created_at")?;
        let replaceable_key: Option<String> = row.try_get("replaceable_key")?;
        let addressable_key: Option<String> = row.try_get("addressable_key")?;

        sqlx::query(
            "UPDATE cn_relay.events \
             SET is_deleted = TRUE, deleted_at = NOW(), is_current = FALSE \
             WHERE event_id = $1",
        )
        .bind(&event_id)
        .execute(&mut *tx)
        .await?;

        if let Some(key) = replaceable_key {
            sqlx::query("DELETE FROM cn_relay.replaceable_current WHERE replaceable_key = $1")
                .bind(&key)
                .execute(&mut *tx)
                .await?;
        }
        if let Some(key) = addressable_key {
            sqlx::query("DELETE FROM cn_relay.addressable_current WHERE addressable_key = $1")
                .bind(&key)
                .execute(&mut *tx)
                .await?;
        }

        let topic_rows =
            sqlx::query("SELECT topic_id FROM cn_relay.event_topics WHERE event_id = $1")
                .bind(&event_id)
                .fetch_all(&mut *tx)
                .await?;
        for topic_row in topic_rows {
            let topic_id: String = topic_row.try_get("topic_id")?;
            let seq: i64 = sqlx::query_scalar(
                "INSERT INTO cn_relay.events_outbox \
                 (op, event_id, topic_id, kind, created_at, ingested_at, effective_key, reason) \
                 VALUES ('delete', $1, $2, $3, $4, NOW(), NULL, 'expiration') \
                 RETURNING seq",
            )
            .bind(&event_id)
            .bind(&topic_id)
            .bind(kind)
            .bind(created_at)
            .fetch_one(&mut *tx)
            .await?;
            max_seq = Some(max_seq.map_or(seq, |current| current.max(seq)));
        }
    }

    if let Some(seq) = max_seq {
        sqlx::query("SELECT pg_notify('cn_relay_outbox', $1)")
            .bind(seq.to_string())
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(rows.len())
}
