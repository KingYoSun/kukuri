//! operator config から `server-manifest.json` を決定論的に生成する。
//!
//! manifest は #355 の authority scope / P2P boundary / capability scope を先取りした
//! schema を持ち、client の dependency 表示 / report routing / consent UI から利用できる。
//!
//! Phase A / Phase B の分離はここでも保たれる。capability scope は
//! `available_enabled`（運用中）と `planned_enabled`（計画中・未提供）を分けて宣言する。

use serde_json::{Value, json};

use crate::capability::{Availability, Capability};
use crate::config::ResolvedConfig;

/// node role を推定する（config 指定があればそれを優先）。
fn node_role(config: &ResolvedConfig) -> String {
    if let Some(role) = config
        .raw
        .manifest
        .node_role
        .clone()
        .filter(|r| !r.trim().is_empty())
    {
        return role;
    }
    "community-node".to_string()
}

fn capability_keys(caps: &[Capability]) -> Vec<String> {
    caps.iter().map(|c| c.key().to_string()).collect()
}

/// `server-manifest.json` の値を構築する。
///
/// serde_json の Map は既定でキー順ソートされるため、出力は決定論的。
pub fn build_manifest(config: &ResolvedConfig) -> Value {
    let server = &config.raw.server;

    let available_enabled: Vec<Capability> = config
        .enabled_capabilities()
        .into_iter()
        .filter(|c| c.availability() == Availability::Available)
        .collect();
    let planned_enabled = config.enabled_planned_capabilities();

    // capabilities マップ: 全 capability の有効・無効。
    let mut capabilities = serde_json::Map::new();
    for cap in Capability::ALL {
        capabilities.insert(cap.key().to_string(), Value::Bool(config.enabled(cap)));
    }

    // authority scope: applies_to は実際に有効な capability から導出する。
    let mut applies_to = vec!["this_node".to_string()];
    if config.enabled(Capability::CommunityIndex) {
        applies_to.push("communities_indexed_by_this_node".to_string());
    }
    if config.enabled(Capability::Moderation) {
        applies_to.push("moderation_events_issued_by_this_node".to_string());
    }
    if config.enabled(Capability::CommunityLocalTrust) {
        applies_to.push("trust_signals_issued_by_this_node".to_string());
    }
    if config.enabled(Capability::BlobCache) {
        applies_to.push("media_cached_by_this_node".to_string());
    }

    let iroh_relay_mode = if config.enabled(Capability::IrohRelay) {
        "dedicated"
    } else {
        "none"
    };

    json!({
        "node_id": server.node_id.clone().unwrap_or_default(),
        "node_name": server.node_name.clone().unwrap_or_else(|| server.domain.clone()),
        "node_role": node_role(config),
        "server_name": server.domain,
        "operator_name": server.operator_name,
        "operator_country": server.country,
        "cloud_provider": server.cloud_provider.clone().unwrap_or_default(),
        "region": server.region.clone().unwrap_or_default(),
        "contact": config.contact(),
        "abuse_contact": config.contact(),
        "terms_url": config.policy_url("terms"),
        "privacy_url": config.policy_url("privacy"),
        "external_transmission_url": config.policy_url("external-transmission"),
        "moderation_policy_url": config.policy_url("moderation-policy"),
        "abuse_policy_url": config.policy_url("abuse-policy"),
        "manifest_version": config.raw.manifest.manifest_version,
        "capabilities": Value::Object(capabilities),
        "capability_scope": {
            "available_enabled": capability_keys(&available_enabled),
            "planned_enabled": capability_keys(&planned_enabled),
        },
        "authority_scope": {
            "applies_to": applies_to,
            "does_not_apply_to": [
                "kukuri_network_as_a_whole",
                "third_party_nodes",
                "user_identity",
                "user_profile_canonical_source",
                "user_social_graph_canonical_source",
            ],
        },
        "p2p_boundary": {
            "identity_authority": false,
            "profile_canonical_store": false,
            "social_graph_canonical_store": false,
            "content_truth_source": false,
            "network_wide_authority": false,
        },
        "features": {
            "community_index": config.enabled(Capability::CommunityIndex),
            "moderation": config.enabled(Capability::Moderation),
            "trust_score": if config.enabled(Capability::CommunityLocalTrust) {
                "community-local"
            } else {
                "none"
            },
            "iroh_relay": config.enabled(Capability::IrohRelay),
            "iroh_relay_mode": iroh_relay_mode,
            "traffic_relay_fallback": config.enabled(Capability::TrafficRelayFallback),
            "private_message_storage": config.enabled(Capability::PrivateMessageStorage),
            "blob_cache": config.enabled(Capability::BlobCache),
        },
        "retention": {
            "connection_logs_days": config.raw.retention.connection_logs_days,
            "moderation_logs_days": config.raw.retention.moderation_logs_days,
        },
    })
}

/// manifest を改行終端の pretty JSON 文字列にする。
pub fn render_manifest(config: &ResolvedConfig) -> String {
    let value = build_manifest(config);
    let mut out = serde_json::to_string_pretty(&value).expect("manifest serialization");
    out.push('\n');
    out
}
