use anyhow::{Context, Result, bail};
use serde_json::Value;
use sqlx::postgres::PgPool;
use sqlx::{Postgres, Row, Transaction};

pub const COMMUNITY_NODE_DOME_BLOB_CACHE_CAPACITY_BYTES: u64 = 10 * 1024 * 1024 * 1024;
pub const DOME_BLOB_CACHE_GC_GRACE_MILLIS: i64 = 24 * 60 * 60 * 1_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StagedDomeBlob {
    pub blob_hash: String,
    pub data: Vec<u8>,
}

pub async fn stage_dome_hosting_blobs(
    pool: &PgPool,
    reference_id: &str,
    blobs: &[StagedDomeBlob],
    now_millis: i64,
) -> Result<()> {
    if reference_id.trim().is_empty() {
        bail!("Dome blob staging reference is required");
    }
    let mut unique = std::collections::BTreeMap::new();
    for blob in blobs {
        if blob.blob_hash != blake3::hash(&blob.data).to_hex().to_string() {
            bail!("Dome staged blob content hash mismatch");
        }
        unique.entry(blob.blob_hash.clone()).or_insert(&blob.data);
    }
    let mut tx = pool.begin().await?;
    let current_bytes: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(bytes), 0)::BIGINT FROM cn_metaverse.dome_blob_cache",
    )
    .fetch_one(&mut *tx)
    .await?;
    let mut additional = 0_u64;
    for (hash, data) in &unique {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM cn_metaverse.dome_blob_cache WHERE blob_hash = $1)",
        )
        .bind(hash)
        .fetch_one(&mut *tx)
        .await?;
        if !exists {
            additional = additional.saturating_add(data.len() as u64);
        }
    }
    if (current_bytes.max(0) as u64).saturating_add(additional)
        > COMMUNITY_NODE_DOME_BLOB_CACHE_CAPACITY_BYTES
    {
        bail!("Community Node metaverse manifest/asset blob cache capacity exceeded");
    }
    for (hash, data) in unique {
        sqlx::query(
            "INSERT INTO cn_metaverse.dome_blob_cache
                (blob_hash, bytes, data, last_accessed_at, unreferenced_at)
             VALUES ($1, $2, $3, $4, NULL)
             ON CONFLICT (blob_hash) DO UPDATE SET
                last_accessed_at = EXCLUDED.last_accessed_at,
                unreferenced_at = NULL",
        )
        .bind(&hash)
        .bind(data.len() as i64)
        .bind(data)
        .bind(now_millis)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO cn_metaverse.dome_blob_pins
                (blob_hash, reason, reference_id, created_at)
             VALUES ($1, 'staging', $2, $3)
             ON CONFLICT DO NOTHING",
        )
        .bind(hash)
        .bind(reference_id)
        .bind(now_millis)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn activate_dome_hosting_blob_pins(
    pool: &PgPool,
    instance_id: &str,
    reference_id: &str,
    now_millis: i64,
) -> Result<()> {
    let prefix = format!("{instance_id}:%");
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO cn_metaverse.dome_blob_pins
            (blob_hash, reason, reference_id, created_at)
         SELECT blob_hash, 'rollback', reference_id, created_at
         FROM cn_metaverse.dome_blob_pins
         WHERE reason = 'current' AND reference_id LIKE $1 AND reference_id <> $2
         ON CONFLICT DO NOTHING",
    )
    .bind(&prefix)
    .bind(reference_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "DELETE FROM cn_metaverse.dome_blob_pins
         WHERE reason = 'current' AND reference_id LIKE $1 AND reference_id <> $2",
    )
    .bind(&prefix)
    .bind(reference_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "DELETE FROM cn_metaverse.dome_blob_pins
         WHERE reason = 'active_lease' AND reference_id LIKE $1",
    )
    .bind(&prefix)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO cn_metaverse.dome_blob_pins
            (blob_hash, reason, reference_id, created_at)
         SELECT blob_hash, 'current', reference_id, $2
         FROM cn_metaverse.dome_blob_pins
         WHERE reason = 'staging' AND reference_id = $1
         ON CONFLICT (blob_hash, reason, reference_id)
         DO UPDATE SET created_at = EXCLUDED.created_at",
    )
    .bind(reference_id)
    .bind(now_millis)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "DELETE FROM cn_metaverse.dome_blob_pins
         WHERE reason = 'staging' AND reference_id = $1",
    )
    .bind(reference_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO cn_metaverse.dome_blob_pins
            (blob_hash, reason, reference_id, created_at)
         SELECT blob_hash, 'active_lease', reference_id, $2
         FROM cn_metaverse.dome_blob_pins
         WHERE reason = 'current' AND reference_id = $1
         ON CONFLICT DO NOTHING",
    )
    .bind(reference_id)
    .bind(now_millis)
    .execute(&mut *tx)
    .await?;
    let stale_references = sqlx::query_scalar::<_, String>(
        "SELECT reference_id
         FROM cn_metaverse.dome_blob_pins
         WHERE reason = 'rollback' AND reference_id LIKE $1
         GROUP BY reference_id
         ORDER BY MAX(created_at) DESC, reference_id DESC
         OFFSET 3",
    )
    .bind(&prefix)
    .fetch_all(&mut *tx)
    .await?;
    for stale in stale_references {
        sqlx::query(
            "DELETE FROM cn_metaverse.dome_blob_pins
             WHERE reason = 'rollback' AND reference_id = $1",
        )
        .bind(stale)
        .execute(&mut *tx)
        .await?;
    }
    mark_unreferenced_dome_blobs(&mut tx, now_millis).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn release_dome_hosting_blob_pins(
    pool: &PgPool,
    reference_id: &str,
    now_millis: i64,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "DELETE FROM cn_metaverse.dome_blob_pins
         WHERE reason = 'active_lease' AND reference_id = $1",
    )
    .bind(reference_id)
    .execute(&mut *tx)
    .await?;
    mark_unreferenced_dome_blobs(&mut tx, now_millis).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn collect_dome_blob_cache_garbage(pool: &PgPool, now_millis: i64) -> Result<u64> {
    let cutoff = now_millis.saturating_sub(DOME_BLOB_CACHE_GC_GRACE_MILLIS);
    let result = sqlx::query(
        "DELETE FROM cn_metaverse.dome_blob_cache
         WHERE unreferenced_at IS NOT NULL AND unreferenced_at <= $1
           AND NOT EXISTS (
             SELECT 1 FROM cn_metaverse.dome_blob_pins pins
             WHERE pins.blob_hash = cn_metaverse.dome_blob_cache.blob_hash
           )",
    )
    .bind(cutoff)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

async fn mark_unreferenced_dome_blobs(
    tx: &mut Transaction<'_, Postgres>,
    now_millis: i64,
) -> Result<()> {
    sqlx::query(
        "UPDATE cn_metaverse.dome_blob_cache cache
         SET unreferenced_at = COALESCE(unreferenced_at, $1)
         WHERE NOT EXISTS (
            SELECT 1 FROM cn_metaverse.dome_blob_pins pins
            WHERE pins.blob_hash = cache.blob_hash
         )",
    )
    .bind(now_millis)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[derive(Clone, Debug, PartialEq)]
pub struct DomeHostingAssignment {
    pub instance_id: String,
    pub owner_pubkey: String,
    pub lease_id: String,
    pub lease_epoch: u64,
    pub expires_at: i64,
    pub session_id: String,
    pub status: String,
    pub signed_lease_json: Value,
    pub instance_manifest_json: Value,
    pub preset_manifest_json: Value,
    pub signed_acceptance_json: Value,
    pub signed_activation_json: Option<Value>,
    pub signed_close_json: Option<Value>,
}

pub struct NewDomeHostingAssignment<'a> {
    pub instance_id: &'a str,
    pub owner_pubkey: &'a str,
    pub lease_id: &'a str,
    pub lease_epoch: u64,
    pub expires_at: i64,
    pub session_id: &'a str,
    pub signed_lease_json: Value,
    pub instance_manifest_json: Value,
    pub preset_manifest_json: Value,
    pub signed_acceptance_json: Value,
}

pub async fn upsert_pending_dome_hosting_assignment(
    pool: &PgPool,
    input: NewDomeHostingAssignment<'_>,
) -> Result<DomeHostingAssignment> {
    if input.instance_id.trim().is_empty()
        || input.owner_pubkey.trim().is_empty()
        || input.lease_id.trim().is_empty()
        || input.session_id.trim().is_empty()
        || input.lease_epoch == 0
    {
        bail!("Dome hosting assignment identity is incomplete");
    }
    let mut tx = pool.begin().await?;
    if let Some(existing) = fetch_assignment_for_update(&mut tx, input.instance_id).await? {
        if existing.lease_epoch > input.lease_epoch {
            bail!("stale Dome Hosting Lease epoch");
        }
        if existing.lease_epoch == input.lease_epoch {
            if existing.lease_id == input.lease_id
                && existing.owner_pubkey == input.owner_pubkey
                && existing.signed_lease_json == input.signed_lease_json
            {
                tx.commit().await?;
                return Ok(existing);
            }
            bail!("split-brain Dome Hosting Lease epoch");
        }
    }

    sqlx::query(
        "INSERT INTO cn_metaverse.dome_hosting_assignments (
            instance_id, owner_pubkey, lease_id, lease_epoch, expires_at, session_id, status,
            signed_lease_json, instance_manifest_json, preset_manifest_json,
            signed_acceptance_json, signed_activation_json, signed_close_json
         ) VALUES ($1, $2, $3, $4, $5, $6, 'pending', $7, $8, $9, $10, NULL, NULL)
         ON CONFLICT (instance_id) DO UPDATE SET
            owner_pubkey = EXCLUDED.owner_pubkey,
            lease_id = EXCLUDED.lease_id,
            lease_epoch = EXCLUDED.lease_epoch,
            expires_at = EXCLUDED.expires_at,
            session_id = EXCLUDED.session_id,
            status = 'pending',
            signed_lease_json = EXCLUDED.signed_lease_json,
            instance_manifest_json = EXCLUDED.instance_manifest_json,
            preset_manifest_json = EXCLUDED.preset_manifest_json,
            signed_acceptance_json = EXCLUDED.signed_acceptance_json,
            signed_activation_json = NULL,
            signed_close_json = NULL,
            updated_at = NOW()",
    )
    .bind(input.instance_id)
    .bind(input.owner_pubkey)
    .bind(input.lease_id)
    .bind(input.lease_epoch as i64)
    .bind(input.expires_at)
    .bind(input.session_id)
    .bind(input.signed_lease_json)
    .bind(input.instance_manifest_json)
    .bind(input.preset_manifest_json)
    .bind(input.signed_acceptance_json)
    .execute(&mut *tx)
    .await?;
    let stored = fetch_assignment_for_update(&mut tx, input.instance_id)
        .await?
        .context("stored Dome hosting assignment is unavailable")?;
    tx.commit().await?;
    Ok(stored)
}

