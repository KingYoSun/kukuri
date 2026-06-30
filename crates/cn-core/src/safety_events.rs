//! signed moderation event / risk signal の永続化と visibility 配布境界（#405）。
//!
//! community node は自分の authority scope 内の判断を signed moderation event として保存・配布でき、
//! risk signal を trustness / relation 反映のために保存する。いずれも issuer node の advisory であり
//! network-wide command ではない（`docs/architecture/moderation-event-trust-semantics.md`）。
//!
//! 配布境界は visibility（`local` / `subscribed_nodes` / `public`）で決まる。
//! - `local` は issuer node の外へ出さない（配布クエリは返さない）。
//! - `subscribed_nodes` は購読 node に配布する。
//! - `public` は公開 advisory。
//!
//! suspected unknown CSAM / CSE は `local` 既定であり、誤検知を public advisory として拡散しない。
//! risk signal は `expires_at` 失効後は配布対象から除外する。
//!
//! enum 列は `cn-safety` の serde 表現（snake_case）と一致させるため、serde を経由して文字列化・
//! 復元する。これにより列値と canonical 表現の drift を防ぎ、ロード後も moderation event の署名検証が
//! 通る（body を列から型として復元し、`canonical_bytes()` が決定論的に再シリアライズする）。

use anyhow::{Context, Result, anyhow, bail};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use sqlx::Row;
use sqlx::postgres::{PgPool, PgRow};
use uuid::Uuid;

use kukuri_cn_safety::event::{ModerationEventBody, SignedModerationEvent};
use kukuri_cn_safety::verdict::SafetyLabel;
use kukuri_cn_safety::{AppealStatus, RiskSignalTarget, SafetyRiskSignal};
use kukuri_cn_safety_runtime::verify_signed_event;

/// 配布クエリの受け手区分。`local` はどの audience にも配布しない。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DistributionAudience {
    /// この node を trust input として購読している node。`subscribed_nodes` と `public` を見る。
    SubscribedNodes,
    /// 公開 advisory の受け手。`public` のみを見る。
    Public,
}

impl DistributionAudience {
    /// この audience に配布してよい visibility 文字列の集合。
    fn allowed_visibilities(self) -> Vec<String> {
        match self {
            DistributionAudience::SubscribedNodes => {
                vec!["subscribed_nodes".to_string(), "public".to_string()]
            }
            DistributionAudience::Public => vec!["public".to_string()],
        }
    }
}

/// 永続化された signed moderation event。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredModerationEvent {
    /// 復元した署名済み event（body + signature）。ロード後も署名検証できる。
    pub event: SignedModerationEvent,
    /// 永続化時刻（署名対象ではない）。
    pub persisted_at: DateTime<Utc>,
}

/// 永続化された risk signal。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredRiskSignal {
    /// 永続化側が採番した id。
    pub id: String,
    /// この signal を保持する issuer node。
    pub issuer_node_id: String,
    /// risk signal 本体。
    pub signal: SafetyRiskSignal,
    /// 永続化時刻。
    pub persisted_at: DateTime<Utc>,
}

