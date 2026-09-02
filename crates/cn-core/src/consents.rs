use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::postgres::{PgPool, PgRow};

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
                effective_date::text AS effective_date, language,
                policy_snapshot_revision, material_change, requires_reconsent
         FROM cn_admin.policies
         WHERE is_current = TRUE
         ORDER BY policy_slug ASC",
    )
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(|row| policy_from_row(&row)).collect()
}

/// 現行正文を要求言語の厳密な参考訳へ解決する。対応する同一正文 version の訳が
/// 無ければ正文を返し、`fallback` metadata で判別可能にする。
pub async fn list_policies_for_language(
    pool: &PgPool,
    requested_language: Option<&str>,
) -> Result<Vec<CommunityNodePolicyDocument>> {
    let policies = list_policies(pool).await?;
    let Some(language) = requested_language.filter(|value| !value.trim().is_empty()) else {
        return Ok(policies);
    };
    let mut localized = Vec::with_capacity(policies.len());
    for policy in policies {
        localized.push(
            get_policy_revision(
                pool,
                policy.policy_slug.as_str(),
                policy.policy_version,
                Some(language),
            )
            .await?
            .expect("current policy revision must exist"),
        );
    }
    Ok(localized)
}

pub async fn list_policy_revisions(
    pool: &PgPool,
    policy_slug: &str,
) -> Result<Vec<CommunityNodePolicyDocument>> {
    let rows = sqlx::query(
        "SELECT policy_slug, policy_version, title, body_markdown, required,
                effective_date::text AS effective_date, language,
                policy_snapshot_revision, material_change, requires_reconsent
         FROM cn_admin.policies
         WHERE policy_slug = $1
         ORDER BY policy_version DESC",
    )
    .bind(policy_slug)
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(|row| policy_from_row(&row)).collect()
}

pub async fn get_policy_revision(
    pool: &PgPool,
    policy_slug: &str,
    policy_version: i32,
    requested_language: Option<&str>,
) -> Result<Option<CommunityNodePolicyDocument>> {
    let row = sqlx::query(
        "SELECT policy_slug, policy_version, title, body_markdown, required,
                effective_date::text AS effective_date, language,
                policy_snapshot_revision, material_change, requires_reconsent
         FROM cn_admin.policies
         WHERE policy_slug = $1 AND policy_version = $2",
    )
    .bind(policy_slug)
    .bind(policy_version)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let mut policy = policy_from_row(&row)?;
    let requested = requested_language
        .map(str::trim)
        .filter(|language| !language.is_empty());
    policy.requested_language = requested.map(str::to_string);
    let authoritative = policy.language.clone();
    if let Some(requested) = requested
        && authoritative.as_deref() != Some(requested)
    {
        let translation = sqlx::query(
            "SELECT title, body_markdown, language, translation_revision
             FROM cn_admin.policy_translations
             WHERE policy_slug = $1 AND policy_version = $2
               AND language = $3 AND is_current = TRUE",
        )
        .bind(policy_slug)
        .bind(policy_version)
        .bind(requested)
        .fetch_optional(pool)
        .await?;
        if let Some(translation) = translation {
            policy.title = translation.try_get("title")?;
            policy.body_markdown = translation.try_get("body_markdown")?;
            policy.language = Some(translation.try_get("language")?);
            policy.reference_translation = true;
            policy.translation_revision = Some(translation.try_get("translation_revision")?);
            policy.translation_of_version = Some(policy_version);
        } else {
            policy.fallback = true;
        }
    }
    Ok(Some(policy))
}

fn policy_from_row(row: &PgRow) -> Result<CommunityNodePolicyDocument> {
    let language: Option<String> = row.try_get("language")?;
    Ok(CommunityNodePolicyDocument {
        policy_slug: row.try_get("policy_slug")?,
        policy_version: row.try_get("policy_version")?,
        title: row.try_get("title")?,
        body_markdown: row.try_get("body_markdown")?,
        required: row.try_get("required")?,
        effective_date: row.try_get("effective_date")?,
        language: language.clone(),
        policy_snapshot_revision: row.try_get("policy_snapshot_revision")?,
        authoritative_language: language,
        reference_translation: false,
        translation_revision: None,
        translation_of_version: None,
        fallback: false,
        requested_language: None,
        material_change: row.try_get("material_change")?,
        requires_reconsent: row.try_get("requires_reconsent")?,
    })
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
            p.policy_snapshot_revision,
            c.accepted_at,
            prev.previously_accepted_version
         FROM cn_admin.policies p
         LEFT JOIN cn_user.policy_consents c
           ON c.policy_slug = p.policy_slug
          AND c.policy_version = p.policy_version
          AND c.subscriber_pubkey = $1
          AND (
            p.policy_snapshot_revision IS NULL
            OR c.policy_snapshot_revision = p.policy_snapshot_revision
          )
         LEFT JOIN (
            SELECT policy_slug, MAX(policy_version) AS previously_accepted_version
            FROM cn_user.policy_consents
            WHERE subscriber_pubkey = $1
            GROUP BY policy_slug
         ) prev
           ON prev.policy_slug = p.policy_slug
         WHERE p.is_current = TRUE
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
                policy_snapshot_revision: row.try_get("policy_snapshot_revision")?,
                accepted_at,
                previously_accepted_version: row.try_get("previously_accepted_version")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let all_required_accepted = items
        .iter()
        .filter(|item| item.required)
        .all(|item| item.accepted_at.is_some());
    let policy_snapshot_revision = items
        .iter()
        .find_map(|item| item.policy_snapshot_revision.clone());
    Ok(CommunityNodeConsentStatus {
        all_required_accepted,
        items,
        policy_snapshot_revision,
    })
}

