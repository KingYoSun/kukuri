//! 公開ノードの準備完了判定（実行時情報込み。#616）。
//!
//! `cn-operator safety readiness` の静的判定に、実際の外部プロバイダへの疎通確認を合成して
//! `provider_credential_valid` を確定させる。疎通確認は合成データのみを送り、資格情報の値・
//! 応答本文・Match Data を出力にも保存にも含めない。結果は期限付きで Postgres に保存し、
//! 期限内の再実行では外部プロバイダを叩かない（外部負荷と発報回数の抑制）。

use std::path::Path;

use anyhow::{Context, Result, bail};
use chrono::{Duration, Utc};
use sqlx::PgPool;

use kukuri_cn_core::{
    ReadinessProbeRecord, initialize_database, list_readiness_probes, upsert_readiness_probe,
};
use kukuri_cn_operator::{
    ReadinessCheck, ReadinessStatus, apply_runtime_checks, evaluate_public_node_readiness,
    load_and_validate,
};
use kukuri_cn_safety_arachnid::{
    SYNTHETIC_PROBE_PDQ_HASH, ShieldClient, ShieldError, ShieldProviderConfig,
};
use kukuri_cn_safety_vlm::{VlmClient, VlmError, VlmProviderConfig, VlmScanInput};

/// 疎通確認で送る無害な合成文（判定結果そのものは使わない）。
const VLM_PROBE_TEXT: &str = "kukuri readiness probe: benign connectivity check";

/// 疎通確認 1 回の結果。detail に秘匿情報を含めない契約。
#[derive(Clone, Debug)]
struct ProbeOutcome {
    pass: bool,
    detail: String,
}

/// Project Arachnid Shield への疎通確認（合成 PDQ hash の送信）。
///
/// 認証・接続・応答解釈の成否だけを判定し、分類結果は使わない。エラーは HTTP 状態の
/// 区分（認証拒否 / 頻度制限 / プロバイダ側エラー / 時間切れ / 接続失敗 / 解釈失敗）へ
/// 写像し、応答本文を含めない（`ShieldError` 自体が本文を持たない）。
async fn probe_arachnid(client: &ShieldClient) -> ProbeOutcome {
    match client
        .scan_pdq(&[SYNTHETIC_PROBE_PDQ_HASH.to_string()])
        .await
    {
        Ok(_) => ProbeOutcome {
            pass: true,
            detail: "認証と応答受信に成功".to_string(),
        },
        Err(ShieldError::Unauthorized { status }) => ProbeOutcome {
            pass: false,
            detail: format!("認証拒否 (HTTP {status})"),
        },
        Err(ShieldError::Http { status: 429 }) => ProbeOutcome {
            pass: false,
            detail: "頻度制限 (HTTP 429)".to_string(),
        },
        Err(ShieldError::Http { status }) if status >= 500 => ProbeOutcome {
            pass: false,
            detail: format!("プロバイダ側エラー (HTTP {status})"),
        },
        Err(ShieldError::Http { status }) => ProbeOutcome {
            pass: false,
            detail: format!("予期しない応答 (HTTP {status})"),
        },
        Err(ShieldError::Timeout) => ProbeOutcome {
            pass: false,
            detail: "時間切れ".to_string(),
        },
        Err(ShieldError::Network { detail }) => ProbeOutcome {
            pass: false,
            detail: format!("接続失敗: {detail}"),
        },
        Err(ShieldError::Protocol { .. }) => ProbeOutcome {
            pass: false,
            detail: "応答の解釈に失敗".to_string(),
        },
    }
}

