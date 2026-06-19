//! operator config から運営者向け文書群を決定論的に生成する。
//!
//! 出力は wall-clock 非依存（version は config 由来）であり、同じ config からは同じ出力が得られる。
//!
//! Phase A / Phase B の分離:
//! - 運用中の開示（外部送信表示・データ取扱い）は Available かつ有効な capability のみに基づく。
//! - Planned（計画中・未提供）capability は、各文書で明示的に「計画中」として分離して記述し、
//!   運用中であるかのような開示には含めない。

use std::fmt::Write as _;

use crate::capability::{Availability, Capability, ExternalDestination};
use crate::config::ResolvedConfig;
use crate::manifest::{build_manifest, render_manifest};

/// すべての生成文書に付す共通の注記。
const LEGAL_DISCLAIMER: &str = "> 注記: この文書は operator config から自動生成された下書きであり、法的助言ではありません。\n\
> 最終的な内容・適法性の判断は、運営者自身および必要に応じて総合通信局・弁護士等の専門家への確認が必要です。";

const MANIFEST_FILE: &str = "server-manifest.json";

/// 生成された 1 ファイル。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedFile {
    pub filename: String,
    pub content: String,
}

/// 有効かつ Available な capability。
fn available_enabled(config: &ResolvedConfig) -> Vec<Capability> {
    config
        .enabled_capabilities()
        .into_iter()
        .filter(|c| c.availability() == Availability::Available)
        .collect()
}

/// 有効かつ Planned な capability。
fn planned_enabled(config: &ResolvedConfig) -> Vec<Capability> {
    config.enabled_planned_capabilities()
}

/// 有効な Available capability に基づく外部送信先（重複排除、Capability::ALL 順）。
fn external_destinations(config: &ResolvedConfig) -> Vec<ExternalDestination> {
    let mut dests = vec![ExternalDestination::CommunityServer];
    for cap in available_enabled(config) {
        if let Some(dest) = cap.meta().external_transmission
            && !dests.contains(&dest)
        {
            dests.push(dest);
        }
    }
    dests
}

/// 文書ヘッダ（タイトル + 運営者情報 + 注記）。
fn header(config: &ResolvedConfig, title: &str) -> String {
    let s = &config.raw.server;
    format!(
        "# {title}\n\n\
         - 運営者: {operator}\n\
         - サーバー: {domain}\n\
         - 所在国: {country}\n\
         - manifest version: {version}\n\n\
         {disclaimer}\n",
        title = title,
        operator = s.operator_name,
        domain = s.domain,
        country = s.country,
        version = config.raw.manifest.manifest_version,
        disclaimer = LEGAL_DISCLAIMER,
    )
}

/// 計画中 capability があれば、それを明示する共通セクション。
fn planned_section(config: &ResolvedConfig) -> String {
    let planned = planned_enabled(config);
    if planned.is_empty() {
        return String::new();
    }
    let mut s = String::new();
    let _ = writeln!(s, "\n## 計画中（この配布物では未提供）の capability\n");
    let _ = writeln!(
        s,
        "以下の capability は config 上で宣言されていますが、現行の community node 実装では提供されていません。"
    );
    let _ = writeln!(
        s,
        "そのため、本文書では「運用中の機能」としては扱わず、将来提供する予定の設計（spec）として記載します。\n"
    );
    for cap in planned {
        let m = cap.meta();
        let _ = writeln!(
            s,
            "- **{}**（{}）: {}",
            m.display_name,
            Availability::Planned.label_ja(),
            m.purpose
        );
    }
    s
}

// ---------------------------------------------------------------------------
// 各文書ジェネレータ
// ---------------------------------------------------------------------------