/// signed moderation event を保存する（event id で冪等）。
///
/// 同一 id が既に存在する場合は上書きせず（最初の writer が権威）、既存レコードを返す。
/// `target_id` が空 / 空白の event は保存しない。
///
/// 保存前に署名を検証する（trust boundary）。`body.issuer_node_id` の公開鍵で canonical digest の
/// schnorr 署名を検証し、改竄 / 別鍵 / issuer 詐称の event は保存しない。これにより、配布クエリが
/// visibility だけで返すレコードが常に検証済みであることを保証する。
pub async fn persist_signed_moderation_event(
    pool: &PgPool,
    event: &SignedModerationEvent,
) -> Result<StoredModerationEvent> {
    let body = &event.body;
    if body.id.trim().is_empty() {
        bail!("moderation event id must not be empty");
    }
    if body.target_id.trim().is_empty() {
        bail!("moderation event target_id must not be empty");
    }
    verify_signed_event(event)
        .map_err(|err| anyhow!("refusing to persist unverified moderation event: {err}"))?;

    let labels =
        serde_json::to_value(&body.labels).context("failed to encode moderation labels")?;
    sqlx::query(
        "INSERT INTO cn_safety.signed_moderation_events
            (id, issuer_node_id, target_type, target_id, action, reason_code, severity, basis,
             visibility, confidence, policy_version, labels, signature, event_created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(&body.id)
    .bind(&body.issuer_node_id)
    .bind(to_db_enum(&body.target_type)?)
    .bind(&body.target_id)
    .bind(to_db_enum(&body.action)?)
    .bind(to_db_enum(&body.reason_code)?)
    .bind(to_db_enum(&body.severity)?)
    .bind(to_db_enum(&body.basis)?)
    .bind(to_db_enum(&body.visibility)?)
    .bind(body.confidence.map(i16::from))
    .bind(&body.policy_version)
    .bind(labels)
    .bind(&event.signature)
    .bind(&body.created_at)
    .execute(pool)
    .await?;

    // 冪等のため、保存後は常に id で再取得して権威レコードを返す。
    get_signed_moderation_event(pool, &body.id)
        .await?
        .context("persisted moderation event disappeared")
}

/// signed moderation event を id で取得する。
pub async fn get_signed_moderation_event(
    pool: &PgPool,
    id: &str,
) -> Result<Option<StoredModerationEvent>> {
    let row = sqlx::query(
        "SELECT id, issuer_node_id, target_type, target_id, action, reason_code, severity, basis,
                visibility, confidence, policy_version, labels, signature, event_created_at, persisted_at
         FROM cn_safety.signed_moderation_events
         WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    row.as_ref().map(moderation_event_from_row).transpose()
}

/// signed moderation event を新着順で取得する（運営者の監査用。visibility を問わない）。
pub async fn list_signed_moderation_events(
    pool: &PgPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<StoredModerationEvent>> {
    let rows = sqlx::query(
        "SELECT id, issuer_node_id, target_type, target_id, action, reason_code, severity, basis,
                visibility, confidence, policy_version, labels, signature, event_created_at, persisted_at
         FROM cn_safety.signed_moderation_events
         ORDER BY persisted_at DESC
         LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    rows.iter().map(moderation_event_from_row).collect()
}

/// 配布境界に従って配布可能な signed moderation event を返す。
///
/// `local` は決して返さない。audience が `SubscribedNodes` なら `subscribed_nodes` + `public`、
/// `Public` なら `public` のみ。
pub async fn list_distributable_moderation_events(
    pool: &PgPool,
    audience: DistributionAudience,
    limit: i64,
    offset: i64,
) -> Result<Vec<StoredModerationEvent>> {
    let rows = sqlx::query(
        "SELECT id, issuer_node_id, target_type, target_id, action, reason_code, severity, basis,
                visibility, confidence, policy_version, labels, signature, event_created_at, persisted_at
         FROM cn_safety.signed_moderation_events
         WHERE visibility = ANY($1)
         ORDER BY persisted_at DESC
         LIMIT $2 OFFSET $3",
    )
    .bind(audience.allowed_visibilities())
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    rows.iter().map(moderation_event_from_row).collect()
}

/// risk signal を保存する。`target_id` が空 / 空白なら保存しない。新しい id を採番して返す。
pub async fn persist_risk_signal(
    pool: &PgPool,
    issuer_node_id: &str,
    signal: &SafetyRiskSignal,
) -> Result<StoredRiskSignal> {
    if issuer_node_id.trim().is_empty() {
        bail!("risk signal issuer_node_id must not be empty");
    }
    if signal.target_id.trim().is_empty() {
        bail!("risk signal target_id must not be empty");
    }
    let id = Uuid::new_v4().to_string();
    let row = sqlx::query(
        "INSERT INTO cn_safety.risk_signals
            (id, issuer_node_id, target, target_id, category, severity, basis, visibility,
             confidence, expires_at, appeal_status)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
         RETURNING id, issuer_node_id, target, target_id, category, severity, basis, visibility,
                   confidence, expires_at, appeal_status, persisted_at",
    )
    .bind(&id)
    .bind(issuer_node_id)
    .bind(to_db_enum(&signal.target)?)
    .bind(&signal.target_id)
    .bind(to_db_enum(&signal.category)?)
    .bind(to_db_enum(&signal.severity)?)
    .bind(to_db_enum(&signal.basis)?)
    .bind(to_db_enum(&signal.visibility)?)
    .bind(signal.confidence.map(i16::from))
    .bind(signal.expires_at.as_deref())
    .bind(signal.appeal_status.map(|s| to_db_enum(&s)).transpose()?)
    .fetch_one(pool)
    .await?;
    risk_signal_from_row(&row)
}

/// risk signal を id で取得する。
pub async fn get_risk_signal(pool: &PgPool, id: &str) -> Result<Option<StoredRiskSignal>> {
    let row = sqlx::query(
        "SELECT id, issuer_node_id, target, target_id, category, severity, basis, visibility,
                confidence, expires_at, appeal_status, persisted_at
         FROM cn_safety.risk_signals
         WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    row.as_ref().map(risk_signal_from_row).transpose()
}

/// 対象ごとの risk signal を新着順で取得する（visibility を問わない、node-local な参照）。
pub async fn list_risk_signals_for_target(
    pool: &PgPool,
    target: RiskSignalTarget,
    target_id: &str,
) -> Result<Vec<StoredRiskSignal>> {
    let rows = sqlx::query(
        "SELECT id, issuer_node_id, target, target_id, category, severity, basis, visibility,
                confidence, expires_at, appeal_status, persisted_at
         FROM cn_safety.risk_signals
         WHERE target = $1 AND target_id = $2
         ORDER BY persisted_at DESC",
    )
    .bind(to_db_enum(&target)?)
    .bind(target_id)
    .fetch_all(pool)
    .await?;
    rows.iter().map(risk_signal_from_row).collect()
}

/// 配布境界に従って配布可能な risk signal を返す。
///
/// `local` は返さず、audience に応じて `subscribed_nodes` / `public` を返す。さらに `now_rfc3339`
/// 時点で `expires_at` が失効している signal は除外する（`expires_at` NULL は無期限で残る）。
pub async fn list_distributable_risk_signals(
    pool: &PgPool,
    audience: DistributionAudience,
    now_rfc3339: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<StoredRiskSignal>> {
    let rows = sqlx::query(
        "SELECT id, issuer_node_id, target, target_id, category, severity, basis, visibility,
                confidence, expires_at, appeal_status, persisted_at
         FROM cn_safety.risk_signals
         WHERE visibility = ANY($1)
           AND (expires_at IS NULL OR expires_at::timestamptz > $2::timestamptz)
         ORDER BY persisted_at DESC
         LIMIT $3 OFFSET $4",
    )
    .bind(audience.allowed_visibilities())
    .bind(now_rfc3339)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    rows.iter().map(risk_signal_from_row).collect()
}

fn moderation_event_from_row(row: &PgRow) -> Result<StoredModerationEvent> {
    let confidence: Option<i16> = row.try_get("confidence")?;
    let labels_value: Value = row.try_get("labels")?;
    let labels: Vec<SafetyLabel> =
        serde_json::from_value(labels_value).context("invalid stored moderation labels")?;
    let body = ModerationEventBody {
        id: row.try_get("id")?,
        issuer_node_id: row.try_get("issuer_node_id")?,
        target_type: from_db_enum("target_type", &row.try_get::<String, _>("target_type")?)?,
        target_id: row.try_get("target_id")?,
        action: from_db_enum("action", &row.try_get::<String, _>("action")?)?,
        labels,
        reason_code: from_db_enum("reason_code", &row.try_get::<String, _>("reason_code")?)?,
        severity: from_db_enum("severity", &row.try_get::<String, _>("severity")?)?,
        confidence: confidence.map(|v| v as u8),
        basis: from_db_enum("basis", &row.try_get::<String, _>("basis")?)?,
        visibility: from_db_enum("visibility", &row.try_get::<String, _>("visibility")?)?,
        policy_version: row.try_get("policy_version")?,
        created_at: row.try_get("event_created_at")?,
    };
    Ok(StoredModerationEvent {
        event: SignedModerationEvent {
            body,
            signature: row.try_get("signature")?,
        },
        persisted_at: row.try_get("persisted_at")?,
    })
}

fn risk_signal_from_row(row: &PgRow) -> Result<StoredRiskSignal> {
    let confidence: Option<i16> = row.try_get("confidence")?;
    let appeal_status: Option<String> = row.try_get("appeal_status")?;
    let appeal_status: Option<AppealStatus> = appeal_status
        .map(|s| from_db_enum("appeal_status", &s))
        .transpose()?;
    let signal = SafetyRiskSignal {
        target: from_db_enum("target", &row.try_get::<String, _>("target")?)?,
        target_id: row.try_get("target_id")?,
        category: from_db_enum("category", &row.try_get::<String, _>("category")?)?,
        severity: from_db_enum("severity", &row.try_get::<String, _>("severity")?)?,
        basis: from_db_enum("basis", &row.try_get::<String, _>("basis")?)?,
        confidence: confidence.map(|v| v as u8),
        visibility: from_db_enum("visibility", &row.try_get::<String, _>("visibility")?)?,
        expires_at: row.try_get("expires_at")?,
        appeal_status,
    };
    Ok(StoredRiskSignal {
        id: row.try_get("id")?,
        issuer_node_id: row.try_get("issuer_node_id")?,
        signal,
        persisted_at: row.try_get("persisted_at")?,
    })
}

/// `cn-safety` の snake_case enum を DB 列文字列へ写す。
fn to_db_enum<T: Serialize>(value: &T) -> Result<String> {
    match serde_json::to_value(value).context("failed to encode enum value")? {
        Value::String(s) => Ok(s),
        other => bail!("expected snake_case string enum, got {other}"),
    }
}

/// DB 列文字列を `cn-safety` の snake_case enum へ戻す。
fn from_db_enum<T: DeserializeOwned>(field: &str, value: &str) -> Result<T> {
    serde_json::from_value(Value::String(value.to_string()))
        .with_context(|| format!("invalid stored `{field}` value `{value}`"))
}