/// OpenAI-compatible な視覚言語モデルへの疎通確認（無害な合成文の送信）。
///
/// 接続先・モデルへの到達と、設定した応答形式の解析成功までを判定する。
async fn probe_vlm(client: &VlmClient) -> ProbeOutcome {
    let input = VlmScanInput {
        text: Some(VLM_PROBE_TEXT),
        media: None,
    };
    match client.moderate(&input).await {
        Ok(_) => ProbeOutcome {
            pass: true,
            detail: "接続と応答形式の解析に成功".to_string(),
        },
        Err(VlmError::Unauthorized { status }) => ProbeOutcome {
            pass: false,
            detail: format!("認証拒否 (HTTP {status})"),
        },
        Err(VlmError::Http { status: 429 }) => ProbeOutcome {
            pass: false,
            detail: "頻度制限 (HTTP 429)".to_string(),
        },
        Err(VlmError::Http { status }) if status >= 500 => ProbeOutcome {
            pass: false,
            detail: format!("プロバイダ側エラー (HTTP {status})"),
        },
        Err(VlmError::Http { status }) => ProbeOutcome {
            pass: false,
            detail: format!("予期しない応答 (HTTP {status})"),
        },
        Err(VlmError::Timeout) => ProbeOutcome {
            pass: false,
            detail: "時間切れ".to_string(),
        },
        Err(VlmError::Network { detail }) => ProbeOutcome {
            pass: false,
            detail: format!("接続失敗: {detail}"),
        },
        Err(VlmError::Protocol { .. }) => ProbeOutcome {
            pass: false,
            detail: "応答形式の解析に失敗".to_string(),
        },
    }
}

