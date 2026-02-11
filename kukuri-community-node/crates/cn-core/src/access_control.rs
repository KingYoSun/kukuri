use anyhow::{anyhow, Result};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use nostr_sdk::prelude::{nip44, Keys, PublicKey};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{Pool, Postgres, Row, Transaction};

use crate::{auth, nostr, topic};

const KIP_NAMESPACE: &str = "kukuri";
const KIP_VERSION: &str = "1";
const KIND_KEY_ENVELOPE: u32 = 39020;
pub const DISTRIBUTION_STATUS_PENDING: &str = "pending";
pub const DISTRIBUTION_STATUS_SUCCESS: &str = "success";
pub const DISTRIBUTION_STATUS_FAILED: &str = "failed";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionResult {
    pub recipient_pubkey: String,
    pub status: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotateSummary {
    pub topic_id: String,
    pub scope: String,
    pub previous_epoch: i64,
    pub new_epoch: i64,
    pub recipients: usize,
    pub distribution_results: Vec<DistributionResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeSummary {
    pub topic_id: String,
    pub scope: String,
    pub revoked_pubkey: String,
    pub rotation: RotateSummary,
}

pub fn normalize_scope(scope: &str) -> Result<String> {
    let normalized = scope.trim().to_lowercase();
    match normalized.as_str() {
        "friend" | "invite" | "friend_plus" => Ok(normalized),
        _ => Err(anyhow!("invalid scope: {scope}")),
    }
}

pub fn normalize_distribution_status(status: &str) -> Result<String> {
    let normalized = status.trim().to_lowercase();
    match normalized.as_str() {
        DISTRIBUTION_STATUS_PENDING | DISTRIBUTION_STATUS_SUCCESS | DISTRIBUTION_STATUS_FAILED => {
            Ok(normalized)
        }
        _ => Err(anyhow!("invalid distribution status: {status}")),
    }
}

pub fn normalize_pubkey(pubkey: &str) -> Result<String> {
    let trimmed = pubkey.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("pubkey is empty"));
    }
    let parsed = PublicKey::from_hex(trimmed).map_err(|_| anyhow!("invalid pubkey"))?;
    Ok(parsed.to_hex())
}

pub fn build_key_envelope_event(
    node_keys: &Keys,
    recipient_pubkey: &str,
    topic_id: &str,
    scope: &str,
    epoch: i64,
    key_b64: &str,
) -> Result<nostr::RawEvent> {
    if epoch <= 0 {
        return Err(anyhow!("epoch must be positive"));
    }

    let recipient = PublicKey::from_hex(recipient_pubkey).map_err(|_| anyhow!("invalid pubkey"))?;
    let issued_at = auth::unix_seconds()? as i64;
    let payload = json!({
        "schema": "kukuri-key-envelope-v1",
        "topic": topic_id,
        "scope": scope,
        "epoch": epoch,
        "key_b64": key_b64,
        "issued_at": issued_at
    });
    let encrypted = nip44::encrypt(
        node_keys.secret_key(),
        &recipient,
        payload.to_string(),
        nip44::Version::V2,
    )
    .map_err(|err| anyhow!("key encrypt failed: {err}"))?;

    let d_tag = format!("keyenv:{topic_id}:{scope}:{epoch}:{recipient_pubkey}");
    let tags = vec![
        vec!["p".to_string(), recipient_pubkey.to_string()],
        vec!["t".to_string(), topic_id.to_string()],
        vec!["scope".to_string(), scope.to_string()],
        vec!["epoch".to_string(), epoch.to_string()],
        vec!["k".to_string(), KIP_NAMESPACE.to_string()],
        vec!["ver".to_string(), KIP_VERSION.to_string()],
        vec!["d".to_string(), d_tag],
    ];

    nostr::build_signed_event(node_keys, KIND_KEY_ENVELOPE as u16, tags, encrypted)
}