fn gen_network_diagram(config: &ResolvedConfig) -> String {
    let mut s = header(config, "ネットワーク構成説明");
    let _ = writeln!(s, "\n## 通信経路の基本優先度\n");
    let _ = writeln!(
        s,
        "kukuri の基本通信優先度は `Direct P2P -> Relay Supported P2P -> Relay Fallback` です。\
         community node はこの経路を補助する層であり、ユーザーの所属先（home server）ではありません。\n"
    );
    let _ = writeln!(s, "## 有効な接続補助 capability\n");
    let _ = writeln!(s, "```text");
    let _ = writeln!(s, "client");
    let _ = writeln!(s, "  |");
    for cap in available_enabled(config) {
        let _ = writeln!(s, "  +-- {} ({})", cap.meta().display_name, cap.key());
    }
    let _ = writeln!(s, "```\n");

    if config.enabled(Capability::IrohRelay) || config.enabled(Capability::TrafficRelayFallback) {
        let _ = writeln!(s, "## relay に関する補足\n");
        let _ = writeln!(
            s,
            "iroh relay / traffic relay fallback が有効です。これらは単なる signaling ではなく、\
             Direct / Relay Supported P2P が成立しない場合に、暗号化済みであっても実 traffic が relay を\
             経由し得ます。届出要否は構成と所在地に依存するため、別途確認してください。\n"
        );
    }

    // manifest の authority scope / P2P boundary を文書へ反映する。
    let manifest = build_manifest(config);
    let _ = writeln!(s, "## node role と責任境界 (authority scope)\n");
    let _ = writeln!(
        s,
        "- node role: `{}`",
        serde_json::to_value(manifest.node_role)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "community-node".to_string())
    );
    let _ = writeln!(s, "\n本ノードが責任を負う範囲 (applies_to):\n");
    for item in &manifest.authority_scope.applies_to {
        let _ = writeln!(s, "- `{item}`");
    }
    let _ = writeln!(s, "\n本ノードが責任を負わない範囲 (does_not_apply_to):\n");
    for item in &manifest.authority_scope.does_not_apply_to {
        let _ = writeln!(s, "- `{item}`");
    }
    let _ = writeln!(s, "\n## P2P boundary\n");
    let _ = writeln!(
        s,
        "本ノードは以下のいずれの権威も持ちません（kukuri の P2P-first 設計の不変条件）。\n"
    );
    let _ = writeln!(s, "- identity authority: false");
    let _ = writeln!(s, "- profile canonical store: false");
    let _ = writeln!(s, "- social graph canonical store: false");
    let _ = writeln!(s, "- content truth source: false");
    let _ = writeln!(s, "- network-wide authority: false\n");
    let _ = writeln!(
        s,
        "詳細は `server-manifest.json` の `authority_scope` / `p2p_boundary` を参照してください。\n"
    );

    s.push_str(&planned_section(config));
    s
}

fn gen_telecom_notification(config: &ResolvedConfig) -> String {
    let mut s = header(config, "電気通信事業 届出補助資料（役務説明ドラフト）");
    let _ = writeln!(s, "\n## 前提\n");
    let _ = writeln!(
        s,
        "この資料は、クラウド / VPS 利用・回線非設置を前提とした説明ドラフトです。\
         自宅サーバー構成や回線設置を伴う構成は advanced であり、個別確認が必要です。\n"
    );
    let _ = writeln!(s, "## 役務の概要\n");
    let _ = writeln!(
        s,
        "本サービスは、P2P network の補助層として動作する community node です。\
         ユーザーの identity / profile / social graph を所有せず、以下の補助機能を提供します。\n"
    );
    for cap in available_enabled(config) {
        let m = cap.meta();
        let _ = writeln!(s, "- {}: {}", m.display_name, m.telecom_note);
    }
    let _ = writeln!(s, "\n## relay に関する注意\n");
    if config.enabled(Capability::IrohRelay) || config.enabled(Capability::TrafficRelayFallback) {
        let _ = writeln!(
            s,
            "iroh relay / traffic relay fallback が有効なため、暗号化済み traffic の中継が発生し得ます。\
             これを signaling only と混同せず、役務区分・届出要否を総合通信局・専門家に事前確認してください。"
        );
    } else {
        let _ = writeln!(
            s,
            "relay 系 capability は無効です。実 traffic の中継は前提としていません。\
             ただし届出要否は最終的に運営者自身で確認してください。"
        );
    }
    let _ = writeln!(s, "\n## 構成情報\n");
    let _ = writeln!(
        s,
        "- クラウド / インフラ: {}",
        config
            .raw
            .server
            .cloud_provider
            .clone()
            .unwrap_or_else(|| "未指定".to_string())
    );
    let _ = writeln!(
        s,
        "- リージョン: {}",
        config
            .raw
            .server
            .region
            .clone()
            .unwrap_or_else(|| "未指定".to_string())
    );
    s.push_str(&planned_section(config));
    s
}

