use super::*;

// slim manifest 型(CommunityNodeManifest ほか)は kukuri-cn-protocol へ移動(WP-H3)。
// 取得ロジックと round-trip 契約テスト(サーバ実出力 → slim 型)はここに残す。
pub use kukuri_cn_protocol::{
    CommunityNodeAuthorityScope, CommunityNodeCapabilityScope, CommunityNodeManifest,
    CommunityNodeP2pBoundary,
};

/// manifest fetch の結果状態。error は command の Err として返すため含めない。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommunityNodeManifestFetchStatus {
    /// manifest を取得・解析できた。
    Ok,
    /// node が manifest を公開していない（404）。client は default node へ fallback しない。
    Absent,
}

/// manifest fetch の結果。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunityNodeManifestFetch {
    pub status: CommunityNodeManifestFetchStatus,
    #[serde(default)]
    pub manifest: Option<CommunityNodeManifest>,
}

impl DesktopRuntime {
    /// public manifest endpoint (#356) から unauthenticated に manifest を取得する。
    ///
    /// - 200: 解析して `Ok` 状態で返す
    /// - 404: node が manifest 未公開。`Absent` 状態で返す（default node へ fallback しない）
    /// - その他/通信失敗: `Err`（呼び出し側は error として表示する）
    pub(crate) async fn request_community_node_manifest(
        &self,
        base_url: &str,
    ) -> Result<CommunityNodeManifestFetch> {
        let base_url = normalize_http_url(base_url)?;
        let client = community_node_http_client()?;
        let url = format!("{base_url}{NODE_MANIFEST_PATH}");
        let response = client
            .get(url)
            .send()
            .await
            .context("failed to fetch community node manifest")?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(CommunityNodeManifestFetch {
                status: CommunityNodeManifestFetchStatus::Absent,
                manifest: None,
            });
        }
        let response = response
            .error_for_status()
            .context("community node manifest request failed")?;
        let manifest = response
            .json::<CommunityNodeManifest>()
            .await
            .context("failed to decode community node manifest")?;
        Ok(CommunityNodeManifestFetch {
            status: CommunityNodeManifestFetchStatus::Ok,
            manifest: Some(manifest),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #356 の server-manifest.json と同形の JSON が client slim 型へ解析できることを確認する。
    const SAMPLE_MANIFEST: &str = r#"{
        "node_id": "",
        "node_name": "example-kukuri.net",
        "node_role": "default-onboarding-node",
        "server_name": "example-kukuri.net",
        "manifest_version": "v1",
        "extra_unknown_field": "ignored",
        "capability_scope": {
            "available_enabled": ["auth_consent", "iroh_relay"],
            "planned_enabled": ["moderation"]
        },
        "authority_scope": {
            "applies_to": ["this_node"],
            "does_not_apply_to": ["user_identity", "kukuri_network_as_a_whole"]
        },
        "p2p_boundary": {
            "identity_authority": false,
            "profile_canonical_store": false,
            "social_graph_canonical_store": false,
            "content_truth_source": false,
            "network_wide_authority": false
        },
        "abuse_contact": "abuse@example-kukuri.net",
        "report_endpoint": "https://example-kukuri.net/v1/report",
        "terms_url": "https://example-kukuri.net/terms"
    }"#;

    #[test]
    fn parses_manifest_and_ignores_unknown_fields() {
        let manifest: CommunityNodeManifest = serde_json::from_str(SAMPLE_MANIFEST).unwrap();
        assert_eq!(manifest.node_role, "default-onboarding-node");
        assert_eq!(manifest.capability_scope.available_enabled.len(), 2);
        assert_eq!(
            manifest.capability_scope.planned_enabled,
            vec!["moderation"]
        );
        assert!(
            manifest
                .authority_scope
                .does_not_apply_to
                .contains(&"user_identity".to_string())
        );
        assert!(!manifest.p2p_boundary.network_wide_authority);
        assert_eq!(manifest.abuse_contact, "abuse@example-kukuri.net");
        assert_eq!(
            manifest.report_endpoint,
            "https://example-kukuri.net/v1/report"
        );
    }

    #[test]
    fn report_endpoint_defaults_to_empty_when_absent() {
        // report endpoint 未公開の node（Phase A）でも壊れず、空文字になる。
        let manifest: CommunityNodeManifest = serde_json::from_str("{}").unwrap();
        assert_eq!(manifest.report_endpoint, "");
    }

    #[test]
    fn defaults_fill_missing_fields() {
        // 最小 JSON でも default で補完され壊れない。
        let manifest: CommunityNodeManifest = serde_json::from_str("{}").unwrap();
        assert_eq!(manifest.node_role, "");
        assert!(manifest.capability_scope.available_enabled.is_empty());
        assert!(!manifest.p2p_boundary.identity_authority);
    }

    #[test]
    fn fetch_result_serializes_with_snake_case_status() {
        let fetch = CommunityNodeManifestFetch {
            status: CommunityNodeManifestFetchStatus::Absent,
            manifest: None,
        };
        let json = serde_json::to_value(&fetch).unwrap();
        assert_eq!(json["status"], "absent");
        assert!(json["manifest"].is_null());
    }

    // ---- WP-S3 T6: server 実出力との round-trip(rename drift 検出) ----
    //
    // 上の SAMPLE_MANIFEST 系テストは「手書き JSON に対する tolerant reader」
    // (欠落 default・未知フィールド無視)を検証する。ここは役割が異なり、
    // cn-operator の build_manifest が生成する実出力を slim 型が読めること、
    // すなわち server/client 間のフィールド rename drift を CI で検出する。
    // fail した場合は server 側 manifest.rs か本 slim 型の rename を疑うこと。

    const ROUND_TRIP_FIXTURE_YAML: &str = r#"server:
  domain: golden.example
  operator_name: Golden Operator
  country: JP
  cloud_provider: ExampleCloud
  region: example-region-1
  contact: abuse@golden.example
  node_id: node-golden-1
  node_name: Golden Node

profile: relay-enabled

features:
  community_index: true
  moderation: true
  community_local_trust: true
  report_endpoint: true
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

safety:
  profile: public-node
  policy_version: 2026-06-public-node-v1
  indexing:
    index_before_scan: false
    on_scan_error: hold
  storage:
    permanent_blob_storage: false
  events:
    emit_signed_moderation_events: true
    signing_key_secret_id: kukuri-cn-safety-signing-key
  providers:
    known_csam:
      provider: project-arachnid-shield
      required: true
      credential_secret_id: kukuri-cn-safety-known-csam

manifest:
  manifest_version: v1
  node_role: default-onboarding-node

acknowledge_planned_capabilities: true
"#;

    #[test]
    fn server_manifest_output_round_trips_into_slim_type() {
        let resolved = kukuri_cn_operator::load_and_validate(ROUND_TRIP_FIXTURE_YAML)
            .expect("fixture config validates");
        let server_json = serde_json::to_value(kukuri_cn_operator::build_manifest(&resolved))
            .expect("server manifest serializes");
        let slim: CommunityNodeManifest =
            serde_json::from_value(server_json.clone()).expect("slim parses server output");

        let expected = CommunityNodeManifest {
            node_id: "node-golden-1".to_string(),
            node_name: "Golden Node".to_string(),
            node_role: "default-onboarding-node".to_string(),
            server_name: "golden.example".to_string(),
            manifest_version: "v1".to_string(),
            capability_scope: CommunityNodeCapabilityScope {
                available_enabled: vec![
                    "auth_consent".to_string(),
                    "bootstrap_assist".to_string(),
                    "topic_rendezvous".to_string(),
                    "iroh_relay".to_string(),
                    "traffic_relay_fallback".to_string(),
                    "cloudflare_proxy".to_string(),
                    "report_endpoint".to_string(),
                ],
                planned_enabled: vec![
                    "community_index".to_string(),
                    "moderation".to_string(),
                    "community_local_trust".to_string(),
                ],
            },
            authority_scope: CommunityNodeAuthorityScope {
                applies_to: vec![
                    "this_node".to_string(),
                    "communities_indexed_by_this_node".to_string(),
                    "moderation_events_issued_by_this_node".to_string(),
                    "trust_signals_issued_by_this_node".to_string(),
                ],
                does_not_apply_to: vec![
                    "kukuri_network_as_a_whole".to_string(),
                    "third_party_nodes".to_string(),
                    "user_identity".to_string(),
                    "user_profile_canonical_source".to_string(),
                    "user_social_graph_canonical_source".to_string(),
                ],
            },
            p2p_boundary: CommunityNodeP2pBoundary {
                identity_authority: false,
                profile_canonical_store: false,
                social_graph_canonical_store: false,
                content_truth_source: false,
                network_wide_authority: false,
            },
            abuse_contact: "abuse@golden.example".to_string(),
            report_endpoint: "https://golden.example/v1/report".to_string(),
            terms_url: "https://golden.example/terms".to_string(),
            privacy_url: "https://golden.example/privacy".to_string(),
            moderation_policy_url: "https://golden.example/moderation-policy".to_string(),
        };
        assert_eq!(slim, expected);

        // slim が emit する全キー(nested 含む)が server 出力にも存在すること。
        // p2p_boundary のように「全フィールドが default 値」の構造は上の
        // assert_eq では rename drift を検出できないため、この包含が担う。
        let slim_json = serde_json::to_value(&slim).expect("slim serializes");
        assert_json_keys_subset(&slim_json, &server_json, "$");
    }

    fn assert_json_keys_subset(slim: &serde_json::Value, server: &serde_json::Value, path: &str) {
        if let serde_json::Value::Object(slim_map) = slim {
            let server_map = server
                .as_object()
                .unwrap_or_else(|| panic!("server value at {path} is not an object"));
            for (key, slim_child) in slim_map {
                let server_child = server_map.get(key).unwrap_or_else(|| {
                    panic!("server manifest is missing key {path}.{key} that the client reads")
                });
                assert_json_keys_subset(slim_child, server_child, &format!("{path}.{key}"));
            }
        }
    }
}