/// operator config から生成した current policy を同期する。
///
/// version は単調増加とし、同一 version の本文・metadata 差し替えを拒否する。旧英語
/// placeholder だけは #860 の一回限りの移行対象として同意履歴を破棄して置換する。
pub async fn sync_policies(pool: &PgPool, policies: &[CommunityNodePolicyDocument]) -> Result<()> {
    let mut tx = pool.begin().await?;
    for policy in policies
        .iter()
        .filter(|policy| !policy.reference_translation)
    {
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
                    effective_date::text AS effective_date, language,
                    policy_snapshot_revision
             FROM cn_admin.policies
             WHERE policy_slug = $1 AND is_current = TRUE
             FOR UPDATE",
        )
        .bind(&policy.policy_slug)
        .fetch_optional(&mut *tx)
        .await?;
        let had_existing = existing.is_some();
        if let Some(existing) = existing {
            let version: i32 = existing.try_get("policy_version")?;
            let title: String = existing.try_get("title")?;
            let body: String = existing.try_get("body_markdown")?;
            let required: bool = existing.try_get("required")?;
            let effective_date: Option<String> = existing.try_get("effective_date")?;
            let language: Option<String> = existing.try_get("language")?;
            let snapshot_revision: Option<String> = existing.try_get("policy_snapshot_revision")?;
            let identical_document = version == policy.policy_version
                && title == policy.title
                && body == policy.body_markdown
                && required == policy.required
                && effective_date == policy.effective_date
                && language == policy.language;
            if identical_document && snapshot_revision == policy.policy_snapshot_revision {
                continue;
            }
            // catalog-wide の法務設定が変わっても、本文・正文 metadata が同一なら文書
            // version の水増しは要求しない。正文は不変のまま current snapshot だけを
            // 進め、旧 snapshot の同意を自動的に無効化する。
            if identical_document {
                sqlx::query(
                    "UPDATE cn_admin.policies
                     SET policy_snapshot_revision = $3,
                         material_change = TRUE, requires_reconsent = TRUE,
                         updated_at = NOW()
                     WHERE policy_slug = $1 AND policy_version = $2",
                )
                .bind(&policy.policy_slug)
                .bind(version)
                .bind(&policy.policy_snapshot_revision)
                .execute(&mut *tx)
                .await?;
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
                sqlx::query(
                    "UPDATE cn_admin.policies
                     SET title = $2, body_markdown = $3, required = $4,
                         effective_date = $5::date, language = $6,
                         policy_snapshot_revision = $7,
                         material_change = TRUE, requires_reconsent = TRUE,
                         updated_at = NOW()
                     WHERE policy_slug = $1 AND policy_version = 1",
                )
                .bind(&policy.policy_slug)
                .bind(&policy.title)
                .bind(&policy.body_markdown)
                .bind(policy.required)
                .bind(&policy.effective_date)
                .bind(&policy.language)
                .bind(&policy.policy_snapshot_revision)
                .execute(&mut *tx)
                .await?;
                continue;
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
                 SET is_current = FALSE, retired_at = NOW(), updated_at = NOW()
                 WHERE policy_slug = $1 AND policy_version = $2",
            )
            .bind(&policy.policy_slug)
            .bind(version)
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query(
            "INSERT INTO cn_admin.policies
                (policy_slug, policy_version, title, body_markdown, required,
                 effective_date, language, is_current, policy_snapshot_revision,
                 material_change, requires_reconsent)
             VALUES ($1, $2, $3, $4, $5, $6::date, $7, TRUE, $8, $9, $9)",
        )
        .bind(&policy.policy_slug)
        .bind(policy.policy_version)
        .bind(&policy.title)
        .bind(&policy.body_markdown)
        .bind(policy.required)
        .bind(&policy.effective_date)
        .bind(&policy.language)
        .bind(&policy.policy_snapshot_revision)
        .bind(had_existing)
        .execute(&mut *tx)
        .await?;
    }
    for translation in policies
        .iter()
        .filter(|policy| policy.reference_translation)
    {
        let language = translation
            .language
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("reference translation language is required"))?;
        let revision = translation
            .translation_revision
            .filter(|revision| *revision > 0)
            .ok_or_else(|| anyhow::anyhow!("reference translation revision is required"))?;
        if translation.translation_of_version != Some(translation.policy_version) {
            bail!(
                "reference translation for `{}` must target authoritative version {}",
                translation.policy_slug,
                translation.policy_version
            );
        }
        let existing = sqlx::query(
            "SELECT translation_revision, title, body_markdown
             FROM cn_admin.policy_translations
             WHERE policy_slug = $1 AND policy_version = $2
               AND language = $3 AND is_current = TRUE
             FOR UPDATE",
        )
        .bind(&translation.policy_slug)
        .bind(translation.policy_version)
        .bind(language)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(existing) = existing {
            let stored_revision: i32 = existing.try_get("translation_revision")?;
            let stored_title: String = existing.try_get("title")?;
            let stored_body: String = existing.try_get("body_markdown")?;
            if stored_revision == revision
                && stored_title == translation.title
                && stored_body == translation.body_markdown
            {
                continue;
            }
            if revision <= stored_revision {
                bail!(
                    "reference translation `{}` language `{}` changed without a revision increase",
                    translation.policy_slug,
                    language
                );
            }
            sqlx::query(
                "UPDATE cn_admin.policy_translations
                 SET is_current = FALSE, retired_at = NOW(), updated_at = NOW()
                 WHERE policy_slug = $1 AND policy_version = $2
                   AND language = $3 AND translation_revision = $4",
            )
            .bind(&translation.policy_slug)
            .bind(translation.policy_version)
            .bind(language)
            .bind(stored_revision)
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query(
            "INSERT INTO cn_admin.policy_translations
                (policy_slug, policy_version, language, translation_revision,
                 title, body_markdown)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&translation.policy_slug)
        .bind(translation.policy_version)
        .bind(language)
        .bind(revision)
        .bind(&translation.title)
        .bind(&translation.body_markdown)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn accept_consents(
    pool: &PgPool,
    pubkey: &str,
    policy_slugs: &[String],
    policy_snapshot_revision: Option<&str>,
) -> Result<CommunityNodeConsentStatus> {
    let pubkey = normalize_pubkey(pubkey)?;
    let mut tx = pool.begin().await?;
    let current_snapshot = sqlx::query_scalar::<_, String>(
        "SELECT policy_snapshot_revision
         FROM cn_admin.policies
         WHERE is_current = TRUE AND required = TRUE
           AND policy_snapshot_revision IS NOT NULL
         LIMIT 1",
    )
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(current_snapshot) = current_snapshot.as_deref()
        && policy_snapshot_revision != Some(current_snapshot)
    {
        bail!("policy snapshot changed; reload policies before accepting");
    }
    let desired = if policy_slugs.is_empty() {
        sqlx::query(
            "SELECT policy_slug, policy_version
             FROM cn_admin.policies
             WHERE required = TRUE AND is_current = TRUE",
        )
        .fetch_all(&mut *tx)
        .await?
    } else {
        let mut records = Vec::new();
        for slug in normalize_slug_list(policy_slugs) {
            let row = sqlx::query(
                "SELECT policy_slug, policy_version
                 FROM cn_admin.policies
                 WHERE policy_slug = $1 AND is_current = TRUE",
            )
            .bind(&slug)
            .fetch_optional(&mut *tx)
            .await?;
            let Some(row) = row else {
                bail!("unknown policy slug `{slug}`");
            };
            records.push(row);
        }
        records
    };

    ensure_active_subscriber(&mut *tx, pubkey.as_str()).await?;
    for row in desired {
        let slug: String = row.try_get("policy_slug")?;
        let version: i32 = row.try_get("policy_version")?;
        sqlx::query(
            "INSERT INTO cn_user.policy_consents
                (subscriber_pubkey, policy_slug, policy_version,
                 policy_snapshot_revision, accepted_at)
             VALUES ($1, $2, $3, $4, NOW())
             ON CONFLICT (subscriber_pubkey, policy_slug, policy_version) DO UPDATE
             SET policy_snapshot_revision = EXCLUDED.policy_snapshot_revision,
                 accepted_at = EXCLUDED.accepted_at",
        )
        .bind(&pubkey)
        .bind(slug)
        .bind(version)
        .bind(policy_snapshot_revision)
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
