//! operator config (`operator-config.yaml`) の schema と検証。
//!
//! このファイルは CLI と server manifest / 生成文書の単一の入力元である。

use std::collections::BTreeMap;

use anyhow::{Result, anyhow, bail};
use serde::{Deserialize, Serialize};

use crate::capability::{Availability, Capability};
use crate::manifest::{AuthorityScopeOverride, NodeRole};
use crate::profile::Profile;

/// `operator-config.yaml` の生表現。
///
/// `features` は未指定キーを許容し、profile の既定値で補完する。
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorConfig {
    pub server: ServerConfig,
    #[serde(default)]
    pub profile: Option<Profile>,
    #[serde(default)]
    pub features: BTreeMap<String, bool>,
    #[serde(default)]
    pub retention: RetentionConfig,
    #[serde(default)]
    pub manifest: ManifestConfig,
    /// Phase B（未実装 / 計画中）capability を有効化することを明示的に承認する。
    ///
    /// これが false のまま Planned capability を有効化すると検証で失敗する。
    /// 実体のない「運用中」開示を生成しないためのガード。
    #[serde(default)]
    pub acknowledge_planned_capabilities: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    pub domain: String,
    pub operator_name: String,
    /// ISO 3166-1 alpha-2（例: JP）。
    pub country: String,
    #[serde(default)]
    pub cloud_provider: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
    /// abuse / 問い合わせ連絡先。未指定なら domain から導出する。
    #[serde(default)]
    pub contact: Option<String>,
    #[serde(default)]
    pub node_id: Option<String>,
    #[serde(default)]
    pub node_name: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RetentionConfig {
    #[serde(default = "default_connection_logs_days")]
    pub connection_logs_days: u32,
    #[serde(default = "default_moderation_logs_days")]
    pub moderation_logs_days: u32,
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            connection_logs_days: default_connection_logs_days(),
            moderation_logs_days: default_moderation_logs_days(),
        }
    }
}

fn default_connection_logs_days() -> u32 {
    30
}

