use anyhow::{anyhow, Result};
use cn_core::{auth, metrics, nostr, topic};
use cn_kip_types::{is_kip_kind, validate_kip_event, ValidationOptions};
use sqlx::{Postgres, Row, Transaction};

use crate::config::RelayRuntimeConfig;
use crate::AppState;

#[derive(Clone, Debug)]
pub struct RelayEvent {
    pub raw: nostr::RawEvent,
    pub topic_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IngestSource {
    Gossip,
    Ws,
}

#[derive(Clone, Debug, Default)]
pub struct IngestContext {
    pub auth_pubkey: Option<String>,
    pub source_topic: Option<String>,
    pub peer_id: Option<String>,
}

#[derive(Debug)]
pub enum IngestOutcome {
    Accepted {
        event: RelayEvent,
        duplicate: bool,
        broadcast_gossip: bool,
    },
    Rejected {
        reason: String,
    },
}

pub async fn ingest_event(
    state: &AppState,
    raw: nostr::RawEvent,
    source: IngestSource,
    context: IngestContext,
) -> Result<IngestOutcome> {
    let now = auth::unix_seconds()? as i64;
    let config_snapshot = state.config.get().await;
    let runtime = RelayRuntimeConfig::from_json(&config_snapshot.config_json);

    let raw_size = serde_json::to_vec(&raw)?.len();
    if raw_size > runtime.limits.max_event_bytes {
        metrics::inc_ingest_rejected(super::SERVICE_NAME, "invalid");
        return Ok(IngestOutcome::Rejected {
            reason: "invalid: event too large".into(),
        });
    }
    if raw.tags.len() > runtime.limits.max_tags {
        metrics::inc_ingest_rejected(super::SERVICE_NAME, "invalid");
        return Ok(IngestOutcome::Rejected {
            reason: "invalid: too many tags".into(),
        });
    }

    if let Err(err) = nostr::verify_event(&raw) {
        metrics::inc_ingest_rejected(super::SERVICE_NAME, "invalid");
        return Ok(IngestOutcome::Rejected {
            reason: format!("invalid: signature failed ({err})"),
        });
    }

    if is_kip_kind(raw.kind) {
        let options = ValidationOptions {
            now,
            verify_signature: false,
            ..ValidationOptions::default()
        };
        if let Err(err) = validate_kip_event(&raw, options) {
            metrics::inc_ingest_rejected(super::SERVICE_NAME, "invalid");
            return Ok(IngestOutcome::Rejected {
                reason: format!("invalid: kip validation failed ({err})"),
            });
        }
    }

    let mut normalized_topics = Vec::new();
    for topic_id in raw.topic_ids() {
        let normalized = topic::normalize_topic_id(&topic_id)?;
        normalized_topics.push(normalized);
    }
    if normalized_topics.is_empty() {
        metrics::inc_ingest_rejected(super::SERVICE_NAME, "invalid");
        return Ok(IngestOutcome::Rejected {
            reason: "invalid: missing topic".into(),
        });
    }

    if let Some(expected) = context.source_topic.as_ref() {
        if !normalized_topics
            .iter()
            .any(|topic_id| topic_id == expected)
        {
            metrics::inc_ingest_rejected(super::SERVICE_NAME, "invalid");
            return Ok(IngestOutcome::Rejected {
                reason: "invalid: topic mismatch".into(),
            });
        }
    }

    if source == IngestSource::Ws {
        let allowed = state.node_topics.read().await;
        if normalized_topics
            .iter()
            .any(|topic_id| !allowed.contains(topic_id))
        {
            metrics::inc_ingest_rejected(super::SERVICE_NAME, "restricted");
            return Ok(IngestOutcome::Rejected {
                reason: "restricted: topic not enabled".into(),
            });
        }
    }

    let expires_at = raw.exp_tag().or(raw.expiration_tag());
    if let Some(exp) = expires_at {
        if exp <= now {
            metrics::inc_ingest_rejected(super::SERVICE_NAME, "invalid");
            return Ok(IngestOutcome::Rejected {
                reason: "invalid: expired".into(),
            });
        }
    }

    let scope = raw
        .first_tag_value("scope")
        .unwrap_or_else(|| "public".into());
    if scope != "public" {
        let Some(epoch_value) = raw.first_tag_value("epoch") else {
            metrics::inc_ingest_rejected(super::SERVICE_NAME, "invalid");
            return Ok(IngestOutcome::Rejected {
                reason: "invalid: missing epoch".into(),
            });
        };
        let epoch: i64 = match epoch_value.parse() {
            Ok(value) => value,
            Err(_) => {
                metrics::inc_ingest_rejected(super::SERVICE_NAME, "invalid");
                return Ok(IngestOutcome::Rejected {
                    reason: "invalid: epoch must be integer".into(),
                });
            }
        };
        if epoch <= 0 {
            metrics::inc_ingest_rejected(super::SERVICE_NAME, "invalid");
            return Ok(IngestOutcome::Rejected {
                reason: "invalid: epoch must be positive".into(),
            });
        }
        // P2P-only: relay does not validate membership/epoch against node DB.
    }

    if runtime.auth.requires_auth(now) && source == IngestSource::Ws {
        let Some(auth_pubkey) = context.auth_pubkey.as_ref() else {
            metrics::inc_ingest_rejected(super::SERVICE_NAME, "auth");
            return Ok(IngestOutcome::Rejected {
                reason: "auth-required: missing auth".into(),
            });
        };
        if auth_pubkey != &raw.pubkey {
            metrics::inc_ingest_rejected(super::SERVICE_NAME, "auth");
            return Ok(IngestOutcome::Rejected {
                reason: "auth-required: pubkey mismatch".into(),
            });
        }
        if !has_current_consents(&state.pool, auth_pubkey).await? {
            metrics::inc_ingest_rejected(super::SERVICE_NAME, "consent");
            return Ok(IngestOutcome::Rejected {
                reason: "consent-required".into(),
            });
        }
        for topic_id in &normalized_topics {
            if !has_active_subscription(&state.pool, auth_pubkey, topic_id).await? {
                metrics::inc_ingest_rejected(super::SERVICE_NAME, "restricted");
                return Ok(IngestOutcome::Rejected {
                    reason: "restricted: subscription required".into(),
                });
            }
        }
    }

    metrics::inc_ingest_received(
        super::SERVICE_NAME,
        match source {
            IngestSource::Gossip => "iroh",
            IngestSource::Ws => "ws",
        },
    );

    if is_ephemeral_kind(raw.kind) {
        return Ok(IngestOutcome::Accepted {
            event: RelayEvent {
                raw,
                topic_ids: normalized_topics,
            },
            duplicate: false,
            broadcast_gossip: source == IngestSource::Ws,
        });
    }

    let mut tx = state.pool.begin().await?;
    let is_new = insert_dedupe(&mut tx, &raw.id).await?;
    if !is_new {
        metrics::inc_dedupe_hit(super::SERVICE_NAME);
        tx.commit().await?;
        return Ok(IngestOutcome::Accepted {
            event: RelayEvent {
                raw,
                topic_ids: normalized_topics,
            },
            duplicate: true,
            broadcast_gossip: false,
        });
    }
    metrics::inc_dedupe_miss(super::SERVICE_NAME);

    let kind = raw.kind as i32;
    let replaceable_key = if is_replaceable_kind(raw.kind) {
        Some(format!("{}:{}", raw.pubkey, raw.kind))
    } else {
        None
    };
    let d_tag = if is_addressable_kind(raw.kind) {
        Some(
            raw.d_tag()
                .ok_or_else(|| anyhow!("invalid: missing d tag"))?,
        )
    } else {
        None
    };
    let addressable_key = d_tag
        .as_deref()
        .map(|tag| format!("{}:{}:{}", raw.kind, raw.pubkey, tag));

    let mut is_current = true;
    if let Some(key) = replaceable_key.as_deref() {
        is_current = replaceable_is_current(
            &mut tx,
            key,
            raw.created_at,
            &raw.id,
            &raw.pubkey,
            raw.kind as i32,
        )
        .await?;
    }
    if let Some(key) = addressable_key.as_deref() {
        let d_tag = d_tag.as_deref().unwrap_or_default();
        is_current = addressable_is_current(
            &mut tx,
            key,
            raw.created_at,
            &raw.id,
            &raw.pubkey,
            raw.kind as i32,
            d_tag,
        )
        .await?;
    }

    sqlx::query(
        "INSERT INTO cn_relay.events          (event_id, pubkey, kind, created_at, tags, content, sig, raw_json, ingested_at, is_deleted, is_ephemeral, is_current, replaceable_key, addressable_key, expires_at)          VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), FALSE, FALSE, $9, $10, $11, $12)",
    )
    .bind(&raw.id)
    .bind(&raw.pubkey)
    .bind(kind)
    .bind(raw.created_at)
    .bind(serde_json::to_value(&raw.tags)?)
    .bind(&raw.content)
    .bind(&raw.sig)
    .bind(serde_json::to_value(&raw)?)
    .bind(is_current)
    .bind(replaceable_key.as_deref())
    .bind(addressable_key.as_deref())
    .bind(expires_at)
    .execute(&mut *tx)
    .await?;

    let mut topic_ids = Vec::new();
    for topic_id in normalized_topics {
        sqlx::query(
            "INSERT INTO cn_relay.event_topics (event_id, topic_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(&raw.id)
        .bind(&topic_id)
        .execute(&mut *tx)
        .await?;
        topic_ids.push(topic_id);
    }

    let tombstoned = apply_tombstones(
        &mut tx,
        &raw,
        replaceable_key.as_deref(),
        addressable_key.as_deref(),
    )
    .await?;
    if tombstoned {
        is_current = false;
    }

    let mut outbox_seqs = Vec::new();
    if is_current && !tombstoned {
        for topic_id in &topic_ids {
            outbox_seqs.push(
                insert_outbox(
                    &mut tx,
                    "upsert",
                    &raw.id,
                    topic_id,
                    kind,
                    raw.created_at,
                    replaceable_key.as_deref().or(addressable_key.as_deref()),
                    None,
                )
                .await?,
            );
        }
    } else if tombstoned {
        outbox_seqs
            .extend(insert_delete_outbox(&mut tx, &raw.id, kind, raw.created_at, "nip09").await?);
    }

    if raw.kind == 5 {
        let delete_outbox = apply_deletions(&mut tx, &raw).await?;
        outbox_seqs.extend(delete_outbox);
    }

    if let Some(max_seq) = outbox_seqs.iter().max() {
        sqlx::query("SELECT pg_notify('cn_relay_outbox', $1)")
            .bind(max_seq.to_string())
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    Ok(IngestOutcome::Accepted {
        event: RelayEvent { raw, topic_ids },
        duplicate: false,
        broadcast_gossip: source == IngestSource::Ws && is_current,
    })
}

fn is_replaceable_kind(kind: u32) -> bool {
    kind == 0 || kind == 3 || (kind >= 10000 && kind < 20000)
}

fn is_addressable_kind(kind: u32) -> bool {
    kind >= 30000 && kind < 40000
}

fn is_ephemeral_kind(kind: u32) -> bool {
    kind >= 20000 && kind < 30000
}

async fn insert_dedupe(tx: &mut Transaction<'_, Postgres>, event_id: &str) -> Result<bool> {
    let result = sqlx::query(
        "INSERT INTO cn_relay.event_dedupe (event_id, first_seen_at, last_seen_at, seen_count) VALUES ($1, NOW(), NOW(), 1) ON CONFLICT DO NOTHING",
    )
    .bind(event_id)
    .execute(&mut **tx)
    .await?;

    if result.rows_affected() == 0 {
        sqlx::query(
            "UPDATE cn_relay.event_dedupe SET last_seen_at = NOW(), seen_count = seen_count + 1 WHERE event_id = $1",
        )
        .bind(event_id)
        .execute(&mut **tx)
        .await?;
        return Ok(false);
    }
    Ok(true)
}

async fn replaceable_is_current(
    tx: &mut Transaction<'_, Postgres>,
    key: &str,
    created_at: i64,
    event_id: &str,
    pubkey: &str,
    kind: i32,
) -> Result<bool> {
    if let Some(row) = sqlx::query(
        "SELECT event_id, created_at FROM cn_relay.replaceable_current WHERE replaceable_key = $1",
    )
    .bind(key)
    .fetch_optional(&mut **tx)
    .await?
    {
        let current_id: String = row.try_get("event_id")?;
        let current_created_at: i64 = row.try_get("created_at")?;
        if !is_newer(created_at, event_id, current_created_at, &current_id) {
            return Ok(false);
        }
        sqlx::query("UPDATE cn_relay.events SET is_current = FALSE WHERE event_id = $1")
            .bind(&current_id)
            .execute(&mut **tx)
            .await?;
    }

    sqlx::query(
        "INSERT INTO cn_relay.replaceable_current          (replaceable_key, event_id, pubkey, kind, created_at, updated_at)          VALUES ($1, $2, $3, $4, $5, NOW())          ON CONFLICT (replaceable_key) DO UPDATE SET event_id = EXCLUDED.event_id, pubkey = EXCLUDED.pubkey, kind = EXCLUDED.kind, created_at = EXCLUDED.created_at, updated_at = NOW()",
    )
    .bind(key)
    .bind(event_id)
    .bind(pubkey)
    .bind(kind)
    .bind(created_at)
    .execute(&mut **tx)
    .await?;

    Ok(true)
}

async fn addressable_is_current(
    tx: &mut Transaction<'_, Postgres>,
    key: &str,
    created_at: i64,
    event_id: &str,
    pubkey: &str,
    kind: i32,
    d_tag: &str,
) -> Result<bool> {
    if let Some(row) = sqlx::query(
        "SELECT event_id, created_at FROM cn_relay.addressable_current WHERE addressable_key = $1",
    )
    .bind(key)
    .fetch_optional(&mut **tx)
    .await?
    {
        let current_id: String = row.try_get("event_id")?;
        let current_created_at: i64 = row.try_get("created_at")?;
        if !is_newer(created_at, event_id, current_created_at, &current_id) {
            return Ok(false);
        }
        sqlx::query("UPDATE cn_relay.events SET is_current = FALSE WHERE event_id = $1")
            .bind(&current_id)
            .execute(&mut **tx)
            .await?;
    }

    sqlx::query(
        "INSERT INTO cn_relay.addressable_current          (addressable_key, event_id, pubkey, kind, d_tag, created_at, updated_at)          VALUES ($1, $2, $3, $4, $5, $6, NOW())          ON CONFLICT (addressable_key) DO UPDATE SET event_id = EXCLUDED.event_id, pubkey = EXCLUDED.pubkey, kind = EXCLUDED.kind, d_tag = EXCLUDED.d_tag, created_at = EXCLUDED.created_at, updated_at = NOW()",
    )
    .bind(key)
    .bind(event_id)
    .bind(pubkey)
    .bind(kind)
    .bind(d_tag)
    .bind(created_at)
    .execute(&mut **tx)
    .await?;

    Ok(true)
}

fn is_newer(new_created_at: i64, new_id: &str, current_created_at: i64, current_id: &str) -> bool {
    if new_created_at > current_created_at {
        return true;
    }
    if new_created_at < current_created_at {
        return false;
    }
    new_id < current_id
}

async fn apply_deletions(
    tx: &mut Transaction<'_, Postgres>,
    deletion: &nostr::RawEvent,
) -> Result<Vec<i64>> {
    let mut outbox_seqs = Vec::new();
    let targets = deletion.tag_values("e");
    for event_id in targets {
        let row = sqlx::query(
            "SELECT event_id, pubkey, kind, created_at, replaceable_key, addressable_key FROM cn_relay.events WHERE event_id = $1",
        )
        .bind(&event_id)
        .fetch_optional(&mut **tx)
        .await?;
        let Some(row) = row else {
            insert_tombstone_event(tx, &event_id, &deletion.id, deletion.created_at).await?;
            continue;
        };

        let pubkey: String = row.try_get("pubkey")?;
        if pubkey != deletion.pubkey {
            continue;
        }
        let kind: i32 = row.try_get("kind")?;
        let created_at: i64 = row.try_get("created_at")?;
        let replaceable_key: Option<String> = row.try_get("replaceable_key")?;
        let addressable_key: Option<String> = row.try_get("addressable_key")?;
        mark_deleted(tx, &event_id).await?;
        if let Some(key) = replaceable_key {
            sqlx::query("DELETE FROM cn_relay.replaceable_current WHERE replaceable_key = $1")
                .bind(&key)
                .execute(&mut **tx)
                .await?;
        }
        if let Some(key) = addressable_key {
            sqlx::query("DELETE FROM cn_relay.addressable_current WHERE addressable_key = $1")
                .bind(&key)
                .execute(&mut **tx)
                .await?;
        }
        outbox_seqs.extend(insert_delete_outbox(tx, &event_id, kind, created_at, "nip09").await?);
    }

    for target in deletion.tag_values("a") {
        if let Some(seq) = apply_addressable_delete(tx, deletion, &target).await? {
            outbox_seqs.extend(seq);
        }
    }

    Ok(outbox_seqs)
}

async fn apply_addressable_delete(
    tx: &mut Transaction<'_, Postgres>,
    deletion: &nostr::RawEvent,
    target: &str,
) -> Result<Option<Vec<i64>>> {
    let parts: Vec<&str> = target.split(':').collect();
    if parts.len() < 3 {
        return Ok(None);
    }
    let kind: i32 = parts[0].parse().unwrap_or_default();
    let pubkey = parts[1];
    if pubkey != deletion.pubkey {
        return Ok(None);
    }
    let d_tag = parts[2..].join(":");
    let key = format!("{kind}:{pubkey}:{d_tag}");
    let row =
        sqlx::query("SELECT event_id FROM cn_relay.addressable_current WHERE addressable_key = $1")
            .bind(&key)
            .fetch_optional(&mut **tx)
            .await?;
    let Some(row) = row else {
        insert_tombstone_addressable(tx, &key, &deletion.id, deletion.created_at).await?;
        return Ok(None);
    };
    let event_id: String = row.try_get("event_id")?;
    let event_row = sqlx::query("SELECT kind, created_at FROM cn_relay.events WHERE event_id = $1")
        .bind(&event_id)
        .fetch_one(&mut **tx)
        .await?;
    let kind: i32 = event_row.try_get("kind")?;
    let created_at: i64 = event_row.try_get("created_at")?;
    mark_deleted(tx, &event_id).await?;
    sqlx::query("DELETE FROM cn_relay.addressable_current WHERE addressable_key = $1")
        .bind(&key)
        .execute(&mut **tx)
        .await?;
    let seqs = insert_delete_outbox(tx, &event_id, kind, created_at, "nip09").await?;
    Ok(Some(seqs))
}

async fn mark_deleted(tx: &mut Transaction<'_, Postgres>, event_id: &str) -> Result<()> {
    sqlx::query(
        "UPDATE cn_relay.events SET is_deleted = TRUE, deleted_at = NOW(), is_current = FALSE WHERE event_id = $1",
    )
    .bind(event_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_delete_outbox(
    tx: &mut Transaction<'_, Postgres>,
    event_id: &str,
    kind: i32,
    created_at: i64,
    reason: &str,
) -> Result<Vec<i64>> {
    let topic_rows = sqlx::query("SELECT topic_id FROM cn_relay.event_topics WHERE event_id = $1")
        .bind(event_id)
        .fetch_all(&mut **tx)
        .await?;
    let mut seqs = Vec::new();
    for row in topic_rows {
        let topic_id: String = row.try_get("topic_id")?;
        seqs.push(
            insert_outbox(
                tx,
                "delete",
                event_id,
                &topic_id,
                kind,
                created_at,
                None,
                Some(reason),
            )
            .await?,
        );
    }
    Ok(seqs)
}

async fn insert_outbox(
    tx: &mut Transaction<'_, Postgres>,
    op: &str,
    event_id: &str,
    topic_id: &str,
    kind: i32,
    created_at: i64,
    effective_key: Option<&str>,
    reason: Option<&str>,
) -> Result<i64> {
    let row = sqlx::query(
        "INSERT INTO cn_relay.events_outbox          (op, event_id, topic_id, kind, created_at, ingested_at, effective_key, reason)          VALUES ($1, $2, $3, $4, $5, NOW(), $6, $7)          RETURNING seq",
    )
    .bind(op)
    .bind(event_id)
    .bind(topic_id)
    .bind(kind)
    .bind(created_at)
    .bind(effective_key)
    .bind(reason)
    .fetch_one(&mut **tx)
    .await?;
    let seq: i64 = row.try_get("seq")?;
    Ok(seq)
}

async fn insert_tombstone_event(
    tx: &mut Transaction<'_, Postgres>,
    event_id: &str,
    deletion_event_id: &str,
    requested_at: i64,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO cn_relay.deletion_tombstones          (target_event_id, deletion_event_id, requested_at)          VALUES ($1, $2, $3)",
    )
    .bind(event_id)
    .bind(deletion_event_id)
    .bind(requested_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_tombstone_addressable(
    tx: &mut Transaction<'_, Postgres>,
    target_a: &str,
    deletion_event_id: &str,
    requested_at: i64,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO cn_relay.deletion_tombstones          (target_a, deletion_event_id, requested_at)          VALUES ($1, $2, $3)",
    )
    .bind(target_a)
    .bind(deletion_event_id)
    .bind(requested_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn apply_tombstones(
    tx: &mut Transaction<'_, Postgres>,
    raw: &nostr::RawEvent,
    replaceable_key: Option<&str>,
    addressable_key: Option<&str>,
) -> Result<bool> {
    let mut deleted = false;
    let tombstones = sqlx::query(
        "SELECT deletion_event_id FROM cn_relay.deletion_tombstones WHERE target_event_id = $1 AND applied_at IS NULL",
    )
    .bind(&raw.id)
    .fetch_all(&mut **tx)
    .await?;
    for tombstone in tombstones {
        let deletion_event_id: String = tombstone.try_get("deletion_event_id")?;
        let pubkey = sqlx::query_scalar::<_, String>(
            "SELECT pubkey FROM cn_relay.events WHERE event_id = $1",
        )
        .bind(&deletion_event_id)
        .fetch_optional(&mut **tx)
        .await?;
        if let Some(pubkey) = pubkey {
            if pubkey == raw.pubkey {
                mark_deleted(tx, &raw.id).await?;
                deleted = true;
                sqlx::query(
                    "UPDATE cn_relay.deletion_tombstones SET applied_at = NOW() WHERE deletion_event_id = $1 AND target_event_id = $2",
                )
                .bind(&deletion_event_id)
                .bind(&raw.id)
                .execute(&mut **tx)
                .await?;
            }
        }
    }
    if let Some(key) = addressable_key {
        let tombstones = sqlx::query(
            "SELECT deletion_event_id FROM cn_relay.deletion_tombstones WHERE target_a = $1 AND applied_at IS NULL",
        )
        .bind(key)
        .fetch_all(&mut **tx)
        .await?;
        for tombstone in tombstones {
            let deletion_event_id: String = tombstone.try_get("deletion_event_id")?;
            let pubkey = sqlx::query_scalar::<_, String>(
                "SELECT pubkey FROM cn_relay.events WHERE event_id = $1",
            )
            .bind(&deletion_event_id)
            .fetch_optional(&mut **tx)
            .await?;
            if let Some(pubkey) = pubkey {
                if pubkey == raw.pubkey {
                    mark_deleted(tx, &raw.id).await?;
                    deleted = true;
                    sqlx::query(
                        "UPDATE cn_relay.deletion_tombstones SET applied_at = NOW() WHERE deletion_event_id = $1 AND target_a = $2",
                    )
                    .bind(&deletion_event_id)
                    .bind(key)
                    .execute(&mut **tx)
                    .await?;
                }
            }
        }
    }
    if deleted {
        if let Some(key) = replaceable_key {
            sqlx::query("DELETE FROM cn_relay.replaceable_current WHERE replaceable_key = $1")
                .bind(key)
                .execute(&mut **tx)
                .await?;
        }
        if let Some(key) = addressable_key {
            sqlx::query("DELETE FROM cn_relay.addressable_current WHERE addressable_key = $1")
                .bind(key)
                .execute(&mut **tx)
                .await?;
        }
    }
    Ok(deleted)
}

pub(crate) async fn has_current_consents(
    pool: &sqlx::Pool<Postgres>,
    pubkey: &str,
) -> Result<bool> {
    let missing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cn_admin.policies p          LEFT JOIN cn_user.policy_consents c            ON c.policy_id = p.policy_id AND c.accepter_pubkey = $1          WHERE p.is_current = TRUE AND p.type IN ('terms','privacy') AND c.policy_id IS NULL",
    )
    .bind(pubkey)
    .fetch_one(pool)
    .await?;
    Ok(missing == 0)
}

async fn has_active_subscription(
    pool: &sqlx::Pool<Postgres>,
    pubkey: &str,
    topic_id: &str,
) -> Result<bool> {
    let status = sqlx::query_scalar::<_, String>(
        "SELECT status FROM cn_user.topic_subscriptions WHERE topic_id = $1 AND subscriber_pubkey = $2",
    )
    .bind(topic_id)
    .bind(pubkey)
    .fetch_optional(pool)
    .await?;
    Ok(status.map(|s| s == "active").unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cn_core::rate_limit::RateLimiter;
    use cn_core::service_config;
    use cn_kip_types::KIND_NODE_TOPIC_SERVICE;
    use nostr_sdk::prelude::Keys;
    use serde_json::json;
    use sqlx::postgres::PgPoolOptions;
    use std::collections::{HashMap, HashSet};
    use std::sync::Arc;
    use tokio::sync::{broadcast, RwLock};

    fn test_state() -> AppState {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost/postgres")
            .expect("lazy pool");
        let config = service_config::static_handle(json!({
            "auth": {
                "mode": "off",
                "enforce_at": null,
                "grace_seconds": 900,
                "ws_auth_timeout_seconds": 10
            },
            "limits": {
                "max_event_bytes": 32768,
                "max_tags": 200
            }
        }));
        let (realtime_tx, _) = broadcast::channel(8);
        AppState {
            pool,
            config,
            rate_limiter: Arc::new(RateLimiter::new()),
            realtime_tx,
            gossip_senders: Arc::new(RwLock::new(HashMap::new())),
            node_topics: Arc::new(RwLock::new(HashSet::new())),
            relay_public_url: None,
        }
    }

    fn build_invalid_kip_event() -> nostr::RawEvent {
        let keys = Keys::generate();
        let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;
        let tags = vec![
            vec![
                "d".to_string(),
                "topic_service:kukuri:topic1:index:private".to_string(),
            ],
            vec!["t".to_string(), "kukuri:topic1".to_string()],
            vec!["role".to_string(), "index".to_string()],
            vec!["scope".to_string(), "private".to_string()],
            vec!["k".to_string(), "kukuri".to_string()],
            vec!["ver".to_string(), "1".to_string()],
            vec!["exp".to_string(), (now + 3600).to_string()],
        ];
        let content = json!({
            "schema": "kukuri-topic-service-v1",
            "topic": "kukuri:topic1",
            "role": "index",
            "scope": "private"
        })
        .to_string();
        nostr::build_signed_event(&keys, KIND_NODE_TOPIC_SERVICE as u16, tags, content)
            .expect("event")
    }

    fn build_ephemeral_event(scope: &str, epoch: Option<&str>) -> nostr::RawEvent {
        let keys = Keys::generate();
        let mut tags = vec![vec!["t".to_string(), "kukuri:topic1".to_string()]];
        tags.push(vec!["scope".to_string(), scope.to_string()]);
        if let Some(epoch) = epoch {
            tags.push(vec!["epoch".to_string(), epoch.to_string()]);
        }
        nostr::build_signed_event(&keys, 20001, tags, "hello".to_string()).expect("event")
    }

    #[tokio::test]
    async fn ingest_event_rejects_invalid_kip_event() {
        let state = test_state();
        let raw = build_invalid_kip_event();

        let outcome = ingest_event(&state, raw, IngestSource::Gossip, IngestContext::default())
            .await
            .expect("ingest result");

        match outcome {
            IngestOutcome::Rejected { reason } => {
                assert!(reason.contains("kip validation failed"));
            }
            _ => panic!("expected rejection for invalid kip event"),
        }
    }

    #[tokio::test]
    async fn ingest_event_accepts_private_scope_without_membership() {
        let state = test_state();
        let raw = build_ephemeral_event("friend", Some("1"));

        let outcome = ingest_event(&state, raw, IngestSource::Gossip, IngestContext::default())
            .await
            .expect("ingest result");

        match outcome {
            IngestOutcome::Accepted { .. } => {}
            _ => panic!("expected acceptance for private scope event"),
        }
    }

    #[tokio::test]
    async fn ingest_event_rejects_private_scope_missing_epoch() {
        let state = test_state();
        let raw = build_ephemeral_event("friend", None);

        let outcome = ingest_event(&state, raw, IngestSource::Gossip, IngestContext::default())
            .await
            .expect("ingest result");

        match outcome {
            IngestOutcome::Rejected { reason } => {
                assert!(reason.contains("missing epoch"));
            }
            _ => panic!("expected rejection for missing epoch"),
        }
    }

    #[tokio::test]
    async fn ingest_event_rejects_private_scope_invalid_epoch() {
        let state = test_state();
        let raw = build_ephemeral_event("friend", Some("nope"));

        let outcome = ingest_event(&state, raw, IngestSource::Gossip, IngestContext::default())
            .await
            .expect("ingest result");

        match outcome {
            IngestOutcome::Rejected { reason } => {
                assert!(reason.contains("epoch must be integer"));
            }
            _ => panic!("expected rejection for invalid epoch"),
        }
    }
}