pub async fn activate_dome_hosting_assignment(
    pool: &PgPool,
    instance_id: &str,
    lease_epoch: u64,
    signed_activation_json: Value,
) -> Result<DomeHostingAssignment> {
    let result = sqlx::query(
        "UPDATE cn_metaverse.dome_hosting_assignments
         SET status = 'active', signed_activation_json = $3, updated_at = NOW()
         WHERE instance_id = $1 AND lease_epoch = $2 AND status IN ('pending', 'active')",
    )
    .bind(instance_id)
    .bind(lease_epoch as i64)
    .bind(signed_activation_json)
    .execute(pool)
    .await?;
    if result.rows_affected() != 1 {
        bail!("pending Dome hosting assignment was not found");
    }
    get_dome_hosting_assignment(pool, instance_id)
        .await?
        .context("activated Dome hosting assignment is unavailable")
}

pub async fn close_dome_hosting_assignment(
    pool: &PgPool,
    instance_id: &str,
    lease_epoch: u64,
    signed_close_json: Value,
) -> Result<DomeHostingAssignment> {
    let result = sqlx::query(
        "UPDATE cn_metaverse.dome_hosting_assignments
         SET status = 'closed', signed_close_json = $3, updated_at = NOW()
         WHERE instance_id = $1 AND lease_epoch = $2 AND status IN ('pending', 'active', 'closed')",
    )
    .bind(instance_id)
    .bind(lease_epoch as i64)
    .bind(signed_close_json)
    .execute(pool)
    .await?;
    if result.rows_affected() != 1 {
        bail!("Dome hosting assignment was not found");
    }
    get_dome_hosting_assignment(pool, instance_id)
        .await?
        .context("closed Dome hosting assignment is unavailable")
}