pub async fn load_or_create_group_key(
    tx: &mut Transaction<'_, Postgres>,
    node_keys: &Keys,
    topic_id: &str,
    scope: &str,
    epoch: i64,
) -> Result<String> {
    let row = sqlx::query(
        "SELECT key_ciphertext FROM cn_admin.topic_scope_keys WHERE topic_id = $1 AND scope = $2 AND epoch = $3",
    )
    .bind(topic_id)
    .bind(scope)
    .bind(epoch as i32)
    .fetch_optional(&mut **tx)
    .await?;

    if let Some(row) = row {
        let ciphertext: String = row.try_get("key_ciphertext")?;
        let plain = nip44::decrypt(node_keys.secret_key(), &node_keys.public_key(), ciphertext)
            .map_err(|err| anyhow!("key decrypt failed: {err}"))?;
        return Ok(plain);
    }

    let mut bytes = [0u8; 32];
    let mut rng = OsRng;
    rng.fill_bytes(&mut bytes);
    let key_b64 = STANDARD.encode(bytes);
    let ciphertext = nip44::encrypt(
        node_keys.secret_key(),
        &node_keys.public_key(),
        &key_b64,
        nip44::Version::V2,
    )
    .map_err(|err| anyhow!("key encrypt failed: {err}"))?;

    sqlx::query(
        "INSERT INTO cn_admin.topic_scope_keys          (topic_id, scope, epoch, key_ciphertext)          VALUES ($1, $2, $3, $4)",
    )
    .bind(topic_id)
    .bind(scope)
    .bind(epoch as i32)
    .bind(ciphertext)
    .execute(&mut **tx)
    .await?;

    Ok(key_b64)
}

pub async fn rotate_epoch(
    pool: &Pool<Postgres>,
    node_keys: &Keys,
    topic_id: &str,
    scope: &str,
) -> Result<RotateSummary> {
    let topic_id = topic::normalize_topic_id(topic_id)?;
    let scope = normalize_scope(scope)?;
    let mut tx = pool.begin().await?;
    let prepared = prepare_rotation_tx(&mut tx, node_keys, &topic_id, &scope).await?;
    tx.commit().await?;

    let distribution_results = distribute_key_envelopes(pool, node_keys, &prepared).await?;

    Ok(RotateSummary {
        topic_id: prepared.topic_id,
        scope: prepared.scope,
        previous_epoch: prepared.previous_epoch,
        new_epoch: prepared.new_epoch,
        recipients: prepared.recipients.len(),
        distribution_results,
    })
}

pub async fn revoke_member_and_rotate(
    pool: &Pool<Postgres>,
    node_keys: &Keys,
    topic_id: &str,
    scope: &str,
    pubkey: &str,
    reason: Option<&str>,
) -> Result<RevokeSummary> {
    let topic_id = topic::normalize_topic_id(topic_id)?;
    let scope = normalize_scope(scope)?;
    let pubkey = normalize_pubkey(pubkey)?;

    let mut tx = pool.begin().await?;
    let status = sqlx::query_scalar::<_, String>(
        "SELECT status FROM cn_user.topic_memberships WHERE topic_id = $1 AND scope = $2 AND pubkey = $3 FOR UPDATE",
    )
    .bind(&topic_id)
    .bind(&scope)
    .bind(&pubkey)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(status) = status else {
        return Err(anyhow!("membership not found"));
    };
    if status != "active" {
        return Err(anyhow!("membership is not active"));
    }

    sqlx::query(
        "UPDATE cn_user.topic_memberships          SET status = 'revoked', revoked_at = NOW(), revoked_reason = $4          WHERE topic_id = $1 AND scope = $2 AND pubkey = $3",
    )
    .bind(&topic_id)
    .bind(&scope)
    .bind(&pubkey)
    .bind(reason)
    .execute(&mut *tx)
    .await?;

    let prepared = prepare_rotation_tx(&mut tx, node_keys, &topic_id, &scope).await?;
    tx.commit().await?;

    let distribution_results = distribute_key_envelopes(pool, node_keys, &prepared).await?;
    let rotation = RotateSummary {
        topic_id: prepared.topic_id,
        scope: prepared.scope,
        previous_epoch: prepared.previous_epoch,
        new_epoch: prepared.new_epoch,
        recipients: prepared.recipients.len(),
        distribution_results,
    };

    Ok(RevokeSummary {
        topic_id,
        scope,
        revoked_pubkey: pubkey,
        rotation,
    })
}

