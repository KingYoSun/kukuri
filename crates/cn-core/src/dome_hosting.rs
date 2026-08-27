use anyhow::{Context, Result, bail};
use serde_json::Value;
use sqlx::postgres::PgPool;
use sqlx::{Postgres, Row, Transaction};

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
