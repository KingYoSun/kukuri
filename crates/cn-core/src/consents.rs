use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::postgres::PgPool;

use crate::database::ensure_active_subscriber;
use crate::errors::{ApiError, ApiResult, consent_required_error};
use crate::models::{CommunityNodeConsentItem, CommunityNodeConsentStatus};
use crate::normalize::normalize_pubkey;

pub async fn get_consent_status(pool: &PgPool, pubkey: &str) -> Result<CommunityNodeConsentStatus> {
    let pubkey = normalize_pubkey(pubkey)?;
    let rows = sqlx::query(
        "SELECT
            p.policy_slug,
            p.policy_version,
            p.title,
            p.required,
            c.accepted_at
         FROM cn_admin.policies p
         LEFT JOIN cn_user.policy_consents c
           ON c.policy_slug = p.policy_slug
          AND c.policy_version = p.policy_version
          AND c.subscriber_pubkey = $1
         ORDER BY p.policy_slug ASC",
    )
    .bind(&pubkey)
    .fetch_all(pool)
    .await?;
    let items = rows
        .into_iter()
        .map(|row| -> Result<CommunityNodeConsentItem> {
            let accepted_at = row
                .try_get::<Option<DateTime<Utc>>, _>("accepted_at")?
                .map(|value| value.timestamp());
            Ok(CommunityNodeConsentItem {
                policy_slug: row.try_get("policy_slug")?,
                policy_version: row.try_get("policy_version")?,
                title: row.try_get("title")?,
                required: row.try_get("required")?,
                accepted_at,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let all_required_accepted = items
        .iter()
        .filter(|item| item.required)
        .all(|item| item.accepted_at.is_some());
    Ok(CommunityNodeConsentStatus {
        all_required_accepted,
        items,
    })
}

pub async fn accept_consents(
    pool: &PgPool,
    pubkey: &str,
    policy_slugs: &[String],
) -> Result<CommunityNodeConsentStatus> {
    let pubkey = normalize_pubkey(pubkey)?;
    let desired = if policy_slugs.is_empty() {
        sqlx::query(
            "SELECT policy_slug, policy_version
             FROM cn_admin.policies
             WHERE required = TRUE",
        )
        .fetch_all(pool)
        .await?
    } else {
        let mut records = Vec::new();
        for slug in normalize_slug_list(policy_slugs) {
            let row = sqlx::query(
                "SELECT policy_slug, policy_version
                 FROM cn_admin.policies
                 WHERE policy_slug = $1",
            )
            .bind(&slug)
            .fetch_optional(pool)
            .await?;
            let Some(row) = row else {
                bail!("unknown policy slug `{slug}`");
            };
            records.push(row);
        }
        records
    };

    let mut tx = pool.begin().await?;
    ensure_active_subscriber(&mut *tx, pubkey.as_str()).await?;
    for row in desired {
        let slug: String = row.try_get("policy_slug")?;
        let version: i32 = row.try_get("policy_version")?;
        sqlx::query(
            "INSERT INTO cn_user.policy_consents
                (subscriber_pubkey, policy_slug, policy_version, accepted_at)
             VALUES ($1, $2, $3, NOW())
             ON CONFLICT (subscriber_pubkey, policy_slug, policy_version) DO UPDATE
             SET accepted_at = EXCLUDED.accepted_at",
        )
        .bind(&pubkey)
        .bind(slug)
        .bind(version)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    get_consent_status(pool, pubkey.as_str()).await
}

pub async fn require_consents(
    pool: &PgPool,
    pubkey: &str,
) -> ApiResult<CommunityNodeConsentStatus> {
    let status = get_consent_status(pool, pubkey).await.map_err(|error| {
        ApiError::new(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            error.to_string(),
        )
    })?;
    if !status.all_required_accepted {
        return Err(consent_required_error(
            "required policies have not been accepted",
        ));
    }
    Ok(status)
}

fn normalize_slug_list(values: &[String]) -> Vec<String> {
    let mut deduped = std::collections::BTreeSet::new();
    for value in values {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            deduped.insert(trimmed.to_string());
        }
    }
    deduped.into_iter().collect()
}