#[derive(Debug)]
struct PreparedRotation {
    topic_id: String,
    scope: String,
    previous_epoch: i64,
    new_epoch: i64,
    key_b64: String,
    recipients: Vec<String>,
}

async fn prepare_rotation_tx(
    tx: &mut Transaction<'_, Postgres>,
    node_keys: &Keys,
    topic_id: &str,
    scope: &str,
) -> Result<PreparedRotation> {
    let row = sqlx::query(
        "INSERT INTO cn_admin.topic_scope_state          (topic_id, scope, current_epoch)          VALUES ($1, $2, 1)          ON CONFLICT (topic_id, scope) DO UPDATE SET current_epoch = cn_admin.topic_scope_state.current_epoch + 1, updated_at = NOW()          RETURNING current_epoch",
    )
    .bind(topic_id)
    .bind(scope)
    .fetch_one(&mut **tx)
    .await?;
    let new_epoch: i32 = row.try_get("current_epoch")?;
    let new_epoch = new_epoch as i64;
    let previous_epoch = new_epoch.saturating_sub(1);

    let key_b64 = load_or_create_group_key(tx, node_keys, topic_id, scope, new_epoch).await?;
    let recipients: Vec<String> = sqlx::query_scalar(
        "SELECT pubkey FROM cn_user.topic_memberships WHERE topic_id = $1 AND scope = $2 AND status = 'active' ORDER BY pubkey",
    )
    .bind(topic_id)
    .bind(scope)
    .fetch_all(&mut **tx)
    .await?;

    for recipient in &recipients {
        upsert_distribution_status_tx(
            tx,
            topic_id,
            scope,
            new_epoch,
            recipient,
            DISTRIBUTION_STATUS_PENDING,
            None,
        )
        .await?;
    }

    Ok(PreparedRotation {
        topic_id: topic_id.to_string(),
        scope: scope.to_string(),
        previous_epoch,
        new_epoch,
        key_b64,
        recipients,
    })
}

async fn distribute_key_envelopes(
    pool: &Pool<Postgres>,
    node_keys: &Keys,
    prepared: &PreparedRotation,
) -> Result<Vec<DistributionResult>> {
    for recipient in &prepared.recipients {
        let distribution = match build_key_envelope_event(
            node_keys,
            recipient,
            &prepared.topic_id,
            &prepared.scope,
            prepared.new_epoch,
            &prepared.key_b64,
        ) {
            Ok(envelope) => {
                let envelope_json = match serde_json::to_value(&envelope) {
                    Ok(value) => value,
                    Err(err) => {
                        let reason = format!("key envelope serialize failed: {err}");
                        upsert_distribution_status(
                            pool,
                            &prepared.topic_id,
                            &prepared.scope,
                            prepared.new_epoch,
                            recipient,
                            DISTRIBUTION_STATUS_FAILED,
                            Some(&reason),
                        )
                        .await?;
                        continue;
                    }
                };

                if let Err(err) = sqlx::query(
                    "INSERT INTO cn_user.key_envelopes                      (topic_id, scope, epoch, recipient_pubkey, key_envelope_event_json)                      VALUES ($1, $2, $3, $4, $5)                      ON CONFLICT (topic_id, scope, epoch, recipient_pubkey)                      DO UPDATE SET key_envelope_event_json = EXCLUDED.key_envelope_event_json",
                )
                .bind(&prepared.topic_id)
                .bind(&prepared.scope)
                .bind(prepared.new_epoch as i32)
                .bind(recipient)
                .bind(envelope_json)
                .execute(pool)
                .await
                {
                    DistributionResult {
                        recipient_pubkey: recipient.clone(),
                        status: DISTRIBUTION_STATUS_FAILED.to_string(),
                        reason: Some(format!("key envelope upsert failed: {err}")),
                    }
                } else {
                    DistributionResult {
                        recipient_pubkey: recipient.clone(),
                        status: DISTRIBUTION_STATUS_SUCCESS.to_string(),
                        reason: None,
                    }
                }
            }
            Err(err) => DistributionResult {
                recipient_pubkey: recipient.clone(),
                status: DISTRIBUTION_STATUS_FAILED.to_string(),
                reason: Some(format!("key envelope build failed: {err}")),
            },
        };

        upsert_distribution_status(
            pool,
            &prepared.topic_id,
            &prepared.scope,
            prepared.new_epoch,
            &distribution.recipient_pubkey,
            &distribution.status,
            distribution.reason.as_deref(),
        )
        .await?;
    }

    fetch_distribution_results(
        pool,
        &prepared.topic_id,
        &prepared.scope,
        prepared.new_epoch,
    )
    .await
}

