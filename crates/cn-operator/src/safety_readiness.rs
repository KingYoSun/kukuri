use kukuri_cn_safety::SafetyProviderCapability;

use crate::config::ResolvedConfig;
use crate::safety_config::{SafetyConfig, SafetyProviderEntry};

pub const PUBLIC_NODE_PROFILE: &str = "public-node";

/// readiness レポートが含む check id の単一の真実源。
///
/// 通常経路（`evaluate_public_node_readiness`）と safety セクション欠落経路
/// （`missing_safety_report`）の双方が、必ずこの集合を同じ順序で網羅する。ID で機械処理する
/// 消費側が経路ごとの差異に依存しないことを保証する（テストで固定）。
pub const READINESS_CHECK_IDS: [&str; 23] = [
    "safety_config_present",
    "safety_profile_public_node",
    "known_csam_provider_configured",
    "known_csam_provider_resolvable",
    "known_csam_provider_required",
    "classifier_providers_resolvable",
    "index_before_scan_disabled",
    "scan_error_fail_closed",
    "signed_moderation_events_enabled",
    "signing_key_secret_configured",
    "permanent_blob_storage_disabled",
    "blob_cache_legal_eviction_ready",
    "known_csam_credential_secret_configured",
    "provider_credential_valid",
    "scan_coverage_metrics_available",
    "indexer_worker_running",
    "indexer_scopes_opened",
    "indexer_ingest_fresh",
    "postgres_migrations_ready",
    "arcadedb_projection_ready",
    "index_truth_projection_consistent",
    "relation_analysis_recent",
    "supported_scopes_public_only",
];

/// 実行時（runtime / indexing）接続後にのみ確定できる判定項目の id 集合（#616）。
///
/// 静的評価（`evaluate_public_node_readiness`）ではこれらを `Unknown` として返し、
/// `cn-cli readiness` が実測結果で置換する。
pub const RUNTIME_CHECK_IDS: [&str; 10] = [
    "provider_credential_valid",
    "scan_coverage_metrics_available",
    "indexer_worker_running",
    "indexer_scopes_opened",
    "indexer_ingest_fresh",
    "postgres_migrations_ready",
    "arcadedb_projection_ready",
    "index_truth_projection_consistent",
    "relation_analysis_recent",
    "supported_scopes_public_only",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadinessStatus {
    Pass,
    Fail,
    Unknown,
}

impl ReadinessStatus {
    pub fn key(self) -> &'static str {
        match self {
            ReadinessStatus::Pass => "pass",
            ReadinessStatus::Fail => "fail",
            ReadinessStatus::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadinessCheck {
    pub id: &'static str,
    pub status: ReadinessStatus,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadinessReport {
    pub profile: String,
    pub checks: Vec<ReadinessCheck>,
}

impl ReadinessReport {
    /// 全 check が `Pass` のときだけ true（最終的な readiness 完了）。
    ///
    /// runtime 接続が必要な check（`provider_credential_valid` /
    /// `scan_coverage_metrics_available`）は本段階では常に `Unknown` を返すため、
    /// static config が完璧でも本関数は false を返す。runtime / indexing が接続されるまで
    /// public indexing 解禁を許さない fail-closed な最終判定として使う。
    pub fn is_ready(&self) -> bool {
        self.checks
            .iter()
            .all(|check| check.status == ReadinessStatus::Pass)
    }

    /// static に検査可能な check に `Fail` が無いか（`Unknown` は許容）。
    ///
    /// config から静的判定できる項目がすべて満たされているかを表す。`Unknown` は
    /// runtime 接続後に解決される未確定項目であり、ここでは blocking 失敗として扱わない。
    pub fn has_blocking_failures(&self) -> bool {
        self.checks
            .iter()
            .any(|check| check.status == ReadinessStatus::Fail)
    }

    /// static checks に `Fail` が無い（= 設定上の不備が無い）か。
    pub fn static_checks_pass(&self) -> bool {
        !self.has_blocking_failures()
    }

    pub fn fail_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|check| check.status == ReadinessStatus::Fail)
            .count()
    }

    pub fn unknown_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|check| check.status == ReadinessStatus::Unknown)
            .count()
    }
}

