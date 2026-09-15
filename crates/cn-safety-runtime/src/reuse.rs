//! 保存済み verdict の再利用判定（#1050）。
//!
//! cn-indexer は pass ごとに scope 内の全 subject を走査する。同じ内容を同じ scan 構成で何度も
//! provider に掛けると、VLM 呼び出しと moderation artifact（risk signal / signed event）が
//! pass の回数だけ増える。本 module は「内容と scan 構成が不変なら保存済み verdict をそのまま
//! 使ってよい」という判定を純関数として提供する。
//!
//! 再利用の鍵は 2 つの fingerprint で、どちらかが違えば再 scan する。
//! - `source_fingerprint`: subject の内容識別子。post は `objects/<id>/state` レコードの
//!   content hash（本文参照・添付参照・status を含む）、blob は blob hash そのもの。
//! - `scan_config_fingerprint`: policy と provider 構成の識別子
//!   （`SafetyOrchestrator::scan_config_fingerprint`）。
//!
//! fail-closed の保存済み verdict（scan failure / provider unavailable / unscanned、および
//! `hold`）は再利用しない。次の pass で必ず再試行し、provider 復旧時に allow へ更新できるように
//! する（#1050 INVAR-2）。

use kukuri_cn_safety::{ReasonCode, SafetyAction, SafetyVerdict};

/// 永続化層から読み戻した verdict（再利用判定の入力）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredVerdictRecord {
    /// verdict record id（index entry の FK 参照先）。
    pub id: String,
    /// 保存済み verdict。labels / provider_capability は永続化していないため空になり得る。
    /// index gate は `is_indexable()` だけを参照するので判定には影響しない。
    pub verdict: SafetyVerdict,
    /// 同じ scan で確定した descriptive 検索タグ（`allow` のみ非空）。
    pub derived_tags: Vec<String>,
    /// 保存時の内容 fingerprint。旧行（#1050 以前）は `None`。
    pub source_fingerprint: Option<String>,
    /// 保存時の scan 構成 fingerprint。旧行は `None`。
    pub scan_config_fingerprint: Option<String>,
}

/// verdict を保存するときに同伴させる再利用用メタデータ。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VerdictPersistMeta {
    pub source_fingerprint: Option<String>,
    pub scan_config_fingerprint: Option<String>,
    pub derived_tags: Vec<String>,
}

/// 今回の scan 対象の fingerprint。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReuseInputs<'a> {
    pub source_fingerprint: &'a str,
    pub scan_config_fingerprint: &'a str,
}

/// 再 scan が必要と判定した理由（metrics / log 用）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RescanReason {
    /// 保存済み verdict が無い（初回）。
    NoStoredVerdict,
    /// 保存済み verdict に fingerprint が無い（#1050 以前の行）。
    MissingFingerprint,
    /// 内容が変わった。
    SourceChanged,
    /// policy / provider 構成が変わった。
    ScanConfigChanged,
    /// 保存済み verdict が fail-closed（scan failure / provider unavailable / unscanned / hold）。
    HeldVerdict,
}

/// 再利用判定の結果。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReuseDecision {
    Reuse,
    Rescan(RescanReason),
}

/// scan の実行有無。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanDisposition {
    /// provider を呼んで新しく判定した。
    Fresh,
    /// 保存済み verdict を再利用した（provider 呼び出し・artifact 生成なし）。
    Reused,
}

/// risk signal 永続化の結果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistedSignal {
    /// 永続化側の signal id（既存行へ集約した場合はその id）。
    pub id: String,
    /// 新しい行を作ったか。既存の活性行 / cleared 行へ集約した場合は false。
    pub newly_created: bool,
}

/// 保存済み verdict が再利用に値するか（fail-closed 判定は再試行する）。
pub fn is_reusable_verdict(verdict: &SafetyVerdict) -> bool {
    !matches!(
        verdict.reason_code,
        ReasonCode::ScanFailed | ReasonCode::ProviderUnavailable | ReasonCode::Unscanned
    ) && verdict.action != SafetyAction::Hold
}

/// 保存済み verdict を再利用してよいか判定する純関数。
pub fn decide(stored: Option<&StoredVerdictRecord>, current: &ReuseInputs<'_>) -> ReuseDecision {
    let Some(stored) = stored else {
        return ReuseDecision::Rescan(RescanReason::NoStoredVerdict);
    };
    let (Some(source), Some(config)) = (
        stored.source_fingerprint.as_deref(),
        stored.scan_config_fingerprint.as_deref(),
    ) else {
        return ReuseDecision::Rescan(RescanReason::MissingFingerprint);
    };
    if config != current.scan_config_fingerprint {
        return ReuseDecision::Rescan(RescanReason::ScanConfigChanged);
    }
    if source != current.source_fingerprint {
        return ReuseDecision::Rescan(RescanReason::SourceChanged);
    }
    if !is_reusable_verdict(&stored.verdict) {
        return ReuseDecision::Rescan(RescanReason::HeldVerdict);
    }
    ReuseDecision::Reuse
}

/// 再 scan の結果が保存済み verdict から実質的に変わったか。
///
/// signed moderation event は「新しい判定」を記録するものなので、action / reason_code /
/// critical のいずれも変わらない再 scan では発行しない（#1050 AC-2）。保存済み verdict が無い
/// 場合は変化ありとみなす。
pub fn verdict_changed(before: Option<&SafetyVerdict>, after: &SafetyVerdict) -> bool {
    match before {
        None => true,
        Some(before) => {
            before.action != after.action
                || before.reason_code != after.reason_code
                || before.critical != after.critical
        }
    }
}