/// operator-config の providers 節から疎通確認の対象 (slot, 実装名) を集める。
fn configured_probe_slots(
    safety: &kukuri_cn_operator::SafetyConfig,
) -> Vec<(&'static str, String)> {
    let slots = [
        ("known_csam", safety.providers.known_csam.as_ref()),
        ("general", safety.providers.general.as_ref()),
        ("unknown_csam", safety.providers.unknown_csam.as_ref()),
    ];
    slots
        .into_iter()
        .filter_map(|(slot, entry)| {
            let entry = entry?;
            let normalized = entry.provider.trim().replace('_', "-");
            if normalized.is_empty() {
                return None;
            }
            Some((slot, normalized))
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn run(
    pool: &PgPool,
    config_path: &Path,
    profile: &str,
    probe_ttl_secs: u64,
    force_probe: bool,
    indexer_status_url: &str,
    ingest_max_age_secs: i64,
    relation_max_age_secs: i64,
) -> Result<()> {
    initialize_database(pool).await?;

    let yaml = std::fs::read_to_string(config_path)
        .with_context(|| format!("{} を読み込めません", config_path.display()))?;
    let resolved = load_and_validate(&yaml)?;
    let mut report = evaluate_public_node_readiness(&resolved, profile);

    let safety = resolved.raw.safety.clone().unwrap_or_default();
    let slots = configured_probe_slots(&safety);

    let cached = if force_probe {
        Vec::new()
    } else {
        list_readiness_probes(pool).await?
    };
    let now = Utc::now();
    let ttl = Duration::seconds(i64::try_from(probe_ttl_secs).unwrap_or(i64::MAX));

    // 同一の実装名は接続先も同一のため、疎通確認は実装名ごとに 1 回だけ行い
    // 該当する全 slot へ結果を写す（視覚言語モデルが general / unknown_csam の両方に
    // 構成される標準構成で、外部への発報を 1 回に抑える）。
    let mut fresh_by_provider: std::collections::BTreeMap<String, ProbeOutcome> =
        std::collections::BTreeMap::new();
    let mut slot_results: Vec<(String, String, ProbeOutcome)> = Vec::new();

    for (slot, provider) in &slots {
        // 期限内の保存結果があれば再利用する。
        if let Some(record) = cached.iter().find(|record| {
            record.provider_slot == *slot
                && record.provider == *provider
                && now - record.checked_at <= ttl
        }) {
            slot_results.push((
                (*slot).to_string(),
                provider.clone(),
                ProbeOutcome {
                    pass: record.pass,
                    detail: format!(
                        "{} (前回 {} の結果を再利用)",
                        record.detail,
                        record.checked_at.to_rfc3339()
                    ),
                },
            ));
            continue;
        }

        let outcome = if let Some(outcome) = fresh_by_provider.get(provider) {
            outcome.clone()
        } else {
            let outcome = match provider.as_str() {
                "project-arachnid-shield" => {
                    match ShieldProviderConfig::from_env()
                        .map_err(anyhow::Error::from)
                        .and_then(|config| ShieldClient::from_config(&config).map_err(Into::into))
                    {
                        Ok(client) => probe_arachnid(&client).await,
                        // 設定エラーの表示は env の名前のみ（値を含まない契約は provider 側）。
                        Err(error) => ProbeOutcome {
                            pass: false,
                            detail: format!("設定不備: {error}"),
                        },
                    }
                }
                "openai-compatible-vlm" => {
                    match VlmProviderConfig::from_env()
                        .map_err(anyhow::Error::from)
                        .and_then(|config| VlmClient::from_config(&config).map_err(Into::into))
                    {
                        Ok(client) => probe_vlm(&client).await,
                        Err(error) => ProbeOutcome {
                            pass: false,
                            detail: format!("設定不備: {error}"),
                        },
                    }
                }
                other => ProbeOutcome {
                    pass: false,
                    detail: format!("疎通確認の方法が未定義の実装名です: {other}"),
                },
            };
            fresh_by_provider.insert(provider.clone(), outcome.clone());
            outcome
        };

        upsert_readiness_probe(
            pool,
            &ReadinessProbeRecord {
                provider_slot: (*slot).to_string(),
                provider: provider.clone(),
                pass: outcome.pass,
                detail: outcome.detail.clone(),
                checked_at: now,
            },
        )
        .await?;
        slot_results.push(((*slot).to_string(), provider.clone(), outcome));
    }

    // provider_credential_valid の合成。対象が無い構成は fail-closed（公開ノードでは
    // known_csam が必須であり、静的判定側も fail になっているはず）。
    let credential_check = if slot_results.is_empty() {
        ReadinessCheck {
            id: "provider_credential_valid",
            status: ReadinessStatus::Fail,
            detail: "疎通確認の対象プロバイダが構成されていません".to_string(),
        }
    } else if slot_results.iter().all(|(_, _, outcome)| outcome.pass) {
        ReadinessCheck {
            id: "provider_credential_valid",
            status: ReadinessStatus::Pass,
            detail: summarize(&slot_results),
        }
    } else {
        ReadinessCheck {
            id: "provider_credential_valid",
            status: ReadinessStatus::Fail,
            detail: summarize(&slot_results),
        }
    };
    apply_runtime_checks(&mut report, vec![credential_check])?;

    // 走査網羅系の実行時判定（#616 T2）。収集の失敗は evaluate 側で不合格へ倒す。
    let findings = super::readiness_runtime::collect(
        pool,
        indexer_status_url,
        ingest_max_age_secs,
        relation_max_age_secs,
    )
    .await;
    apply_runtime_checks(&mut report, super::readiness_runtime::evaluate(&findings))?;

    println!(
        "readiness profile={} ready={} fail={} unknown={}",
        report.profile,
        report.is_ready(),
        report.fail_count(),
        report.unknown_count()
    );
    for check in &report.checks {
        println!("{}  {}  {}", check.status.key(), check.id, check.detail);
    }

    if report.has_blocking_failures() {
        bail!(
            "readiness に不合格の項目があります (fail={})",
            report.fail_count()
        );
    }
    if report.is_ready() {
        println!("OK: すべての readiness check を満たしています。");
    } else {
        println!(
            "NOTE: 残り {} 項目は runtime 接続後に確定します。",
            report.unknown_count()
        );
    }
    Ok(())
}

fn summarize(results: &[(String, String, ProbeOutcome)]) -> String {
    results
        .iter()
        .map(|(slot, provider, outcome)| {
            format!(
                "{slot}={} ({provider}: {})",
                if outcome.pass { "pass" } else { "fail" },
                outcome.detail
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use kukuri_cn_safety_arachnid::ShieldCredentials;
    use kukuri_cn_safety_vlm::{VlmCredentials, VlmResponseFormat};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn shield_client(base_url: &str, timeout_secs: u64) -> ShieldClient {
        let config = ShieldProviderConfig {
            api_base_url: base_url.to_string(),
            timeout: std::time::Duration::from_secs(timeout_secs),
            ..ShieldProviderConfig::default()
        };
        ShieldClient::new(&config, ShieldCredentials::new("probe-user", "probe-pass"))
            .expect("shield client")
    }

    fn vlm_client(base_url: &str, timeout_secs: u64) -> VlmClient {
        let config = VlmProviderConfig {
            api_base_url: base_url.to_string(),
            api_key_env: "UNUSED_TEST_VLM_KEY".to_string(),
            model: "test-org/test-guard".to_string(),
            response_format: VlmResponseFormat::Guard,
            timeout: std::time::Duration::from_secs(timeout_secs),
        };
        VlmClient::new(&config, VlmCredentials::anonymous()).expect("vlm client")
    }

    #[tokio::test]
    async fn arachnid_probe_distinguishes_success_auth_and_rate_limit() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/pdq"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "scanned_hashes": {
                    SYNTHETIC_PROBE_PDQ_HASH: {
                        "classification": "no-known-match"
                    }
                }
            })))
            .expect(1)
            .mount(&server)
            .await;
        let outcome = probe_arachnid(&shield_client(&server.uri(), 5)).await;
        assert!(outcome.pass, "detail: {}", outcome.detail);
        server.reset().await;

        Mock::given(method("POST"))
            .and(path("/v1/pdq"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;
        let outcome = probe_arachnid(&shield_client(&server.uri(), 5)).await;
        assert!(!outcome.pass);
        assert!(outcome.detail.contains("認証拒否"), "{}", outcome.detail);
        server.reset().await;

        Mock::given(method("POST"))
            .and(path("/v1/pdq"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&server)
            .await;
        let outcome = probe_arachnid(&shield_client(&server.uri(), 5)).await;
        assert!(!outcome.pass);
        assert!(outcome.detail.contains("頻度制限"), "{}", outcome.detail);
    }

    #[tokio::test]
    async fn arachnid_probe_reports_timeout_and_server_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/pdq"))
            .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(5)))
            .mount(&server)
            .await;
        let outcome = probe_arachnid(&shield_client(&server.uri(), 1)).await;
        assert!(!outcome.pass);
        assert!(outcome.detail.contains("時間切れ"), "{}", outcome.detail);
        server.reset().await;

        Mock::given(method("POST"))
            .and(path("/v1/pdq"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;
        let outcome = probe_arachnid(&shield_client(&server.uri(), 5)).await;
        assert!(!outcome.pass);
        assert!(
            outcome.detail.contains("プロバイダ側エラー"),
            "{}",
            outcome.detail
        );
    }

    #[tokio::test]
    async fn vlm_probe_passes_on_guard_response_and_fails_on_unparseable_body() {
        let server = MockServer::start().await;
        // guard 形式: 1 行目 safe/unsafe。解析成功 = 疎通確認の合格。
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "choices": [{
                    "message": { "role": "assistant", "content": "safe" },
                    "logprobs": { "content": [{
                        "token": "safe",
                        "logprob": -0.01,
                        "top_logprobs": [
                            { "token": "safe", "logprob": -0.01 },
                            { "token": "unsafe", "logprob": -4.5 }
                        ]
                    }]}
                }]
            })))
            .mount(&server)
            .await;
        let outcome = probe_vlm(&vlm_client(&server.uri(), 5)).await;
        assert!(outcome.pass, "detail: {}", outcome.detail);
        server.reset().await;

        // 解釈できない本文は不合格（安全側）。detail に本文を含めない。
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string("secret-body-must-not-leak not json"),
            )
            .mount(&server)
            .await;
        let outcome = probe_vlm(&vlm_client(&server.uri(), 5)).await;
        assert!(!outcome.pass);
        assert!(
            !outcome.detail.contains("secret-body-must-not-leak"),
            "応答本文が detail に漏れています: {}",
            outcome.detail
        );
    }
}