fn gen_service_description(config: &ResolvedConfig) -> String {
    let mut s = header(config, "サービス説明ドラフト");
    let _ = writeln!(s, "\n## 提供する補助機能（運用中）\n");
    for cap in available_enabled(config) {
        let m = cap.meta();
        let _ = writeln!(s, "### {}\n", m.display_name);
        let _ = writeln!(s, "- 目的: {}", m.purpose);
        let _ = writeln!(s, "- 取扱いデータ: {}", m.handled_data);
        let _ = writeln!(s, "- 保持への影響: {}\n", m.retention_impact);
    }
    s.push_str(&planned_section(config));
    s
}

fn gen_terms(config: &ResolvedConfig) -> String {
    let mut s = header(config, "利用規約（ドラフト）");
    let _ = writeln!(s, "\n## 第1条（本ノードの位置づけ）\n");
    let _ = writeln!(
        s,
        "本 community node は P2P network の補助層であり、ユーザーの identity / profile / social graph を\
         所有しません。本ノードの停止・変更によってもこれらは失われません。\n"
    );
    let _ = writeln!(s, "## 第2条（禁止事項）\n");
    let _ = writeln!(s, "- 法令に違反する目的での利用");
    let _ = writeln!(s, "- 他者の権利を侵害する行為");
    let _ = writeln!(s, "- 本ノードの補助機能の妨害\n");
    let _ = writeln!(s, "## 第3条（免責）\n");
    let _ = writeln!(
        s,
        "運営者は、本ノードが関与した補助機能の範囲でのみ責任を負い、kukuri network 全体・他ノードの\
         活動については責任を負いません。\n"
    );
    let _ = writeln!(s, "## 第4条（capability 別の取扱い）\n");
    for cap in available_enabled(config) {
        let m = cap.meta();
        let _ = writeln!(s, "- {}: {}", m.display_name, m.terms_note);
    }
    s.push_str(&planned_section(config));
    s
}

fn gen_privacy(config: &ResolvedConfig) -> String {
    let mut s = header(config, "プライバシーポリシー（ドラフト）");
    let _ = writeln!(s, "\n## 取得・取扱いするデータ（運用中の capability）\n");
    for cap in available_enabled(config) {
        let m = cap.meta();
        let _ = writeln!(s, "### {}\n", m.display_name);
        let _ = writeln!(s, "- 取扱いデータ: {}", m.handled_data);
        let _ = writeln!(s, "- 取扱いの説明: {}\n", m.privacy_note);
    }
    let _ = writeln!(s, "## 接続ログ・保持期間\n");
    let _ = writeln!(
        s,
        "- 接続ログ保持期間: {} 日",
        config.raw.retention.connection_logs_days
    );
    let _ = writeln!(
        s,
        "- モデレーションログ保持期間: {} 日\n",
        config.raw.retention.moderation_logs_days
    );
    let _ = writeln!(s, "## 外部送信\n");
    let _ = writeln!(
        s,
        "外部送信の詳細は `external-transmission-notice.md` を参照してください。\n"
    );
    s.push_str(&planned_section(config));
    s
}

