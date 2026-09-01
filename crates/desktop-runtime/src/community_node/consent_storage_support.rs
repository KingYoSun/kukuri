use super::*;

/// Node ごとのローカル同意記録(#857)。
///
/// Node 同意の成立判定の SSoT はこのローカル記録で、サーバの `/v1/consents` は
/// 認証後の同期先。記録は Node 識別子(base_url)× policy slug × 版で持ち、
/// 同意時の表示言語とアプリ版も保存する。撤回は記録を消さず `withdrawn_at` で
/// 表現する(過去の同意記録は履歴として保持する)。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CommunityNodeLocalConsentRecord {
    pub policy_slug: String,
    pub policy_version: i32,
    pub accepted_at: i64,
    pub language: String,
    pub app_version: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(optional_fields = nullable))]
pub struct CommunityNodeLocalConsentState {
    #[serde(default)]
    pub records: Vec<CommunityNodeLocalConsentRecord>,
    #[serde(default)]
    pub withdrawn_at: Option<i64>,
}

impl CommunityNodeLocalConsentState {
    /// 撤回されておらず、少なくとも 1 件の同意記録があるか。
    /// これが偽の node へは公開 manifest / 法務文書以外の通信を開始しない。
    pub fn has_active_consent(&self) -> bool {
        self.withdrawn_at.is_none() && !self.records.is_empty()
    }
}

pub(crate) fn load_community_node_local_consents(
    db_path: &Path,
    mode: IdentityStorageMode,
    base_url: &str,
) -> Result<CommunityNodeLocalConsentState> {
    let Some(raw) = load_optional_secret(db_path, mode, COMMUNITY_NODE_CONSENT_PURPOSE, base_url)?
    else {
        return Ok(CommunityNodeLocalConsentState::default());
    };
    serde_json::from_str::<CommunityNodeLocalConsentState>(&raw)
        .context("failed to decode persisted community-node consents")
}

pub(crate) fn persist_community_node_local_consents(
    db_path: &Path,
    mode: IdentityStorageMode,
    base_url: &str,
    state: &CommunityNodeLocalConsentState,
) -> Result<()> {
    let encoded =
        serde_json::to_string(state).context("failed to encode community-node consents")?;
    persist_optional_secret(
        db_path,
        mode,
        COMMUNITY_NODE_CONSENT_PURPOSE,
        base_url,
        encoded.as_str(),
    )
}

/// 公開カタログ(`GET /v1/policies`)の required 文書すべてを、現行版以上で
/// ローカル同意済みか。認証(JWT 発行)を開始してよいかの判定に使う。
pub(crate) fn community_node_local_consent_satisfies_policies(
    state: &CommunityNodeLocalConsentState,
    policies: &[CommunityNodePolicyDocument],
) -> bool {
    state.withdrawn_at.is_none()
        && policies
            .iter()
            .filter(|policy| policy.required)
            .all(|policy| {
                state.records.iter().any(|record| {
                    record.policy_slug == policy.policy_slug
                        && record.policy_version >= policy.policy_version
                })
            })
}

/// サーバの consent status が示す required 文書すべてを現行版以上でローカル同意済みか。
/// 真なら POST /v1/consents での同期(auto 受諾)を許可し、偽なら再同意待ちとして
/// セッションを進めない(#857: 重要変更時の再同意)。
pub(crate) fn community_node_local_consent_covers_status(
    state: &CommunityNodeLocalConsentState,
    status: &CommunityNodeConsentStatus,
) -> bool {
    state.withdrawn_at.is_none()
        && status
            .items
            .iter()
            .filter(|item| item.required)
            .all(|item| {
                state.records.iter().any(|record| {
                    record.policy_slug == item.policy_slug
                        && record.policy_version >= item.policy_version
                })
            })
}

