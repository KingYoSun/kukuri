//! community node の admission（招待コード / whitelist / ban）。#383
//!
//! public community node の利用者を限定するための server-side enforcement。
//! これは node-local な「補助機能提供の可否」判断であり、kukuri network 全体からの
//! アカウント凍結ではない（`docs/architecture/p2p-first-community-node-responsibility-boundary.md`）。
//!
//! enforcement の適用順序（`evaluate_admission`）:
//! 1. `status='banned'` は mode に関わらず拒否する。
//! 2. 既存 `status='active'` subscriber は mode 変更後も通す。
//! 3. それ以外（未登録 pubkey）のみ mode を適用する。
//!    - `open`: admit
//!    - `whitelist`: allowlist 登録のみ admit
//!    - `invite`: allowlist 該当はコード不要 bypass、それ以外は有効コード必須

use std::fmt;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use sqlx::postgres::PgPool;

use crate::config::COMMUNITY_NODE_ADMISSION_SERVICE_NAME;
use crate::normalize::normalize_pubkey;

/// node 全体の入会モード。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdmissionMode {
    /// 署名できる誰でも admit（既定・後方互換）。
    #[default]
    Open,
    /// 有効な招待コード（または allowlist 該当）が必要。
    Invite,
    /// allowlist 登録済みのみ admit。
    Whitelist,
}

impl AdmissionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            AdmissionMode::Open => "open",
            AdmissionMode::Invite => "invite",
            AdmissionMode::Whitelist => "whitelist",
        }
    }
}

/// `cn_admin.service_configs` に保存する admission 設定。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdmissionConfig {
    #[serde(default)]
    pub mode: AdmissionMode,
}

/// admission 拒否の理由。`code()` は API へ返す安定コード。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdmissionRejection {
    Banned,
    InviteRequired,
    InviteInvalid,
    InviteExpired,
    InviteExhausted,
    InviteRevoked,
    NotAllowlisted,
}

impl AdmissionRejection {
    /// API 応答に載せる安定コード。
    pub fn code(&self) -> &'static str {
        match self {
            AdmissionRejection::Banned => "BANNED",
            AdmissionRejection::InviteRequired => "INVITE_REQUIRED",
            AdmissionRejection::InviteInvalid => "INVITE_INVALID",
            AdmissionRejection::InviteExpired => "INVITE_EXPIRED",
            AdmissionRejection::InviteExhausted => "INVITE_EXHAUSTED",
            AdmissionRejection::InviteRevoked => "INVITE_REVOKED",
            AdmissionRejection::NotAllowlisted => "NOT_ALLOWLISTED",
        }
    }

    /// 利用者向けメッセージ。BAN は network 凍結ではなく node-local な制限である旨を反映する。
    pub fn message(&self) -> &'static str {
        match self {
            AdmissionRejection::Banned => {
                "this community node is not providing support services to this account"
            }
            AdmissionRejection::InviteRequired => {
                "this community node requires an invite code to join"
            }
            AdmissionRejection::InviteInvalid => "the provided invite code is not valid",
            AdmissionRejection::InviteExpired => "the provided invite code has expired",
            AdmissionRejection::InviteExhausted => {
                "the provided invite code has reached its usage limit"
            }
            AdmissionRejection::InviteRevoked => "the provided invite code has been revoked",
            AdmissionRejection::NotAllowlisted => {
                "this community node only admits allowlisted accounts"
            }
        }
    }
}

impl fmt::Display for AdmissionRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.message())
    }
}

impl std::error::Error for AdmissionRejection {}

/// 招待コードの運営者向けサマリ（平文は含まない）。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InviteCodeSummary {
    pub code_hash: String,
    pub label: Option<String>,
    pub max_uses: Option<i32>,
    pub used_count: i32,
    pub expires_at: Option<i64>,
    pub revoked_at: Option<i64>,
    pub created_at: i64,
}

/// allowlist エントリ。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllowlistEntry {
    pub pubkey: String,
    pub label: Option<String>,
    pub created_at: i64,
}

