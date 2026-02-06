use anyhow::Result;
use cn_core::auth;
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

async fn cleanup_once(state: &AppState, retention: &RelayRetention) -> Result<()> {
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