pub async fn get_dome_hosting_assignment(
    pool: &PgPool,
    instance_id: &str,
) -> Result<Option<DomeHostingAssignment>> {
    let row = sqlx::query(
        "SELECT instance_id, owner_pubkey, lease_id, lease_epoch, expires_at, session_id,
                status, signed_lease_json, instance_manifest_json, preset_manifest_json,
                signed_acceptance_json, signed_activation_json, signed_close_json
         FROM cn_metaverse.dome_hosting_assignments WHERE instance_id = $1",
    )
    .bind(instance_id)
    .fetch_optional(pool)
    .await?;
    row.map(row_to_assignment).transpose()
}

pub async fn list_recoverable_dome_hosting_assignments(
    pool: &PgPool,
    now_millis: i64,
) -> Result<Vec<DomeHostingAssignment>> {
    let rows = sqlx::query(
        "SELECT instance_id, owner_pubkey, lease_id, lease_epoch, expires_at, session_id,
                status, signed_lease_json, instance_manifest_json, preset_manifest_json,
                signed_acceptance_json, signed_activation_json, signed_close_json
         FROM cn_metaverse.dome_hosting_assignments
         WHERE status = 'active' AND expires_at > $1
         ORDER BY instance_id",
    )
    .bind(now_millis)
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(row_to_assignment).collect()
}