fn gen_external_transmission(config: &ResolvedConfig) -> String {
    let mut s = header(config, "外部送信表示");
    let _ = writeln!(s, "\n## 現在の外部送信先（有効な機能に基づく）\n");
    let _ = writeln!(
        s,
        "以下は、現在有効な機能の構成に基づいて発生し得る外部送信先です。\n"
    );
    for dest in external_destinations(config) {
        let _ = writeln!(s, "### {}\n", dest.display_name());
        let _ = writeln!(s, "{}\n", dest.description());
    }

    // 無効化により送信されないものを明示（analytics: false 等の検証可能性）。
    let mut not_sent: Vec<ExternalDestination> = Vec::new();
    let active = external_destinations(config);
    for dest in [
        ExternalDestination::Cloudflare,
        ExternalDestination::ObjectStorage,
        ExternalDestination::PushProvider,
        ExternalDestination::AnalyticsProvider,
        ExternalDestination::CrashReportProvider,
        ExternalDestination::DedicatedIrohRelay,
        ExternalDestination::PublicRelay,
    ] {
        if !active.contains(&dest) {
            not_sent.push(dest);
        }
    }
    if !not_sent.is_empty() {
        let _ = writeln!(s, "## 送信していない外部送信先（無効な機能）\n");
        for dest in not_sent {
            let _ = writeln!(
                s,
                "- {}: 該当機能が無効のため送信しません。",
                dest.display_name()
            );
        }
        s.push('\n');
    }
    s.push_str(&planned_section(config));
    s
}

fn gen_abuse_policy(config: &ResolvedConfig) -> String {
    let mut s = header(config, "Abuse ポリシー（ドラフト）");
    let _ = writeln!(s, "\n## 連絡先\n");
    let _ = writeln!(s, "- abuse 連絡先: {}\n", config.contact());
    let _ = writeln!(s, "## 対応範囲\n");
    let _ = writeln!(
        s,
        "本ノードは、本ノードが実際に関与した補助機能（index / moderation / cache / relay 等のうち有効なもの）の\
         範囲でのみ abuse 対応を行います。kukuri network 全体の中央通報窓口ではありません。\n"
    );
    if config.enabled(Capability::ReportEndpoint) {
        let _ = writeln!(
            s,
            "（計画中）通報エンドポイントは未実装です。実装までは上記連絡先を窓口とします。\n"
        );
    }
    s.push_str(&planned_section(config));
    s
}

fn gen_moderation_policy(config: &ResolvedConfig) -> String {
    let mut s = header(config, "モデレーションポリシー（ドラフト）");
    let _ = writeln!(s, "\n## authority scope\n");
    let _ = writeln!(
        s,
        "本ノードの moderation / trust signal は、本ノードの authority scope 内でのみ意味を持ちます。\
         これらは network-wide command ではなく、他ノード・client が任意に採用し得る optional trust input です。\n"
    );
    if config.enabled(Capability::Moderation) || config.enabled(Capability::CommunityLocalTrust) {
        let _ = writeln!(
            s,
            "（計画中）moderation / trust signal は現行実装では未提供です。実装方針は #353 / #362 に従います。\n"
        );
    } else {
        let _ = writeln!(
            s,
            "本ノードでは moderation / trust signal を有効化していません。\n"
        );
    }
    let _ = writeln!(s, "## ログ保持\n");
    let _ = writeln!(
        s,
        "- モデレーションログ保持期間: {} 日\n",
        config.raw.retention.moderation_logs_days
    );
    s.push_str(&planned_section(config));
    s
}

fn gen_data_retention(config: &ResolvedConfig) -> String {
    let mut s = header(config, "データ保持ポリシー（ドラフト）");
    let _ = writeln!(s, "\n## 保持期間\n");
    let _ = writeln!(
        s,
        "- 接続ログ: {} 日",
        config.raw.retention.connection_logs_days
    );
    let _ = writeln!(
        s,
        "- モデレーションログ: {} 日\n",
        config.raw.retention.moderation_logs_days
    );
    let _ = writeln!(s, "## capability 別の保持への影響（運用中）\n");
    for cap in available_enabled(config) {
        let m = cap.meta();
        let _ = writeln!(s, "- {}: {}", m.display_name, m.retention_impact);
    }
    s.push_str(&planned_section(config));
    s
}