/// banned subscriber エントリ。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BannedEntry {
    pub pubkey: String,
    pub created_at: i64,
}

/// 招待コード平文の SHA-256 hex digest。
pub fn invite_code_hash(plaintext: &str) -> String {
    let digest = Sha256::digest(plaintext.trim().as_bytes());
    hex::encode(digest)
}

/// 未設定なら `open` を seed する。既存値は変更しない（後方互換）。
pub(crate) async fn ensure_default_admission(pool: &PgPool) -> Result<()> {
    let config_json = serde_json::to_value(AdmissionConfig::default())?;
    sqlx::query(
        "INSERT INTO cn_admin.service_configs (service_name, version, config_json)
         VALUES ($1, 1, $2)
         ON CONFLICT (service_name) DO NOTHING",
    )
    .bind(COMMUNITY_NODE_ADMISSION_SERVICE_NAME)
    .bind(config_json)
    .execute(pool)
    .await?;
    Ok(())
}

/// 現在の admission 設定を読む。
///
/// 行が無い場合のみ既定（open）にフォールバックする（後方互換）。行はあるがパースできない
/// 場合はエラーを伝播して **fail-closed** にする。これにより、未知 mode を書く新しい cn-cli と
/// 旧 cn-user-api の版ずれ等で、enforcement が黙って open に降格することを防ぐ。
pub async fn load_admission_config(pool: &PgPool) -> Result<AdmissionConfig> {
    let value = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT config_json FROM cn_admin.service_configs WHERE service_name = $1",
    )
    .bind(COMMUNITY_NODE_ADMISSION_SERVICE_NAME)
    .fetch_optional(pool)
    .await?;
    match value {
        Some(value) => serde_json::from_value(value)
            .context("failed to parse admission config; refusing to fall back to open mode"),
        None => Ok(AdmissionConfig::default()),
    }
}

/// admission mode を設定する。
pub async fn set_admission_mode(pool: &PgPool, mode: AdmissionMode) -> Result<()> {
    let config_json = serde_json::to_value(AdmissionConfig { mode })?;
    sqlx::query(
        "INSERT INTO cn_admin.service_configs (service_name, version, config_json)
         VALUES ($1, 1, $2)
         ON CONFLICT (service_name) DO UPDATE
         SET version = cn_admin.service_configs.version + 1,
             config_json = EXCLUDED.config_json,
             updated_at = NOW()",
    )
    .bind(COMMUNITY_NODE_ADMISSION_SERVICE_NAME)
    .bind(config_json)
    .execute(pool)
    .await?;
    Ok(())
}