/// 実行時判定の結果を静的報告へ合成する（#616）。
///
/// `runtime` の各項目は、id が一致する既存項目を置換する。静的評価で `Unknown` のまま残る
/// 項目（`provider_credential_valid` / `scan_coverage_metrics_available` 等）を、実際の
/// 疎通確認・実測に基づく合格/不合格で確定させるための単一の合成点。既存 id に無い項目を
/// 渡された場合は末尾へ追加せずエラーにする（判定項目の集合は `READINESS_CHECK_IDS` が
/// 単一の真実源であり、勝手に増やさない）。
pub fn apply_runtime_checks(
    report: &mut ReadinessReport,
    runtime: Vec<ReadinessCheck>,
) -> anyhow::Result<()> {
    for incoming in runtime {
        let Some(existing) = report
            .checks
            .iter_mut()
            .find(|check| check.id == incoming.id)
        else {
            anyhow::bail!(
                "実行時判定の id `{}` は readiness の判定項目集合に存在しません",
                incoming.id
            );
        };
        *existing = incoming;
    }
    Ok(())
}

pub fn evaluate_public_node_readiness(
    config: &ResolvedConfig,
    requested_profile: &str,
) -> ReadinessReport {
    let profile = requested_profile.trim();
    let profile = if profile.is_empty() {
        PUBLIC_NODE_PROFILE
    } else {
        profile
    };
    let Some(safety) = config.raw.safety.as_ref() else {
        return missing_safety_report(profile);
    };

    let mut report = ReadinessReport {
        profile: profile.to_string(),
        checks: vec![
            pass(
                "safety_config_present",
                "operator-config.yaml に safety セクションがあります".to_string(),
            ),
            check_profile(safety, profile),
            check_known_provider_configured(safety),
            check_known_provider_resolvable(safety),
            check_known_provider_required(safety),
            check_classifier_providers_resolvable(safety),
            check_index_before_scan(safety),
            check_on_scan_error(safety),
            check_signed_events(safety),
            check_signing_key_secret(safety),
            check_no_permanent_blob_storage(safety),
            check_blob_cache_legal_eviction(config),
            check_known_provider_secret(safety),
        ],
    };
    report
        .checks
        .extend(RUNTIME_CHECK_IDS.iter().map(|&id| ReadinessCheck {
            id,
            status: ReadinessStatus::Unknown,
            detail: "runtime / indexing 接続後に `cn-cli readiness` が実測で確定する".to_string(),
        }));
    report
}

fn missing_safety_report(profile: &str) -> ReadinessReport {
    // 通常経路と同じ ID 集合を網羅する（READINESS_CHECK_IDS が単一の真実源）。
    // safety セクション自体が無いため、全項目を fail-closed に倒す。
    let checks = READINESS_CHECK_IDS
        .iter()
        .map(|&id| ReadinessCheck {
            id,
            status: ReadinessStatus::Fail,
            detail: "operator-config.yaml に safety セクションがありません".to_string(),
        })
        .collect();
    ReadinessReport {
        profile: profile.to_string(),
        checks,
    }
}

fn check_profile(safety: &SafetyConfig, requested_profile: &str) -> ReadinessCheck {
    let configured = safety.profile.as_deref().unwrap_or(PUBLIC_NODE_PROFILE);
    if requested_profile != PUBLIC_NODE_PROFILE {
        return fail(
            "safety_profile_public_node",
            format!("unsupported safety readiness profile: {requested_profile}"),
        );
    }
    if configured == PUBLIC_NODE_PROFILE {
        pass(
            "safety_profile_public_node",
            format!("safety profile is {configured}"),
        )
    } else {
        fail(
            "safety_profile_public_node",
            format!("safety.profile is {configured}; expected {PUBLIC_NODE_PROFILE}"),
        )
    }
}