async fn fetch_distribution_results(
    pool: &Pool<Postgres>,
    topic_id: &str,
    scope: &str,
    epoch: i64,
) -> Result<Vec<DistributionResult>> {
    let rows = sqlx::query(
        "SELECT recipient_pubkey, status, reason          FROM cn_user.key_envelope_distribution_results          WHERE topic_id = $1 AND scope = $2 AND epoch = $3          ORDER BY recipient_pubkey",
    )
    .bind(topic_id)
    .bind(scope)
    .bind(epoch as i32)
    .fetch_all(pool)
    .await?;

    let mut results = Vec::with_capacity(rows.len());
    for row in rows {
        results.push(DistributionResult {
            recipient_pubkey: row.try_get("recipient_pubkey")?,
            status: row.try_get("status")?,
            reason: row.try_get("reason")?,
        });
    }
    Ok(results)
}

async fn upsert_distribution_status_tx(
    tx: &mut Transaction<'_, Postgres>,
    topic_id: &str,
    scope: &str,
    epoch: i64,
    recipient_pubkey: &str,
    status: &str,
    reason: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO cn_user.key_envelope_distribution_results          (topic_id, scope, epoch, recipient_pubkey, status, reason)          VALUES ($1, $2, $3, $4, $5, $6)          ON CONFLICT (topic_id, scope, epoch, recipient_pubkey)          DO UPDATE SET status = EXCLUDED.status, reason = EXCLUDED.reason, updated_at = NOW()",
    )
    .bind(topic_id)
    .bind(scope)
    .bind(epoch as i32)
    .bind(recipient_pubkey)
    .bind(status)
    .bind(reason)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn upsert_distribution_status(
    pool: &Pool<Postgres>,
    topic_id: &str,
    scope: &str,
    epoch: i64,
    recipient_pubkey: &str,
    status: &str,
    reason: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO cn_user.key_envelope_distribution_results          (topic_id, scope, epoch, recipient_pubkey, status, reason)          VALUES ($1, $2, $3, $4, $5, $6)          ON CONFLICT (topic_id, scope, epoch, recipient_pubkey)          DO UPDATE SET status = EXCLUDED.status, reason = EXCLUDED.reason, updated_at = NOW()",
    )
    .bind(topic_id)
    .bind(scope)
    .bind(epoch as i32)
    .bind(recipient_pubkey)
    .bind(status)
    .bind(reason)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nostr_sdk::prelude::Keys;
    use sqlx::postgres::PgPoolOptions;
    use sqlx::{Pool, Postgres};
    use uuid::Uuid;

    fn database_url() -> String {
        std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/postgres".to_string())
    }

    async fn connect_pool() -> Pool<Postgres> {
        let pool = PgPoolOptions::new()
            .connect(&database_url())
            .await
            .expect("connect postgres");
        crate::migrations::run(&pool).await.expect("run migrations");
        pool
    }

    #[test]
    fn normalize_scope_rejects_public() {
        let err = normalize_scope("public").expect_err("public should be rejected");
        assert!(err.to_string().contains("invalid scope"));
    }

    #[test]
    fn build_key_envelope_event_sets_required_tags() {
        let keys = Keys::generate();
        let recipient = Keys::generate().public_key().to_hex();
        let event =
            build_key_envelope_event(&keys, &recipient, "kukuri:topic1", "invite", 1, "aGVsbG8=")
                .expect("build event");

        assert_eq!(event.kind, KIND_KEY_ENVELOPE);
        assert!(event
            .tags
            .iter()
            .any(|tag| { tag == &vec!["p".to_string(), recipient.clone()] }));
        assert!(event
            .tags
            .iter()
            .any(|tag| tag == &vec!["scope".to_string(), "invite".to_string()]));
        assert!(event
            .tags
            .iter()
            .any(|tag| tag == &vec!["epoch".to_string(), "1".to_string()]));
        assert!(event
            .tags
            .iter()
            .any(|tag| tag == &vec!["k".to_string(), KIP_NAMESPACE.to_string()]));
        assert!(event
            .tags
            .iter()
            .any(|tag| tag == &vec!["ver".to_string(), KIP_VERSION.to_string()]));
    }

    #[test]
    fn normalize_distribution_status_rejects_unknown() {
        let err = normalize_distribution_status("in_progress").expect_err("invalid status");
        assert!(err.to_string().contains("invalid distribution status"));
    }

    #[tokio::test]
    async fn rotate_epoch_records_failed_distribution_for_invalid_pubkey() {
        let pool = connect_pool().await;
        let topic_id = format!("kukuri:core-contract:{}", Uuid::new_v4());
        let scope = "invite";
        let valid_pubkey = Keys::generate().public_key().to_hex();
        let invalid_pubkey = "invalid-pubkey-for-contract-test";

        sqlx::query(
            "INSERT INTO cn_user.topic_memberships              (topic_id, scope, pubkey, status)              VALUES ($1, $2, $3, 'active')",
        )
        .bind(&topic_id)
        .bind(scope)
        .bind(&valid_pubkey)
        .execute(&pool)
        .await
        .expect("insert valid membership");
        sqlx::query(
            "INSERT INTO cn_user.topic_memberships              (topic_id, scope, pubkey, status)              VALUES ($1, $2, $3, 'active')",
        )
        .bind(&topic_id)
        .bind(scope)
        .bind(invalid_pubkey)
        .execute(&pool)
        .await
        .expect("insert invalid membership");

        let summary = rotate_epoch(&pool, &Keys::generate(), &topic_id, scope)
            .await
            .expect("rotate epoch");

        assert_eq!(summary.recipients, 2);
        assert_eq!(summary.distribution_results.len(), 2);

        let success = summary
            .distribution_results
            .iter()
            .find(|item| item.recipient_pubkey == valid_pubkey)
            .expect("success result");
        assert_eq!(success.status, DISTRIBUTION_STATUS_SUCCESS);
        assert!(success.reason.is_none());

        let failed = summary
            .distribution_results
            .iter()
            .find(|item| item.recipient_pubkey == invalid_pubkey)
            .expect("failed result");
        assert_eq!(failed.status, DISTRIBUTION_STATUS_FAILED);
        let failure_reason = failed.reason.as_deref().unwrap_or_default();
        assert!(failure_reason.contains("invalid pubkey"));

        let failed_in_db: Option<String> = sqlx::query_scalar(
            "SELECT status              FROM cn_user.key_envelope_distribution_results              WHERE topic_id = $1 AND scope = $2 AND epoch = $3 AND recipient_pubkey = $4",
        )
        .bind(&topic_id)
        .bind(scope)
        .bind(summary.new_epoch as i32)
        .bind(invalid_pubkey)
        .fetch_optional(&pool)
        .await
        .expect("load distribution status");
        assert_eq!(failed_in_db.as_deref(), Some(DISTRIBUTION_STATUS_FAILED));

        sqlx::query("DELETE FROM cn_user.key_envelope_distribution_results WHERE topic_id = $1")
            .bind(&topic_id)
            .execute(&pool)
            .await
            .expect("cleanup distribution results");
        sqlx::query("DELETE FROM cn_user.key_envelopes WHERE topic_id = $1")
            .bind(&topic_id)
            .execute(&pool)
            .await
            .expect("cleanup key envelopes");
        sqlx::query("DELETE FROM cn_user.topic_memberships WHERE topic_id = $1")
            .bind(&topic_id)
            .execute(&pool)
            .await
            .expect("cleanup memberships");
        sqlx::query("DELETE FROM cn_admin.topic_scope_keys WHERE topic_id = $1")
            .bind(&topic_id)
            .execute(&pool)
            .await
            .expect("cleanup topic scope keys");
        sqlx::query("DELETE FROM cn_admin.topic_scope_state WHERE topic_id = $1")
            .bind(&topic_id)
            .execute(&pool)
            .await
            .expect("cleanup topic scope state");
    }
}
