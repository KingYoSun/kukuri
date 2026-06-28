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
    /// terraform デプロイ用の env 設定（#380）。
    ///
    /// 指定すると `cn-operator generate-tfvars` が operator-config を単一の入力元として
    /// terraform.tfvars を生成できる。未指定なら従来通り docs / manifest のみを生成する
    /// （後方互換）。コスト/データ階層の軸（low-cost / managed-db / ha）であり、capability 軸
    /// （`profile` / `features`）とは独立。
    #[serde(default)]
    pub deploy: Option<DeployConfig>,
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

/// terraform deployment profile（コスト/データ階層の軸）。
///
/// cn-operator の capability profile（`Profile`: minimal / relay-enabled / full-service）とは
/// **別物**。こちらはインフラのコスト/データ階層を選ぶ軸。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeployProfile {
    /// 単一 VM 上で node-local Postgres / Valkey を動かす個人・小規模 operator の入口。
    #[default]
    LowCost,
    /// Cloud SQL + Memorystore（拡張点。tfvars 生成は未対応）。
    ManagedDb,
    /// HA DB/cache + object storage（拡張点。tfvars 生成は未対応）。
    Ha,
}

impl DeployProfile {
    pub fn key(self) -> &'static str {
        match self {
            DeployProfile::LowCost => "low-cost",
            DeployProfile::ManagedDb => "managed-db",
            DeployProfile::Ha => "ha",
        }
    }
}

fn default_deploy_profile() -> DeployProfile {
    DeployProfile::LowCost
}

fn default_region() -> String {
    "asia-northeast1".to_string()
}

fn default_zone() -> String {
    "asia-northeast1-a".to_string()
}

fn default_cn_user_api_image() -> String {
    "ghcr.io/kingyosun/kukuri-cn-user-api:latest".to_string()
}

fn default_cn_iroh_relay_image() -> String {
    "ghcr.io/kingyosun/kukuri-cn-iroh-relay:latest".to_string()
}

fn default_cn_cli_image() -> String {
    "ghcr.io/kingyosun/kukuri-cn-cli:latest".to_string()
}

fn default_machine_type() -> String {
    "e2-small".to_string()
}

fn default_disk_size_gb() -> u32 {
    30
}

fn default_blob_cache_ttl_hours() -> u32 {
    24
}

fn default_blob_cache_path() -> String {
    "/var/lib/kukuri/blob-cache".to_string()
}

fn default_backup_enabled() -> bool {
    true
}

fn default_backup_retention_days() -> u32 {
    30
}

fn default_rate_limit_enabled() -> bool {
    true
}

fn default_rate_limit_per_second() -> u32 {
    10
}

fn default_rate_limit_burst() -> u32 {
    30
}

