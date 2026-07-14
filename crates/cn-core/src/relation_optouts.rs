//! relation の「見えない」opt-out（ADR 0026 §2.6 / §6.3 Decision, #415）。
//!
//! user は自分を (a) 他者の discovery / surfacing、(b) 他者から見た relation read の双方から
//! 外せる。node-local な選択であり **可逆**（解除 = 行の削除）。social graph canonical の
//! 削除ではない。**trust には影響しない**（troll 判定回避の手段にしない — trust read は
//! 本 module を参照しない）。
//!
//! 適用点は read surface（`cn-user-api`）: relation read / neighbors（discovery）の結果から
//! opt-out 済み pubkey を落とす。relation graph（ArcadeDB）からの物理削除はしない
//! （derived state であり、opt-out 解除で即座に見え直せる = 可逆性の実装）。

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use kukuri_cn_protocol::normalize::normalize_pubkey;

/// opt-out を登録する（冪等。既に opt-out 済みなら何もしない = opted_out_at は初回を保持）。
pub async fn set_relation_optout(pool: &PgPool, pubkey: &str) -> Result<()> {
    let pubkey = normalize_pubkey(pubkey)?;
    sqlx::query(
        "INSERT INTO cn_trust.relation_optouts (pubkey) VALUES ($1)
         ON CONFLICT (pubkey) DO NOTHING",
    )
    .bind(&pubkey)
    .execute(pool)
    .await?;
    Ok(())
}

/// opt-out を解除する（冪等。可逆性の実装 = 行の削除）。
pub async fn clear_relation_optout(pool: &PgPool, pubkey: &str) -> Result<()> {
    let pubkey = normalize_pubkey(pubkey)?;
    sqlx::query("DELETE FROM cn_trust.relation_optouts WHERE pubkey = $1")
        .bind(&pubkey)
        .execute(pool)
        .await?;
    Ok(())
}

/// opt-out 状態の read（説明可能性: いつから見えなくなったかを返す）。
pub async fn get_relation_optout(pool: &PgPool, pubkey: &str) -> Result<Option<DateTime<Utc>>> {
    let pubkey = normalize_pubkey(pubkey)?;
    let row: Option<(DateTime<Utc>,)> =
        sqlx::query_as("SELECT opted_out_at FROM cn_trust.relation_optouts WHERE pubkey = $1")
            .bind(&pubkey)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|(at,)| at))
}

/// opt-out 済みか。
pub async fn is_relation_opted_out(pool: &PgPool, pubkey: &str) -> Result<bool> {
    Ok(get_relation_optout(pool, pubkey).await?.is_some())
}

/// 候補 pubkey 列のうち opt-out **していない** ものだけを返す（neighbors / discovery の
/// 結果 filter 用。順序は入力を保つ）。
pub async fn filter_relation_visible(pool: &PgPool, pubkeys: &[String]) -> Result<Vec<String>> {
    if pubkeys.is_empty() {
        return Ok(Vec::new());
    }
    let opted_out: Vec<(String,)> =
        sqlx::query_as("SELECT pubkey FROM cn_trust.relation_optouts WHERE pubkey = ANY($1)")
            .bind(pubkeys)
            .fetch_all(pool)
            .await?;
    let opted_out: std::collections::HashSet<String> =
        opted_out.into_iter().map(|(p,)| p).collect();
    Ok(pubkeys
        .iter()
        .filter(|p| !opted_out.contains(*p))
        .cloned()
        .collect())
}
