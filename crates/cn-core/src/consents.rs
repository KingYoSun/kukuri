use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::postgres::PgPool;

use crate::database::ensure_active_subscriber;
use crate::errors::{ApiError, ApiResult, consent_required_error};
use kukuri_cn_protocol::models::{
    CommunityNodeConsentItem, CommunityNodeConsentStatus, CommunityNodePolicyDocument,
};
use kukuri_cn_protocol::normalize::normalize_pubkey;

/// 公開 policy カタログ(#857)。認証不要の同意提示用で、ユーザー固有情報を含まない。
pub async fn list_policies(pool: &PgPool) -> Result<Vec<CommunityNodePolicyDocument>> {
    let rows = sqlx::query(
        "SELECT policy_slug, policy_version, title, body_markdown, required,
                effective_date::text AS effective_date, language
         FROM cn_admin.policies
         ORDER BY policy_slug ASC",
    )
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|row| -> Result<CommunityNodePolicyDocument> {
            Ok(CommunityNodePolicyDocument {
                policy_slug: row.try_get("policy_slug")?,
                policy_version: row.try_get("policy_version")?,
                title: row.try_get("title")?,
                body_markdown: row.try_get("body_markdown")?,
                required: row.try_get("required")?,
                effective_date: row.try_get("effective_date")?,
                language: row.try_get("language")?,
            })
        })
        .collect()
}

pub async fn get_consent_status(pool: &PgPool, pubkey: &str) -> Result<CommunityNodeConsentStatus> {
    let pubkey = normalize_pubkey(pubkey)?;
    let rows = sqlx::query(
        "SELECT
            p.policy_slug,
            p.policy_version,
            p.title,
            p.body_markdown,
            p.required,
            p.effective_date::text AS effective_date,
            p.language,
            c.accepted_at,
            prev.previously_accepted_version
         FROM cn_admin.policies p
         LEFT JOIN cn_user.policy_consents c
           ON c.policy_slug = p.policy_slug
          AND c.policy_version = p.policy_version
          AND c.subscriber_pubkey = $1
         LEFT JOIN (
            SELECT policy_slug, MAX(policy_version) AS previously_accepted_version
            FROM cn_user.policy_consents
            WHERE subscriber_pubkey = $1
            GROUP BY policy_slug
         ) prev
           ON prev.policy_slug = p.policy_slug
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
                body: row.try_get("body_markdown")?,
                required: row.try_get("required")?,
                effective_date: row.try_get("effective_date")?,
                language: row.try_get("language")?,
                accepted_at,
                previously_accepted_version: row.try_get("previously_accepted_version")?,
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

/// operator config から生成した current policy を同期する。
///
/// version は単調増加とし、同一 version の本文・metadata 差し替えを拒否する。旧英語
/// placeholder だけは #860 の一回限りの移行対象として同意履歴を破棄して置換する。
pub async fn sync_policies(pool: &PgPool, policies: &[CommunityNodePolicyDocument]) -> Result<()> {
    let mut tx = pool.begin().await?;
    for policy in policies {
        if policy.policy_slug.trim().is_empty()
            || policy.policy_version <= 0
            || policy.title.trim().is_empty()
            || policy.body_markdown.trim().is_empty()
            || policy.effective_date.as_deref().is_none_or(str::is_empty)
            || policy.language.as_deref().is_none_or(str::is_empty)
        {
            bail!(
                "operator policy metadata is incomplete for `{}`",
                policy.policy_slug
            );
        }
        let existing = sqlx::query(
            "SELECT policy_version, title, body_markdown, required,
                    effective_date::text AS effective_date, language
             FROM cn_admin.policies
             WHERE policy_slug = $1
             FOR UPDATE",
        )
        .bind(&policy.policy_slug)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(existing) = existing {
            let version: i32 = existing.try_get("policy_version")?;
            let title: String = existing.try_get("title")?;
            let body: String = existing.try_get("body_markdown")?;
            let required: bool = existing.try_get("required")?;
            let effective_date: Option<String> = existing.try_get("effective_date")?;
            let language: Option<String> = existing.try_get("language")?;
            let identical = version == policy.policy_version
                && title == policy.title
                && body == policy.body_markdown
                && required == policy.required
                && effective_date == policy.effective_date
                && language == policy.language;
            if identical {
                continue;
            }
            let legacy_placeholder = version == 1
                && required
                && effective_date.is_none()
                && language.is_none()
                && matches!(
                    (policy.policy_slug.as_str(), title.as_str(), body.as_str()),
                    (
                        "terms_of_service",
                        "Terms of Service",
                        "You must follow the community node terms of service."
                    ) | (
                        "privacy_policy",
                        "Privacy Policy",
                        "You must acknowledge the community node privacy policy."
                    )
                );
            if legacy_placeholder {
                sqlx::query("DELETE FROM cn_user.policy_consents WHERE policy_slug = $1")
                    .bind(&policy.policy_slug)
                    .execute(&mut *tx)
                    .await?;
            } else if policy.policy_version < version {
                bail!(
                    "policy `{}` version rollback is not allowed: stored={}, configured={}",
                    policy.policy_slug,
                    version,
                    policy.policy_version
                );
            } else if policy.policy_version == version {
                bail!(
                    "policy `{}` content changed without a version increase",
                    policy.policy_slug
                );
            }
            sqlx::query(
                "UPDATE cn_admin.policies
                 SET policy_version = $2, title = $3, body_markdown = $4, required = $5,
                     effective_date = $6::date, language = $7, updated_at = NOW()
                 WHERE policy_slug = $1",
            )
            .bind(&policy.policy_slug)
            .bind(policy.policy_version)
            .bind(&policy.title)
            .bind(&policy.body_markdown)
            .bind(policy.required)
            .bind(&policy.effective_date)
            .bind(&policy.language)
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query(
                "INSERT INTO cn_admin.policies
                    (policy_slug, policy_version, title, body_markdown, required, effective_date, language)
                 VALUES ($1, $2, $3, $4, $5, $6::date, $7)",
            )
            .bind(&policy.policy_slug)
            .bind(policy.policy_version)
            .bind(&policy.title)
            .bind(&policy.body_markdown)
            .bind(policy.required)
            .bind(&policy.effective_date)
            .bind(&policy.language)
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;
    Ok(())
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