/// terraform デプロイ用の env 設定（#380）。
///
/// secret は **値ではなく Secret Manager の secret ID** のみを持つ（payload は terraform に
/// 渡さない）。blob cache の on/off は `features.blob_cache` を真実源とし、ここでは sizing
/// （size / ttl / path）のみを持つ。
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeployConfig {
    /// deployment profile（既定 low-cost）。
    #[serde(default = "default_deploy_profile")]
    pub profile: DeployProfile,
    /// GCP project ID（必須）。
    pub project_id: String,
    #[serde(default = "default_region")]
    pub region: String,
    #[serde(default = "default_zone")]
    pub zone: String,
    /// cn-iroh-relay の公開 hostname。low-cost profile では常に必須。
    /// それ以外でも iroh_relay 有効時は必須。
    /// api hostname は `server.domain` から導出する。
    #[serde(default)]
    pub relay_domain: Option<String>,
    /// ACME(Let's Encrypt) 登録 email（必須）。
    pub acme_email: String,
    /// true なら Cloud DNS の既存 zone に A レコードを作成する。
    #[serde(default)]
    pub manage_cloud_dns: bool,
    /// Cloud DNS managed zone 名（manage_cloud_dns=true のとき必須）。
    #[serde(default)]
    pub dns_zone_name: Option<String>,
    #[serde(default = "default_cn_user_api_image")]
    pub cn_user_api_image: String,
    #[serde(default = "default_cn_iroh_relay_image")]
    pub cn_iroh_relay_image: String,
    #[serde(default = "default_cn_cli_image")]
    pub cn_cli_image: String,
    /// COMMUNITY_NODE_JWT_SECRET を保持する Secret Manager secret ID（必須・値ではない）。
    pub jwt_secret_id: String,
    /// Postgres password を保持する Secret Manager secret ID（必須・値ではない）。
    pub postgres_password_secret_id: String,
    #[serde(default = "default_machine_type")]
    pub machine_type: String,
    #[serde(default = "default_disk_size_gb")]
    pub disk_size_gb: u32,
    /// Postgres data 用の専用 persistent disk サイズ（GB）。0 なら boot disk 上の docker volume。
    #[serde(default)]
    pub postgres_data_disk_gb: u32,
    /// blob cache 専用ディスクサイズ（GB）。0 なら専用ディスクなし。
    /// `features.blob_cache=false` のときに > 0 を指定すると検証で失敗する。
    #[serde(default)]
    pub blob_cache_size_gb: u32,
    #[serde(default = "default_blob_cache_ttl_hours")]
    pub blob_cache_ttl_hours: u32,
    #[serde(default = "default_blob_cache_path")]
    pub blob_cache_path: String,
    #[serde(default = "default_backup_enabled")]
    pub backup_enabled: bool,
    #[serde(default = "default_backup_retention_days")]
    pub backup_retention_days: u32,
    #[serde(default = "default_rate_limit_enabled")]
    pub rate_limit_enabled: bool,
    #[serde(default = "default_rate_limit_per_second")]
    pub rate_limit_per_second: u32,
    #[serde(default = "default_rate_limit_burst")]
    pub rate_limit_burst: u32,
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

    /// terraform デプロイ設定（#380）。未指定なら None。
    pub fn deploy(&self) -> Option<&DeployConfig> {
        self.raw.deploy.as_ref()
    }

    /// cn-user-api の公開 hostname。`server.domain` をそのまま使う。
    pub fn api_domain(&self) -> &str {
        self.raw.server.domain.as_str()
    }

    /// blob cache の単一真実源（#380）。`features.blob_cache` を根拠にする。
    pub fn blob_cache_enabled(&self) -> bool {
        self.enabled(Capability::BlobCache)
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

    // deploy セクションの検証（指定されている場合のみ。未指定は従来通り通す）。
    if let Some(deploy) = resolved.raw.deploy.as_ref() {
        validate_deploy(&resolved, deploy)?;
    }

    Ok(resolved)
}

/// deploy セクションを検証する（#380）。
fn validate_deploy(resolved: &ResolvedConfig, deploy: &DeployConfig) -> Result<()> {
    let project_id = require_deploy_string("deploy.project_id", &deploy.project_id)?;
    let acme_email = require_deploy_string("deploy.acme_email", &deploy.acme_email)?;
    let jwt_secret_id = require_deploy_string("deploy.jwt_secret_id", &deploy.jwt_secret_id)?;
    let postgres_password_secret_id = require_deploy_string(
        "deploy.postgres_password_secret_id",
        &deploy.postgres_password_secret_id,
    )?;

    validate_deploy_string("deploy.region", &deploy.region)?;
    validate_deploy_string("deploy.zone", &deploy.zone)?;
    validate_deploy_string("deploy.cn_user_api_image", &deploy.cn_user_api_image)?;
    validate_deploy_string("deploy.cn_iroh_relay_image", &deploy.cn_iroh_relay_image)?;
    validate_deploy_string("deploy.cn_cli_image", &deploy.cn_cli_image)?;
    validate_deploy_string("deploy.machine_type", &deploy.machine_type)?;
    validate_deploy_string("deploy.blob_cache_path", &deploy.blob_cache_path)?;

    // low-cost template は cn-iroh-relay を常に配置し、relay_domain を Caddy / compose / certbot で使う。
    let relay_domain = if deploy.profile == DeployProfile::LowCost {
        let relay_domain = deploy
            .relay_domain
            .as_ref()
            .map(|d| require_deploy_string("deploy.relay_domain", d))
            .transpose()?
            .ok_or_else(|| {
                anyhow!("deploy.profile=low-cost の場合、deploy.relay_domain は必須です")
            })?;
        Some(relay_domain)
    } else if resolved.enabled(Capability::IrohRelay) {
        // managed-db / ha は tfvars 未対応だが、relay capability を開示するなら hostname は必要。
        let relay_domain = deploy
            .relay_domain
            .as_ref()
            .map(|d| require_deploy_string("deploy.relay_domain", d))
            .transpose()?
            .ok_or_else(|| {
                anyhow!("iroh_relay capability が有効な場合、deploy.relay_domain は必須です")
            })?;
        Some(relay_domain)
    } else {
        deploy
            .relay_domain
            .as_ref()
            .map(|d| require_deploy_string("deploy.relay_domain", d))
            .transpose()?
    };

    // Cloud DNS を管理するなら zone 名が必須。
    if deploy.manage_cloud_dns
        && deploy
            .dns_zone_name
            .as_ref()
            .map(|z| z.trim().is_empty())
            .unwrap_or(true)
    {
        bail!("deploy.manage_cloud_dns=true の場合、deploy.dns_zone_name は必須です");
    }

    if let Some(dns_zone_name) = deploy.dns_zone_name.as_ref() {
        validate_deploy_string("deploy.dns_zone_name", dns_zone_name)?;
    }

    // blob cache の真実源は features.blob_cache。無効なのに sizing を指定するのは矛盾。
    if !resolved.blob_cache_enabled() && deploy.blob_cache_size_gb > 0 {
        bail!(
            "features.blob_cache=false ですが deploy.blob_cache_size_gb > 0 が指定されています。\n\
             blob cache の on/off は features.blob_cache を真実源とします。\n\
             有効化する場合は features.blob_cache: true を設定してください。"
        );
    }

    // profile（low-cost / managed-db / ha）の tfvars 生成対応可否は generate-tfvars 側で判定する。
    // ここで弾くと managed-db / ha を deploy に書いた config が docs / manifest 生成すらできなくなる。

    if deploy.profile == DeployProfile::LowCost {
        validate_gcp_project_id("deploy.project_id", project_id)?;
        validate_gcp_location("deploy.region", deploy.region.trim())?;
        validate_gcp_location("deploy.zone", deploy.zone.trim())?;
        validate_dns_hostname("server.domain", resolved.api_domain().trim())?;
        if let Some(relay_domain) = relay_domain {
            validate_dns_hostname("deploy.relay_domain", relay_domain)?;
        }
        validate_acme_email(acme_email)?;
        validate_secret_id("deploy.jwt_secret_id", jwt_secret_id)?;
        validate_secret_id(
            "deploy.postgres_password_secret_id",
            postgres_password_secret_id,
        )?;
        validate_container_image("deploy.cn_user_api_image", deploy.cn_user_api_image.trim())?;
        validate_container_image(
            "deploy.cn_iroh_relay_image",
            deploy.cn_iroh_relay_image.trim(),
        )?;
        validate_container_image("deploy.cn_cli_image", deploy.cn_cli_image.trim())?;
        validate_machine_type("deploy.machine_type", deploy.machine_type.trim())?;
        validate_absolute_path("deploy.blob_cache_path", deploy.blob_cache_path.trim())?;
        if let Some(dns_zone_name) = deploy
            .dns_zone_name
            .as_ref()
            .map(|z| z.trim())
            .filter(|z| !z.is_empty())
        {
            validate_gcp_name("deploy.dns_zone_name", dns_zone_name)?;
        }
    }

    Ok(())
}

fn require_deploy_string<'a>(field: &str, value: &'a str) -> Result<&'a str> {
    validate_deploy_string(field, value)?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        bail!("{field} は必須です");
    }
    Ok(trimmed)
}