fn check_known_provider_configured(safety: &SafetyConfig) -> ReadinessCheck {
    let Some(provider) = known_csam_provider(safety) else {
        let capability = SafetyProviderCapability::KnownCsamHashMatch;
        return fail(
            "known_csam_provider_configured",
            format!("missing provider for capability {capability:?}"),
        );
    };
    pass(
        "known_csam_provider_configured",
        format!(
            "provider={} capability=known_csam_hash_match",
            provider.provider
        ),
    )
}

/// runtime（`resolve_provider`、cn-core）が解決できる known_csam provider 実装名。
///
/// underscore 表記（`project_arachnid_shield`）は runtime と同様に hyphen へ正規化して
/// 受理する。ここに無い実装名は runtime 構築が fail-closed で失敗するため、readiness の
/// 段階で fail にして設定ミスを早期に検出する。
const RESOLVABLE_KNOWN_CSAM_PROVIDERS: [&str; 2] = ["mock", "project-arachnid-shield"];

fn check_known_provider_resolvable(safety: &SafetyConfig) -> ReadinessCheck {
    let Some(provider) = known_csam_provider(safety) else {
        return fail(
            "known_csam_provider_resolvable",
            "known CSAM provider is missing".to_string(),
        );
    };
    let normalized = provider.provider.trim().replace('_', "-");
    if RESOLVABLE_KNOWN_CSAM_PROVIDERS.contains(&normalized.as_str()) {
        pass(
            "known_csam_provider_resolvable",
            format!("provider={normalized} is resolvable by the runtime"),
        )
    } else {
        fail(
            "known_csam_provider_resolvable",
            format!(
                "provider `{}` is not a known implementation (supported: {})",
                provider.provider,
                RESOLVABLE_KNOWN_CSAM_PROVIDERS.join(" / ")
            ),
        )
    }
}

/// runtime（`resolve_provider`、cn-core）が general / unknown_csam slot で解決できる
/// classifier 系 provider 実装名（#420）。
const RESOLVABLE_CLASSIFIER_PROVIDERS: [&str; 2] = ["mock", "openai-compatible-vlm"];

/// general / unknown_csam slot の provider 実装名が runtime で解決可能かを検査する。
///
/// 両 slot とも任意（未構成は pass）。構成されている場合のみ、実装名が
/// `RESOLVABLE_CLASSIFIER_PROVIDERS` に含まれることを要求する（known-match 専用の
/// `project-arachnid-shield` はここでは解決できない）。
fn check_classifier_providers_resolvable(safety: &SafetyConfig) -> ReadinessCheck {
    let slots = [
        ("general", safety.providers.general.as_ref()),
        ("unknown_csam", safety.providers.unknown_csam.as_ref()),
    ];
    let mut resolved = Vec::new();
    for (slot, entry) in slots {
        let Some(entry) = entry else {
            continue;
        };
        let normalized = entry.provider.trim().replace('_', "-");
        if !RESOLVABLE_CLASSIFIER_PROVIDERS.contains(&normalized.as_str()) {
            return fail(
                "classifier_providers_resolvable",
                format!(
                    "provider `{}` for slot `{slot}` is not a known classifier implementation \
                     (supported: {})",
                    entry.provider,
                    RESOLVABLE_CLASSIFIER_PROVIDERS.join(" / ")
                ),
            );
        }
        resolved.push(format!("{slot}={normalized}"));
    }
    if resolved.is_empty() {
        pass(
            "classifier_providers_resolvable",
            "no classifier providers configured (general / unknown_csam slots are optional)"
                .to_string(),
        )
    } else {
        pass(
            "classifier_providers_resolvable",
            format!("resolvable: {}", resolved.join(", ")),
        )
    }
}

fn check_known_provider_required(safety: &SafetyConfig) -> ReadinessCheck {
    match known_csam_provider(safety) {
        Some(provider) if provider.required => pass(
            "known_csam_provider_required",
            "known CSAM provider is marked required".to_string(),
        ),
        Some(_) => fail(
            "known_csam_provider_required",
            "known CSAM provider must be required for public-node readiness".to_string(),
        ),
        None => fail(
            "known_csam_provider_required",
            "known CSAM provider is missing".to_string(),
        ),
    }
}