async fn fetch_assignment_for_update(
    tx: &mut Transaction<'_, Postgres>,
    instance_id: &str,
) -> Result<Option<DomeHostingAssignment>> {
    let row = sqlx::query(
        "SELECT instance_id, owner_pubkey, lease_id, lease_epoch, expires_at, session_id,
                status, signed_lease_json, instance_manifest_json, preset_manifest_json,
                signed_acceptance_json, signed_activation_json, signed_close_json
         FROM cn_metaverse.dome_hosting_assignments WHERE instance_id = $1 FOR UPDATE",
    )
    .bind(instance_id)
    .fetch_optional(&mut **tx)
    .await?;
    row.map(row_to_assignment).transpose()
}

fn row_to_assignment(row: sqlx::postgres::PgRow) -> Result<DomeHostingAssignment> {
    let lease_epoch: i64 = row.try_get("lease_epoch")?;
    Ok(DomeHostingAssignment {
        instance_id: row.try_get("instance_id")?,
        owner_pubkey: row.try_get("owner_pubkey")?,
        lease_id: row.try_get("lease_id")?,
        lease_epoch: lease_epoch.try_into().context("invalid lease epoch")?,
        expires_at: row.try_get("expires_at")?,
        session_id: row.try_get("session_id")?,
        status: row.try_get("status")?,
        signed_lease_json: row.try_get("signed_lease_json")?,
        instance_manifest_json: row.try_get("instance_manifest_json")?,
        preset_manifest_json: row.try_get("preset_manifest_json")?,
        signed_acceptance_json: row.try_get("signed_acceptance_json")?,
        signed_activation_json: row.try_get("signed_activation_json")?,
        signed_close_json: row.try_get("signed_close_json")?,
    })
}
