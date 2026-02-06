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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotateSummary {
    pub topic_id: String,
    pub scope: String,
    pub previous_epoch: i64,
    pub new_epoch: i64,
    pub recipients: usize,
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
    let summary = rotate_epoch_tx(&mut tx, node_keys, &topic_id, &scope).await?;
    tx.commit().await?;
    Ok(summary)
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

    let rotation = rotate_epoch_tx(&mut tx, node_keys, &topic_id, &scope).await?;
    tx.commit().await?;

    Ok(RevokeSummary {
        topic_id,
        scope,
        revoked_pubkey: pubkey,
        rotation,
    })
}

async fn rotate_epoch_tx(
    tx: &mut Transaction<'_, Postgres>,
    node_keys: &Keys,
    topic_id: &str,
    scope: &str,
) -> Result<RotateSummary> {
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
        let envelope =
            build_key_envelope_event(node_keys, recipient, topic_id, scope, new_epoch, &key_b64)?;
        let envelope_json = serde_json::to_value(&envelope)?;
        sqlx::query(
            "INSERT INTO cn_user.key_envelopes              (topic_id, scope, epoch, recipient_pubkey, key_envelope_event_json)              VALUES ($1, $2, $3, $4, $5)              ON CONFLICT (topic_id, scope, epoch, recipient_pubkey) DO UPDATE SET key_envelope_event_json = EXCLUDED.key_envelope_event_json",
        )
        .bind(topic_id)
        .bind(scope)
        .bind(new_epoch as i32)
        .bind(recipient)
        .bind(envelope_json)
        .execute(&mut **tx)
        .await?;
    }

    Ok(RotateSummary {
        topic_id: topic_id.to_string(),
        scope: scope.to_string(),
        previous_epoch,
        new_epoch,
        recipients: recipients.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nostr_sdk::prelude::Keys;

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
}
