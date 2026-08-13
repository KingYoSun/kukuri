//! relation distance opt-out の永続化と node-local 表示判定（ADR 0026 §2.6 / §6.3）。
//!
//! opt-out は privacy / block / graph 離脱ではない。本人が明示的に選択した場合だけ、
//! node policy より遠い相手との user / post surfacing を相互に抑制する。relation graph と
//! trust 入力は保持し、解除時に直ちに表示を復帰できるようにする。

use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use kukuri_cn_protocol::normalize::normalize_pubkey;
use kukuri_cn_trust::RelationStore;

/// opt-out を登録する（冪等。既存行の `opted_out_at` は初回値を保持）。
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

/// opt-out を解除する（冪等。行を削除して即時に表示を復帰する）。
pub async fn clear_relation_optout(pool: &PgPool, pubkey: &str) -> Result<()> {
    let pubkey = normalize_pubkey(pubkey)?;
    sqlx::query("DELETE FROM cn_trust.relation_optouts WHERE pubkey = $1")
        .bind(&pubkey)
        .execute(pool)
        .await?;
    Ok(())
}

/// opt-out 状態と設定時刻を読む。
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

/// DB 非依存の distance opt-out 判定。
///
/// proximity が未観測なら距離境界外として扱う。双方とも未選択、同一 user、または
/// 閾値以上の pair は抑制しない。
pub fn should_suppress_relation_pair(
    viewer_opted_out: bool,
    target_opted_out: bool,
    proximity_score: Option<f64>,
    min_proximity: f64,
) -> bool {
    if !(viewer_opted_out || target_opted_out) {
        return false;
    }
    match proximity_score {
        Some(score) if score.is_finite() && (0.0..=1.0).contains(&score) => score < min_proximity,
        _ => true,
    }
}

fn validate_min_proximity(min_proximity: f64) -> Result<()> {
    if !min_proximity.is_finite() || min_proximity <= 0.0 || min_proximity > 1.0 {
        bail!("relation distance opt-out min proximity must be within (0, 1]");
    }
    Ok(())
}

/// viewer / target の選択状態と観測済み proximity から相互抑制を判定する。
pub async fn relation_pair_is_suppressed(
    pool: &PgPool,
    viewer: &str,
    target: &str,
    proximity_score: Option<f64>,
    min_proximity: f64,
) -> Result<bool> {
    validate_min_proximity(min_proximity)?;
    let viewer = normalize_pubkey(viewer)?;
    let target = normalize_pubkey(target)?;
    if viewer == target {
        return Ok(false);
    }
    let selected: Vec<(String,)> =
        sqlx::query_as("SELECT pubkey FROM cn_trust.relation_optouts WHERE pubkey = ANY($1)")
            .bind(vec![viewer.clone(), target.clone()])
            .fetch_all(pool)
            .await?;
    let viewer_opted_out = selected.iter().any(|(pubkey,)| pubkey == &viewer);
    let target_opted_out = selected.iter().any(|(pubkey,)| pubkey == &target);
    Ok(should_suppress_relation_pair(
        viewer_opted_out,
        target_opted_out,
        proximity_score,
        min_proximity,
    ))
}

/// 候補 pubkey のうち viewer から表示可能なものだけを入力順で返す。
///
/// relation backend または DB の失敗はエラーとして返し、呼び出し側が surface 全体を
/// fail-closed にできるようにする。
pub async fn filter_relation_visible(
    pool: &PgPool,
    relation: &dyn RelationStore,
    viewer: &str,
    pubkeys: &[String],
    min_proximity: f64,
) -> Result<Vec<String>> {
    validate_min_proximity(min_proximity)?;
    let viewer = normalize_pubkey(viewer)?;
    let normalized: Vec<(String, String)> = pubkeys
        .iter()
        .map(|target| Ok((target.clone(), normalize_pubkey(target)?)))
        .collect::<Result<_>>()?;
    let mut pair_pubkeys: Vec<String> = normalized
        .iter()
        .map(|(_, target)| target.clone())
        .collect();
    pair_pubkeys.push(viewer.clone());
    pair_pubkeys.sort();
    pair_pubkeys.dedup();
    let selected: std::collections::HashSet<String> = sqlx::query_as::<_, (String,)>(
        "SELECT pubkey FROM cn_trust.relation_optouts WHERE pubkey = ANY($1)",
    )
    .bind(pair_pubkeys)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(pubkey,)| pubkey)
    .collect();
    let viewer_opted_out = selected.contains(&viewer);
    let mut visible = Vec::with_capacity(pubkeys.len());
    for (original, target) in normalized {
        if target == viewer || (!viewer_opted_out && !selected.contains(&target)) {
            visible.push(original);
            continue;
        }
        let proximity = relation.pairwise_proximity(&viewer, &target).await?;
        if !should_suppress_relation_pair(
            viewer_opted_out,
            selected.contains(&target),
            proximity.as_ref().map(|value| value.score),
            min_proximity,
        ) {
            visible.push(original);
        }
    }
    Ok(visible)
}

#[cfg(test)]
mod tests {
    use super::should_suppress_relation_pair;

    #[test]
    fn distance_optout_matrix_is_explicit_and_symmetric() {
        for (viewer, target) in [(true, false), (false, true), (true, true)] {
            assert!(should_suppress_relation_pair(
                viewer,
                target,
                Some(0.49),
                0.5
            ));
            assert!(should_suppress_relation_pair(viewer, target, None, 0.5));
            assert!(!should_suppress_relation_pair(
                viewer,
                target,
                Some(0.5),
                0.5
            ));
            assert!(!should_suppress_relation_pair(
                viewer,
                target,
                Some(0.9),
                0.5
            ));
        }
        assert!(!should_suppress_relation_pair(false, false, Some(0.1), 0.5));
        assert!(!should_suppress_relation_pair(false, false, None, 0.5));
        assert!(should_suppress_relation_pair(
            true,
            false,
            Some(f64::NAN),
            0.5
        ));
        assert!(should_suppress_relation_pair(true, false, Some(1.1), 0.5));
    }
}