fn default_moderation_logs_days() -> u32 {
    180
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestConfig {
    /// node role。未指定なら profile / 有効 capability から推定する。
    #[serde(default)]
    pub node_role: Option<NodeRole>,
    /// manifest / policy version。決定論的出力のため config 由来とする。
    #[serde(default = "default_manifest_version")]
    pub manifest_version: String,
    /// authority scope を operator が明示的に拡張・上書きするための設定。
    /// 未指定なら applies_to は有効 capability から導出し、does_not_apply_to は安全な default を使う。
    #[serde(default)]
    pub authority_scope: AuthorityScopeOverride,
}

fn default_manifest_version() -> String {
    "v1".to_string()
}

/// profile / features を解決し検証済みの設定。
#[derive(Clone, Debug)]
pub struct ResolvedConfig {
    pub raw: OperatorConfig,
    /// capability ごとの有効・無効（全 capability を網羅）。
    enabled: BTreeMap<Capability, bool>,
}

impl ResolvedConfig {
    pub fn enabled(&self, capability: Capability) -> bool {
        self.enabled.get(&capability).copied().unwrap_or(false)
    }

    /// `Capability::ALL` の順序で有効な capability を返す。
    pub fn enabled_capabilities(&self) -> Vec<Capability> {
        Capability::ALL
            .iter()
            .copied()
            .filter(|cap| self.enabled(*cap))
            .collect()
    }

    /// `Capability::ALL` の順序で無効な capability を返す。
    pub fn disabled_capabilities(&self) -> Vec<Capability> {
        Capability::ALL
            .iter()
            .copied()
            .filter(|cap| !self.enabled(*cap))
            .collect()
    }

    /// 有効かつ Phase B（計画中）の capability。
    pub fn enabled_planned_capabilities(&self) -> Vec<Capability> {
        self.enabled_capabilities()
            .into_iter()
            .filter(|cap| cap.availability().is_planned())
            .collect()
    }

    pub fn contact(&self) -> String {
        self.raw
            .server
            .contact
            .clone()
            .filter(|c| !c.trim().is_empty())
            .unwrap_or_else(|| format!("abuse@{}", self.raw.server.domain))
    }

    pub fn policy_url(&self, path: &str) -> String {
        format!("https://{}/{}", self.raw.server.domain, path)
    }

    /// 通報受付 endpoint（#370）。report_endpoint capability が有効なときのみ絶対 URL を返す。
    /// 無効なら空文字を返し、client（#310）は abuse_contact 案内に切り替える。
    pub fn report_endpoint(&self) -> String {
        if self.enabled(Capability::ReportEndpoint) {
            self.policy_url("v1/report")
        } else {
            String::new()
        }
    }
}

/// YAML 文字列をパースする。
pub fn parse_config(yaml: &str) -> Result<OperatorConfig> {
    let config: OperatorConfig = serde_yaml::from_str(yaml)
        .map_err(|e| anyhow!("operator-config.yaml のパースに失敗しました: {e}"))?;
    Ok(config)
}

/// profile と features を解決し、必須項目・Phase B 承認を検証する。
pub fn resolve_and_validate(config: OperatorConfig) -> Result<ResolvedConfig> {
    // 必須フィールド。
    if config.server.domain.trim().is_empty() {
        bail!("server.domain は必須です");
    }
    if config.server.operator_name.trim().is_empty() {
        bail!("server.operator_name は必須です");
    }
    if config.server.country.trim().len() != 2 {
        bail!("server.country は ISO 3166-1 alpha-2（2文字、例: JP）で指定してください");
    }

    // 未知の feature キーを拒否する（typo によるサイレントな無効化を防ぐ）。
    let known: BTreeMap<&str, Capability> = Capability::ALL.iter().map(|c| (c.key(), *c)).collect();
    for key in config.features.keys() {
        if !known.contains_key(key.as_str()) {
            bail!(
                "features に未知のキー `{key}` があります。指定可能: {}",
                Capability::ALL
                    .iter()
                    .map(|c| c.key())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }

    // profile 既定値 -> features 上書き の順で解決する。
    let profile_defaults = config
        .profile
        .map(|p| p.feature_defaults())
        .unwrap_or_default();

    let mut enabled: BTreeMap<Capability, bool> = BTreeMap::new();
    for cap in Capability::ALL {
        // auth_consent は baseline として常に有効。
        let baseline = matches!(cap, Capability::AuthConsent);
        let from_profile = profile_defaults.get(&cap).copied().unwrap_or(baseline);
        let value = config
            .features
            .get(cap.key())
            .copied()
            .unwrap_or(from_profile);
        enabled.insert(cap, value || baseline);
    }

    let resolved = ResolvedConfig {
        raw: config,
        enabled,
    };

    // Phase B capability の承認ガード。
    let planned = resolved.enabled_planned_capabilities();
    if !planned.is_empty() && !resolved.raw.acknowledge_planned_capabilities {
        let names = planned
            .iter()
            .map(|c| c.key())
            .collect::<Vec<_>>()
            .join(", ");
        bail!(
            "計画中（未実装）の capability が有効化されています: {names}\n\
             これらは現行の community node 実装では提供されません。\n\
             運用中であるかのような開示文書の生成を防ぐため、\n\
             config に `acknowledge_planned_capabilities: true` を設定して\n\
             「spec として記述する」ことを明示的に承認してください。\n\
             承認した場合でも、生成文書ではこれらは「{}」として扱われます。",
            Availability::Planned.label_ja()
        );
    }

    Ok(resolved)
}

/// パースと解決・検証をまとめて行う。
pub fn load_and_validate(yaml: &str) -> Result<ResolvedConfig> {
    resolve_and_validate(parse_config(yaml)?)
}