fn check_index_before_scan(safety: &SafetyConfig) -> ReadinessCheck {
    if safety.indexing.index_before_scan {
        fail(
            "index_before_scan_disabled",
            "safety.indexing.index_before_scan must be false".to_string(),
        )
    } else {
        pass(
            "index_before_scan_disabled",
            "index_before_scan=false".to_string(),
        )
    }
}

fn check_on_scan_error(safety: &SafetyConfig) -> ReadinessCheck {
    if safety.indexing.on_scan_error.allows_indexing() {
        fail(
            "scan_error_fail_closed",
            "safety.indexing.on_scan_error must not be allow".to_string(),
        )
    } else {
        pass(
            "scan_error_fail_closed",
            format!(
                "on_scan_error={} is fail-closed",
                safety.indexing.on_scan_error.key()
            ),
        )
    }
}

fn check_signed_events(safety: &SafetyConfig) -> ReadinessCheck {
    if safety.events.emit_signed_moderation_events {
        pass(
            "signed_moderation_events_enabled",
            "emit_signed_moderation_events=true".to_string(),
        )
    } else {
        fail(
            "signed_moderation_events_enabled",
            "signed moderation events must be enabled".to_string(),
        )
    }
}

fn check_signing_key_secret(safety: &SafetyConfig) -> ReadinessCheck {
    match safety.events.signing_key_secret_id.as_deref() {
        Some(secret_id) if !secret_id.trim().is_empty() => pass(
            "signing_key_secret_configured",
            format!("signing_key_secret_id={secret_id}"),
        ),
        Some(_) | None => fail(
            "signing_key_secret_configured",
            "safety.events.signing_key_secret_id is required for real-key moderation event signing"
                .to_string(),
        ),
    }
}

fn check_no_permanent_blob_storage(safety: &SafetyConfig) -> ReadinessCheck {
    if safety.storage.permanent_blob_storage {
        fail(
            "permanent_blob_storage_disabled",
            "safety.storage.permanent_blob_storage must be false".to_string(),
        )
    } else {
        pass(
            "permanent_blob_storage_disabled",
            "permanent_blob_storage=false".to_string(),
        )
    }
}

fn check_blob_cache_legal_eviction(config: &ResolvedConfig) -> ReadinessCheck {
    if config.blob_cache_enabled() {
        fail(
            "blob_cache_legal_eviction_ready",
            "features.blob_cache=true ですが、法的な送信防止対象を削除し再 cache を拒否する backend は未接続です。backend が実装されるまでは blob_cache を無効にしてください"
                .to_string(),
        )
    } else {
        pass(
            "blob_cache_legal_eviction_ready",
            "blob_cache=false のため永続 cache の削除・再取得拒否は不要です".to_string(),
        )
    }
}

fn check_known_provider_secret(safety: &SafetyConfig) -> ReadinessCheck {
    match known_csam_provider(safety).and_then(|provider| provider.credential_secret_id.as_deref())
    {
        Some(secret_id) if !secret_id.trim().is_empty() => pass(
            "known_csam_credential_secret_configured",
            format!("credential_secret_id={secret_id}"),
        ),
        Some(_) | None => fail(
            "known_csam_credential_secret_configured",
            "known CSAM provider credential_secret_id is required".to_string(),
        ),
    }
}

fn known_csam_provider(safety: &SafetyConfig) -> Option<&SafetyProviderEntry> {
    safety
        .providers
        .known_csam
        .as_ref()
        .filter(|provider| !provider.provider.trim().is_empty())
}

fn pass(id: &'static str, detail: String) -> ReadinessCheck {
    ReadinessCheck {
        id,
        status: ReadinessStatus::Pass,
        detail,
    }
}

fn fail(id: &'static str, detail: String) -> ReadinessCheck {
    ReadinessCheck {
        id,
        status: ReadinessStatus::Fail,
        detail,
    }
}