fn validate_deploy_string(field: &str, value: &str) -> Result<()> {
    if value.chars().any(char::is_control) {
        bail!("{field} に制御文字は指定できません");
    }
    Ok(())
}

fn validate_gcp_project_id(field: &str, value: &str) -> Result<()> {
    let bytes = value.as_bytes();
    let valid_len = (6..=30).contains(&bytes.len());
    let valid_start = bytes.first().is_some_and(u8::is_ascii_lowercase);
    let valid_end = bytes
        .last()
        .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit());
    let valid_chars = bytes
        .iter()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-');

    if !(valid_len && valid_start && valid_end && valid_chars) {
        bail!(
            "{field} は GCP project ID 形式（小文字英数字と hyphen、6-30 文字）で指定してください"
        );
    }
    Ok(())
}

fn validate_gcp_location(field: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        bail!("{field} は小文字英数字と hyphen のみで指定してください");
    }
    Ok(())
}

fn validate_dns_hostname(field: &str, value: &str) -> Result<()> {
    if value.len() > 253 || value.trim_end_matches('.').is_empty() {
        bail!("{field} は DNS hostname 形式で指定してください");
    }

    for label in value.trim_end_matches('.').split('.') {
        let bytes = label.as_bytes();
        let valid_len = !bytes.is_empty() && bytes.len() <= 63;
        let valid_edges = bytes
            .first()
            .zip(bytes.last())
            .is_some_and(|(first, last)| {
                first.is_ascii_alphanumeric() && last.is_ascii_alphanumeric()
            });
        let valid_chars = bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'-');
        if !(valid_len && valid_edges && valid_chars) {
            bail!("{field} は DNS hostname 形式で指定してください");
        }
    }

    Ok(())
}

fn validate_acme_email(value: &str) -> Result<()> {
    if value.contains(char::is_whitespace) {
        bail!("deploy.acme_email に空白は指定できません");
    }
    let Some((local, domain)) = value.split_once('@') else {
        bail!("deploy.acme_email は email 形式で指定してください");
    };
    if local.is_empty() || domain.contains('@') {
        bail!("deploy.acme_email は email 形式で指定してください");
    }
    validate_dns_hostname("deploy.acme_email の domain", domain)
}

fn validate_secret_id(field: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 255
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        bail!(
            "{field} は Secret Manager secret ID 形式（英数字、hyphen、underscore）で指定してください"
        );
    }
    Ok(())
}

fn validate_container_image(field: &str, value: &str) -> Result<()> {
    if value.is_empty() || value.contains(char::is_whitespace) {
        bail!("{field} は空白を含まない container image 参照で指定してください");
    }
    Ok(())
}

fn validate_machine_type(field: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        bail!("{field} は小文字英数字と hyphen のみで指定してください");
    }
    Ok(())
}

fn validate_absolute_path(field: &str, value: &str) -> Result<()> {
    if !value.starts_with('/') || value.contains(char::is_whitespace) {
        bail!("{field} は空白を含まない absolute path で指定してください");
    }
    Ok(())
}

fn validate_gcp_name(field: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        bail!("{field} は小文字英数字と hyphen のみで指定してください");
    }
    Ok(())
}

/// パースと解決・検証をまとめて行う。
pub fn load_and_validate(yaml: &str) -> Result<ResolvedConfig> {
    resolve_and_validate(parse_config(yaml)?)
}