fn gen_prior_consultation_email(config: &ResolvedConfig) -> String {
    let s_cfg = &config.raw.server;
    let mut s = header(config, "事前相談メールテンプレート");
    let _ = writeln!(s, "\n## 件名\n");
    let _ = writeln!(
        s,
        "電気通信事業の届出要否に関する事前相談（{}）\n",
        s_cfg.domain
    );
    let _ = writeln!(s, "## 本文（ドラフト）\n");
    let _ = writeln!(s, "```text");
    let _ = writeln!(s, "ご担当者様");
    s.push('\n');
    let _ = writeln!(
        s,
        "{operator} と申します。P2P network の補助層として動作する community node の",
        operator = s_cfg.operator_name
    );
    let _ = writeln!(
        s,
        "運営に関し、電気通信事業の届出要否について事前相談させていただきたくご連絡しました。"
    );
    s.push('\n');
    let _ = writeln!(s, "■ サービス概要");
    let _ = writeln!(s, "- ドメイン: {}", s_cfg.domain);
    let _ = writeln!(
        s,
        "- インフラ: {} / 回線非設置（クラウド / VPS 利用）",
        s_cfg
            .cloud_provider
            .clone()
            .unwrap_or_else(|| "クラウド".to_string())
    );
    let _ = writeln!(
        s,
        "- 役割: ユーザーの identity / profile / social graph を所有しない補助ノード"
    );
    s.push('\n');
    let _ = writeln!(s, "■ 有効な補助機能");
    for cap in available_enabled(config) {
        let _ = writeln!(s, "- {}", cap.meta().display_name);
    }
    if config.enabled(Capability::IrohRelay) || config.enabled(Capability::TrafficRelayFallback) {
        s.push('\n');
        let _ = writeln!(s, "■ relay について");
        let _ = writeln!(
            s,
            "暗号化済み traffic の relay 中継が発生し得ます（signaling only ではありません）。"
        );
    }
    s.push('\n');
    let _ = writeln!(
        s,
        "上記構成における届出要否についてご教示いただけますと幸いです。"
    );
    let _ = writeln!(s, "```\n");
    s
}

// ---------------------------------------------------------------------------
// 集約
// ---------------------------------------------------------------------------

/// すべての生成文書を filename 昇順で返す。
pub fn generate_all(config: &ResolvedConfig) -> Vec<GeneratedFile> {
    let mut files = vec![
        GeneratedFile {
            filename: MANIFEST_FILE.to_string(),
            content: render_manifest(config),
        },
        GeneratedFile {
            filename: "network-diagram.md".to_string(),
            content: gen_network_diagram(config),
        },
        GeneratedFile {
            filename: "telecom-notification-draft.md".to_string(),
            content: gen_telecom_notification(config),
        },
        GeneratedFile {
            filename: "service-description-draft.md".to_string(),
            content: gen_service_description(config),
        },
        GeneratedFile {
            filename: "terms.md".to_string(),
            content: gen_terms(config),
        },
        GeneratedFile {
            filename: "privacy-policy.md".to_string(),
            content: gen_privacy(config),
        },
        GeneratedFile {
            filename: "external-transmission-notice.md".to_string(),
            content: gen_external_transmission(config),
        },
        GeneratedFile {
            filename: "abuse-policy.md".to_string(),
            content: gen_abuse_policy(config),
        },
        GeneratedFile {
            filename: "moderation-policy.md".to_string(),
            content: gen_moderation_policy(config),
        },
        GeneratedFile {
            filename: "data-retention-policy.md".to_string(),
            content: gen_data_retention(config),
        },
        GeneratedFile {
            filename: "prior-consultation-email.md".to_string(),
            content: gen_prior_consultation_email(config),
        },
    ];
    files.sort_by(|a, b| a.filename.cmp(&b.filename));
    files
}