/// 招待コードを発行し、平文を返す（保存は hash のみ）。平文はこの戻り値でのみ得られる。
pub async fn issue_invite_code(
    pool: &PgPool,
    label: Option<&str>,
    max_uses: Option<i32>,
    expires_at: Option<DateTime<Utc>>,
) -> Result<String> {
    if let Some(max_uses) = max_uses
        && max_uses < 1
    {
        anyhow::bail!("max_uses must be at least 1 when specified");
    }
    let plaintext = uuid::Uuid::new_v4().simple().to_string();
    let code_hash = invite_code_hash(plaintext.as_str());
    let label = normalize_optional(label);
    sqlx::query(
        "INSERT INTO cn_admin.invite_codes (code_hash, label, max_uses, expires_at)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(&code_hash)
    .bind(label)
    .bind(max_uses)
    .bind(expires_at)
    .execute(pool)
    .await?;
    Ok(plaintext)
}

/// 招待コードを新着順で一覧する。
pub async fn list_invite_codes(pool: &PgPool) -> Result<Vec<InviteCodeSummary>> {
    let rows = sqlx::query(
        "SELECT code_hash, label, max_uses, used_count, expires_at, revoked_at, created_at
         FROM cn_admin.invite_codes
         ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|row| -> Result<InviteCodeSummary> {
            Ok(InviteCodeSummary {
                code_hash: row.try_get("code_hash")?,
                label: row.try_get("label")?,
                max_uses: row.try_get("max_uses")?,
                used_count: row.try_get("used_count")?,
                expires_at: row
                    .try_get::<Option<DateTime<Utc>>, _>("expires_at")?
                    .map(|value| value.timestamp()),
                revoked_at: row
                    .try_get::<Option<DateTime<Utc>>, _>("revoked_at")?
                    .map(|value| value.timestamp()),
                created_at: row.try_get::<DateTime<Utc>, _>("created_at")?.timestamp(),
            })
        })
        .collect()
}

/// 招待コード（平文）を取り消す。該当があれば true。
pub async fn revoke_invite_code(pool: &PgPool, plaintext: &str) -> Result<bool> {
    let code_hash = invite_code_hash(plaintext);
    let result = sqlx::query(
        "UPDATE cn_admin.invite_codes
         SET revoked_at = NOW()
         WHERE code_hash = $1 AND revoked_at IS NULL",
    )
    .bind(&code_hash)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// pubkey を allowlist に追加する。
pub async fn add_allowlist(pool: &PgPool, pubkey: &str, label: Option<&str>) -> Result<()> {
    let pubkey = normalize_pubkey(pubkey)?;
    let label = normalize_optional(label);
    sqlx::query(
        "INSERT INTO cn_admin.admission_allowlist (pubkey, label)
         VALUES ($1, $2)
         ON CONFLICT (pubkey) DO UPDATE
         SET label = EXCLUDED.label",
    )
    .bind(&pubkey)
    .bind(label)
    .execute(pool)
    .await?;
    Ok(())
}

/// pubkey を allowlist から削除する。該当があれば true。
pub async fn remove_allowlist(pool: &PgPool, pubkey: &str) -> Result<bool> {
    let pubkey = normalize_pubkey(pubkey)?;
    let result = sqlx::query("DELETE FROM cn_admin.admission_allowlist WHERE pubkey = $1")
        .bind(&pubkey)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// allowlist を一覧する。
pub async fn list_allowlist(pool: &PgPool) -> Result<Vec<AllowlistEntry>> {
    let rows = sqlx::query(
        "SELECT pubkey, label, created_at
         FROM cn_admin.admission_allowlist
         ORDER BY created_at DESC, pubkey ASC",
    )
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|row| -> Result<AllowlistEntry> {
            Ok(AllowlistEntry {
                pubkey: row.try_get("pubkey")?,
                label: row.try_get("label")?,
                created_at: row.try_get::<DateTime<Utc>, _>("created_at")?.timestamp(),
            })
        })
        .collect()
}

/// subscriber を ban する。未登録 pubkey の事前 ban は banned 行を upsert する。
/// 既存 active subscriber の ban は `require_bearer_identity` の status 再チェックで
/// 既存トークンも即時失効する。
///
/// `admitted` は変更しない。これにより、現メンバーを ban→unban すると member 資格が戻り、
/// 未参加のまま事前 ban→unban した pubkey は未参加（admitted=false）のままになる。
pub async fn ban_subscriber(pool: &PgPool, pubkey: &str) -> Result<()> {
    let pubkey = normalize_pubkey(pubkey)?;
    sqlx::query(
        "INSERT INTO cn_user.subscriber_accounts (subscriber_pubkey, status, admitted)
         VALUES ($1, 'banned', FALSE)
         ON CONFLICT (subscriber_pubkey) DO UPDATE
         SET status = 'banned'",
    )
    .bind(&pubkey)
    .execute(pool)
    .await?;
    Ok(())
}

/// ban を解除し active に戻す。該当があれば true。
///
/// `admitted` は維持する。未参加のまま事前 ban された pubkey（admitted=false）は unban 後も
/// 未参加のままなので、invite/whitelist mode では改めて招待コード/allowlist が必要になる
/// （unban が admission を迂回しない）。
pub async fn unban_subscriber(pool: &PgPool, pubkey: &str) -> Result<bool> {
    let pubkey = normalize_pubkey(pubkey)?;
    let result = sqlx::query(
        "UPDATE cn_user.subscriber_accounts
         SET status = 'active'
         WHERE subscriber_pubkey = $1 AND status = 'banned'",
    )
    .bind(&pubkey)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// banned subscriber を一覧する。
pub async fn list_banned(pool: &PgPool) -> Result<Vec<BannedEntry>> {
    let rows = sqlx::query(
        "SELECT subscriber_pubkey, created_at
         FROM cn_user.subscriber_accounts
         WHERE status = 'banned'
         ORDER BY created_at DESC, subscriber_pubkey ASC",
    )
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|row| -> Result<BannedEntry> {
            Ok(BannedEntry {
                pubkey: row.try_get("subscriber_pubkey")?,
                created_at: row.try_get::<DateTime<Utc>, _>("created_at")?.timestamp(),
            })
        })
        .collect()
}

/// admission を評価する。トランザクション内で呼び、invite redeem を subscriber 作成と原子化する。
///
/// 戻り値: 通過なら `Ok(())`。拒否なら `AdmissionRejection` を typed error として持つ
/// `anyhow::Error`（呼び出し側は downcast で 403 + code へマップできる）。DB error は
/// そのまま propagate する。
pub(crate) async fn evaluate_admission(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    pubkey: &str,
    mode: AdmissionMode,
    invite_code: Option<&str>,
) -> Result<()> {
    // 1. 既存 subscriber の状態を確認する。`admitted` で「現メンバー」と
    //    「未参加のまま ban/unban されただけの pubkey」を区別する。
    let existing = sqlx::query(
        "SELECT status, admitted
         FROM cn_user.subscriber_accounts
         WHERE subscriber_pubkey = $1",
    )
    .bind(pubkey)
    .fetch_optional(&mut **tx)
    .await?;

    if let Some(row) = existing {
        let status: String = row.try_get("status")?;
        let admitted: bool = row.try_get("admitted")?;
        match status.as_str() {
            // banned は mode に関わらず拒否する。
            "banned" => return Err(AdmissionRejection::Banned.into()),
            // 既に admission を通過した現メンバーは mode 変更後も通す。
            // status は active でも admitted=false（pre-ban を unban しただけ）の場合は
            // 下の mode 適用へ進み、改めて invite/whitelist を要求する。
            "active" if admitted => return Ok(()),
            _ => {}
        }
    }

    match mode {
        AdmissionMode::Open => Ok(()),
        AdmissionMode::Whitelist => {
            if is_allowlisted(tx, pubkey).await? {
                Ok(())
            } else {
                Err(AdmissionRejection::NotAllowlisted.into())
            }
        }
        AdmissionMode::Invite => {
            // allowlist 該当はコード不要 bypass。
            if is_allowlisted(tx, pubkey).await? {
                return Ok(());
            }
            let Some(code) = invite_code.map(str::trim).filter(|code| !code.is_empty()) else {
                return Err(AdmissionRejection::InviteRequired.into());
            };
            redeem_invite_code(tx, code).await
        }
    }
}

async fn is_allowlisted(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    pubkey: &str,
) -> Result<bool> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM cn_admin.admission_allowlist WHERE pubkey = $1)",
    )
    .bind(pubkey)
    .fetch_one(&mut **tx)
    .await?;
    Ok(exists)
}

