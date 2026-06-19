use super::*;

/// community node manifest（#355/#356）の client 側 slim 表現。
///
/// dependency 表示に必要なフィールドのみを保持する。未知フィールドは無視し、欠落は
/// default で補う（manifest schema が拡張されても client が壊れない）。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunityNodeManifest {
    #[serde(default)]
    pub node_id: String,
    #[serde(default)]
    pub node_name: String,
    #[serde(default)]
    pub node_role: String,
    #[serde(default)]
    pub server_name: String,
    #[serde(default)]
    pub manifest_version: String,
    #[serde(default)]
    pub capability_scope: CommunityNodeCapabilityScope,
    #[serde(default)]
    pub authority_scope: CommunityNodeAuthorityScope,
    #[serde(default)]
    pub p2p_boundary: CommunityNodeP2pBoundary,
    #[serde(default)]
    pub abuse_contact: String,
    #[serde(default)]
    pub terms_url: String,
    #[serde(default)]
    pub privacy_url: String,
    #[serde(default)]
    pub moderation_policy_url: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunityNodeCapabilityScope {
    #[serde(default)]
    pub available_enabled: Vec<String>,
    #[serde(default)]
    pub planned_enabled: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunityNodeAuthorityScope {
    #[serde(default)]
    pub applies_to: Vec<String>,
    #[serde(default)]
    pub does_not_apply_to: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunityNodeP2pBoundary {
    #[serde(default)]
    pub identity_authority: bool,
    #[serde(default)]
    pub profile_canonical_store: bool,
    #[serde(default)]
    pub social_graph_canonical_store: bool,
    #[serde(default)]
    pub content_truth_source: bool,
    #[serde(default)]
    pub network_wide_authority: bool,
}

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
        let url = format!("{}/v1/node/manifest", base_url);
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
        "terms_url": "https://example-kukuri.net/terms"
    }"#;

    #[test]
    fn parses_manifest_and_ignores_unknown_fields() {
        let manifest: CommunityNodeManifest = serde_json::from_str(SAMPLE_MANIFEST).unwrap();
        assert_eq!(manifest.node_role, "default-onboarding-node");
        assert_eq!(manifest.capability_scope.available_enabled.len(), 2);
        assert_eq!(manifest.capability_scope.planned_enabled, vec!["moderation"]);
        assert!(
            manifest
                .authority_scope
                .does_not_apply_to
                .contains(&"user_identity".to_string())
        );
        assert!(!manifest.p2p_boundary.network_wide_authority);
        assert_eq!(manifest.abuse_contact, "abuse@example-kukuri.net");
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
}