/// 同意記録を追記する。同一 slug + 版の既存記録は日時・言語・アプリ版を更新し、
/// 別版の記録は履歴として残す。撤回状態は解除される。
pub(crate) fn record_community_node_local_consents(
    state: &mut CommunityNodeLocalConsentState,
    documents: &[CommunityNodeConsentDocumentRef],
    language: &str,
    app_version: &str,
    accepted_at: i64,
) {
    state.withdrawn_at = None;
    for document in documents {
        if let Some(existing) = state.records.iter_mut().find(|record| {
            record.policy_slug == document.policy_slug
                && record.policy_version == document.policy_version
        }) {
            existing.accepted_at = accepted_at;
            existing.language = language.to_string();
            existing.app_version = app_version.to_string();
        } else {
            state.records.push(CommunityNodeLocalConsentRecord {
                policy_slug: document.policy_slug.clone(),
                policy_version: document.policy_version,
                accepted_at,
                language: language.to_string(),
                app_version: app_version.to_string(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(slug: &str, version: i32) -> CommunityNodeLocalConsentRecord {
        CommunityNodeLocalConsentRecord {
            policy_slug: slug.to_string(),
            policy_version: version,
            accepted_at: 1_700_000_000,
            language: "ja".to_string(),
            app_version: "0.1.8".to_string(),
        }
    }

    fn policy(slug: &str, version: i32, required: bool) -> CommunityNodePolicyDocument {
        CommunityNodePolicyDocument {
            policy_slug: slug.to_string(),
            policy_version: version,
            title: slug.to_string(),
            body_markdown: "body".to_string(),
            required,
        }
    }

    #[test]
    fn active_consent_requires_records_and_no_withdrawal() {
        assert!(!CommunityNodeLocalConsentState::default().has_active_consent());
        let state = CommunityNodeLocalConsentState {
            records: vec![record("terms", 1)],
            withdrawn_at: None,
        };
        assert!(state.has_active_consent());
        let withdrawn = CommunityNodeLocalConsentState {
            withdrawn_at: Some(1_700_000_001),
            ..state
        };
        assert!(!withdrawn.has_active_consent());
    }

    #[test]
    fn satisfies_policies_requires_current_or_newer_version_per_required_policy() {
        let state = CommunityNodeLocalConsentState {
            records: vec![record("terms", 2)],
            withdrawn_at: None,
        };
        assert!(community_node_local_consent_satisfies_policies(
            &state,
            &[policy("terms", 2, true)]
        ));
        // 版が上がったら再同意が必要。
        assert!(!community_node_local_consent_satisfies_policies(
            &state,
            &[policy("terms", 3, true)]
        ));
        // required でない文書は判定に影響しない。
        assert!(community_node_local_consent_satisfies_policies(
            &state,
            &[policy("terms", 2, true), policy("optional", 1, false)]
        ));
        // 未同意の required 文書があれば不成立。
        assert!(!community_node_local_consent_satisfies_policies(
            &state,
            &[policy("terms", 2, true), policy("privacy", 1, true)]
        ));
        // 撤回済みは常に不成立。
        let withdrawn = CommunityNodeLocalConsentState {
            withdrawn_at: Some(1_700_000_001),
            ..state
        };
        assert!(!community_node_local_consent_satisfies_policies(
            &withdrawn,
            &[policy("terms", 2, true)]
        ));
    }

    #[test]
    fn record_consents_appends_history_and_clears_withdrawal() {
        let mut state = CommunityNodeLocalConsentState {
            records: vec![record("terms", 1)],
            withdrawn_at: Some(1_700_000_001),
        };
        record_community_node_local_consents(
            &mut state,
            &[CommunityNodeConsentDocumentRef {
                policy_slug: "terms".to_string(),
                policy_version: 2,
            }],
            "en",
            "0.2.0",
            1_700_000_100,
        );
        assert_eq!(state.withdrawn_at, None);
        // 旧版の記録は履歴として残る。
        assert_eq!(state.records.len(), 2);
        let latest = state
            .records
            .iter()
            .find(|record| record.policy_version == 2)
            .expect("new record");
        assert_eq!(latest.language, "en");
        assert_eq!(latest.app_version, "0.2.0");
        assert_eq!(latest.accepted_at, 1_700_000_100);
    }
}