/// 招待コードを原子的に消費する。
///
/// 行を `FOR UPDATE` でロックして状態を 1 度だけ読み、消費可否と拒否理由の **両方**を同じ
/// 状態（revoked / expires / max_uses / used_count）から導出する。これにより消費判定と
/// 返す理由コードがドリフトしない（予測条件の二重定義を避ける）。消費可能なときだけ used_count を
/// 加算する。行ロックにより同時 redeem の over-consume を防ぐ。
async fn redeem_invite_code(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    plaintext: &str,
) -> Result<()> {
    let code_hash = invite_code_hash(plaintext);
    let row = sqlx::query(
        "SELECT revoked_at, expires_at, max_uses, used_count
         FROM cn_admin.invite_codes
         WHERE code_hash = $1
         FOR UPDATE",
    )
    .bind(&code_hash)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(row) = row else {
        return Err(AdmissionRejection::InviteInvalid.into());
    };
    let revoked_at: Option<DateTime<Utc>> = row.try_get("revoked_at")?;
    let expires_at: Option<DateTime<Utc>> = row.try_get("expires_at")?;
    let max_uses: Option<i32> = row.try_get("max_uses")?;
    let used_count: i32 = row.try_get("used_count")?;

    // 消費可否と拒否理由を単一の状態から判定する。
    if revoked_at.is_some() {
        return Err(AdmissionRejection::InviteRevoked.into());
    }
    if expires_at.is_some_and(|expires_at| expires_at <= Utc::now()) {
        return Err(AdmissionRejection::InviteExpired.into());
    }
    if max_uses.is_some_and(|max_uses| used_count >= max_uses) {
        return Err(AdmissionRejection::InviteExhausted.into());
    }

    // ここまで来たら消費可能。行は FOR UPDATE でロック済みなので加算は安全。
    sqlx::query(
        "UPDATE cn_admin.invite_codes
         SET used_count = used_count + 1
         WHERE code_hash = $1",
    )
    .bind(&code_hash)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn normalize_optional(value: Option<&str>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admission_mode_serde_roundtrip() {
        for (mode, json) in [
            (AdmissionMode::Open, "\"open\""),
            (AdmissionMode::Invite, "\"invite\""),
            (AdmissionMode::Whitelist, "\"whitelist\""),
        ] {
            let serialized = serde_json::to_string(&mode).unwrap();
            assert_eq!(serialized, json);
            let deserialized: AdmissionMode = serde_json::from_str(json).unwrap();
            assert_eq!(deserialized, mode);
        }
    }

    #[test]
    fn admission_config_defaults_to_open() {
        assert_eq!(AdmissionConfig::default().mode, AdmissionMode::Open);
        // mode 省略の JSON は serde field default で open になる（既存設定の前方互換）。
        let parsed: AdmissionConfig = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(parsed.mode, AdmissionMode::Open);
    }

    #[test]
    fn admission_config_rejects_unknown_mode() {
        // 未知 mode はパース失敗する。load_admission_config はこれをそのまま伝播し
        // fail-closed にする（黙って open に降格しない）。
        let parsed: Result<AdmissionConfig, _> =
            serde_json::from_value(serde_json::json!({ "mode": "paid" }));
        assert!(parsed.is_err());
    }

    #[test]
    fn rejection_codes_are_stable() {
        assert_eq!(AdmissionRejection::Banned.code(), "BANNED");
        assert_eq!(AdmissionRejection::InviteRequired.code(), "INVITE_REQUIRED");
        assert_eq!(AdmissionRejection::InviteInvalid.code(), "INVITE_INVALID");
        assert_eq!(AdmissionRejection::InviteExpired.code(), "INVITE_EXPIRED");
        assert_eq!(
            AdmissionRejection::InviteExhausted.code(),
            "INVITE_EXHAUSTED"
        );
        assert_eq!(AdmissionRejection::InviteRevoked.code(), "INVITE_REVOKED");
        assert_eq!(AdmissionRejection::NotAllowlisted.code(), "NOT_ALLOWLISTED");
    }

    #[test]
    fn rejection_downcasts_through_anyhow() {
        // auth.rs -> cn-user-api の downcast マッピングが成立することを固定する。
        let error: anyhow::Error = AdmissionRejection::InviteExpired.into();
        let rejection = error
            .downcast_ref::<AdmissionRejection>()
            .expect("admission rejection downcast");
        assert_eq!(rejection.code(), "INVITE_EXPIRED");
    }

    #[test]
    fn invite_code_hash_is_sha256_hex_and_trims() {
        let hash = invite_code_hash("abc");
        // SHA-256("abc") の既知ベクタ。
        assert_eq!(
            hash,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        // 前後空白は無視する（CLI の貼り付け揺れ対策）。
        assert_eq!(invite_code_hash("  abc  "), hash);
    }
}
