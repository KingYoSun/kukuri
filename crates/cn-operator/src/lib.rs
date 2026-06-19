//! kukuri community node operator docs generator (#352)。
//!
//! operator config (`operator-config.yaml`) を単一の入力元として、運営者向けの
//! 利用規約・プライバシーポリシー・外部送信表示・電気通信届出補助資料・server manifest を
//! 決定論的に生成する。
//!
//! Phase A / Phase B の分離:
//! - Phase A (`Availability::Available`): 現行 community node 実装 / デプロイ構成として
//!   提供できる capability。生成文書で「運用中」として開示してよい。
//! - Phase B (`Availability::Planned`): 未実装の capability（index / moderation / trust /
//!   report endpoint）。config で宣言できるが、生成文書では「計画中・未提供」として扱い、
//!   運用中の外部送信・データ取扱い開示には含めない。`acknowledge_planned_capabilities`
//!   による明示承認がなければ検証で失敗する。

pub mod capability;
pub mod capability_risk;
pub mod config;
pub mod docs;
pub mod drift;
pub mod manifest;
pub mod profile;

pub use capability::{Availability, Capability, CapabilityMeta, ExternalDestination};
pub use capability_risk::CapabilityRiskPractices;
pub use config::{
    OperatorConfig, ResolvedConfig, RetentionConfig, ServerConfig, load_and_validate, parse_config,
    resolve_and_validate,
};
pub use docs::{GeneratedFile, generate_all};
pub use drift::{DriftReport, check_drift};
pub use manifest::{
    AuthorityScope, AuthorityScopeOverride, Capabilities, CapabilityScope, CommunityNodeManifest,
    ManifestFeatures, ManifestRetention, NodeRole, P2pBoundary, build_manifest, manifest_value,
    render_manifest,
};
pub use profile::Profile;

/// `operator init` が出力するサンプル config。
pub const SAMPLE_CONFIG: &str = r#"server:
  domain: example-kukuri.net
  operator_name: Example Operator
  country: JP
  cloud_provider: AWS
  region: ap-northeast-1
  contact: abuse@example-kukuri.net

# profile が features の既定値を与える。個別の features キーで上書きできる。
profile: relay-enabled

features:
  community_index: true
  moderation: true
  community_local_trust: true
  iroh_relay: true
  traffic_relay_fallback: true
  private_message_storage: false
  blob_cache: false
  analytics: false
  crash_report: false
  cloudflare_proxy: true

retention:
  connection_logs_days: 30
  moderation_logs_days: 180

manifest:
  manifest_version: v1
  # node_role 未指定なら有効 capability から推定する（既定: community-node）。
  # default onboarding node の場合は明示する:
  #   node_role: default-onboarding-node
  # authority_scope:
  #   additional_applies_to: []        # 導出された applies_to に追加する項目
  #   does_not_apply_to: null          # 未指定なら安全な default を使う

# community_index / moderation / community_local_trust / report_endpoint は
# 現行実装では未提供（計画中）。spec として記述することを明示的に承認する。
acknowledge_planned_capabilities: true
"#;
