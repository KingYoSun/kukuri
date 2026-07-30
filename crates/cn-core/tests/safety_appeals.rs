//! appeal 経路 / operator レビューの Postgres integration テスト（#420 / ADR 0028 §2.3 / §2.8）。
//!
//! `KUKURI_CN_RUN_INTEGRATION_TESTS=1` のときだけ実 DB に接続して実行する。
//!
//! ADR 0028 contract:
//! - `false_positive_appeal_path_exists`
//! - `appeal_cleared_propagates_and_reverts_trust_contribution`
//! - `operator_review_can_edit_detection_metadata`

use anyhow::Result;
use kukuri_cn_core::{
    DistributionAudience, RiskSignalCorrection, RiskSignalMetadataEdit, TestDatabase,
    connect_postgres, dispute_risk_signal, edit_risk_signal_detection_metadata, get_risk_signal,
    initialize_database, list_distributable_risk_signals, list_trust_risk_inputs,
    persist_risk_signal, reissue_corrected_risk_signal, update_risk_signal_appeal_status,
};
use kukuri_cn_safety::{
    AppealStatus, Basis, RiskSignalTarget, SafetyCategory, SafetyRiskSignal, Severity, Visibility,
};

const DEFAULT_ADMIN_DATABASE_URL: &str = "postgres://cn:cn_password@127.0.0.1:15432/cn";
const ISSUER: &str = "issuer-node-1";
const NOW: &str = "2026-07-30T09:00:00Z";

fn integration_test_admin_database_url() -> Option<String> {
    kukuri_test_support::gated_env_url(
        "KUKURI_CN_RUN_INTEGRATION_TESTS",
        "COMMUNITY_NODE_DATABASE_URL",
        DEFAULT_ADMIN_DATABASE_URL,
    )
}

/// VLM 由来の suspected signal（basis = ClassifierScore、配布可 visibility）。
fn suspected_signal(target_id: &str, category: SafetyCategory) -> SafetyRiskSignal {
    SafetyRiskSignal {
        target: RiskSignalTarget::BlobCid,
        target_id: target_id.to_string(),
        category,
        severity: Severity::Critical,
        basis: Basis::ClassifierScore,
        confidence: Some(90),
        visibility: Visibility::SubscribedNodes,
        expires_at: None,
        appeal_status: Some(AppealStatus::None),
    }
}

#[tokio::test]
async fn false_positive_appeal_path_exists() -> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!(
            "skipping cn-core safety appeals integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1"
        );
        return Ok(());
    };
    let database = TestDatabase::create(admin_url.as_str(), "cn_core_appeal_path").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    let result = async {
        initialize_database(&pool).await?;

        // issuer node が発行した suspected advisory に対し、user / client が
        // （/v1/report の appeal 参照経由で）異議を申し立てられる。
        let stored = persist_risk_signal(
            &pool,
            ISSUER,
            &suspected_signal("bafy-fp", SafetyCategory::Csam),
        )
        .await?;
        assert_eq!(
            stored.signal.appeal_status,
            Some(AppealStatus::None),
            "advisory starts undisputed"
        );

        let disputed = dispute_risk_signal(&pool, &stored.id).await?;
        assert_eq!(disputed.signal.appeal_status, Some(AppealStatus::Disputed));

        // 冪等: 二重申し立ては同じ Disputed のまま成功する。
        let again = dispute_risk_signal(&pool, &stored.id).await?;
        assert_eq!(again.signal.appeal_status, Some(AppealStatus::Disputed));

        // operator は申し立て中の advisory を参照できる（レビュー導線）。
        let visible = get_risk_signal(&pool, &stored.id).await?.expect("exists");
        assert_eq!(visible.signal.appeal_status, Some(AppealStatus::Disputed));

        // 不正な遷移（None → Cleared の飛び越し）は拒否される。
        let other = persist_risk_signal(
            &pool,
            ISSUER,
            &suspected_signal("bafy-2", SafetyCategory::Nsfw),
        )
        .await?;
        assert!(
            update_risk_signal_appeal_status(&pool, &other.id, AppealStatus::Cleared)
                .await
                .is_err(),
            "None -> Cleared must be rejected"
        );

        Ok::<(), anyhow::Error>(())
    }
    .await;
    database.cleanup().await?;
    result
}

#[tokio::test]
async fn appeal_cleared_propagates_and_reverts_trust_contribution() -> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!(
            "skipping cn-core safety appeals integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1"
        );
        return Ok(());
    };
    let database = TestDatabase::create(admin_url.as_str(), "cn_core_appeal_cleared").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    let result = async {
        initialize_database(&pool).await?;

        let stored = persist_risk_signal(
            &pool,
            ISSUER,
            &suspected_signal("pubkey-target", SafetyCategory::Csam),
        )
        .await?;

        // Cleared 前は trust 入力に寄与している。
        let inputs =
            list_trust_risk_inputs(&pool, RiskSignalTarget::BlobCid, "pubkey-target", NOW).await?;
        assert_eq!(inputs.absolute.len(), 1);

        // Disputed → Cleared（operator 認容）。
        dispute_risk_signal(&pool, &stored.id).await?;
        let cleared =
            update_risk_signal_appeal_status(&pool, &stored.id, AppealStatus::Cleared).await?;
        assert_eq!(cleared.signal.appeal_status, Some(AppealStatus::Cleared));

        // 伝播: Cleared の signal は失効させず、配布クエリに **残る**（受け手が
        // appeal_status を見て寄与を除外できるように配布し続ける）。
        let distributable = list_distributable_risk_signals(
            &pool,
            DistributionAudience::SubscribedNodes,
            NOW,
            50,
            0,
        )
        .await?;
        let found = distributable
            .iter()
            .find(|s| s.id == stored.id)
            .expect("cleared advisory keeps distributing the correction");
        assert_eq!(found.signal.appeal_status, Some(AppealStatus::Cleared));

        // trust 寄与の戻し: 供給層が Cleared を除外する（絶対成分への負寄与が消える）。
        let inputs =
            list_trust_risk_inputs(&pool, RiskSignalTarget::BlobCid, "pubkey-target", NOW).await?;
        assert!(inputs.absolute.is_empty());
        assert!(inputs.relative.is_empty());

        Ok::<(), anyhow::Error>(())
    }
    .await;
    database.cleanup().await?;
    result
}

#[tokio::test]
async fn operator_review_can_edit_detection_metadata() -> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!(
            "skipping cn-core safety appeals integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1"
        );
        return Ok(());
    };
    let database = TestDatabase::create(admin_url.as_str(), "cn_core_operator_review").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    let result = async {
        initialize_database(&pool).await?;

        let stored = persist_risk_signal(
            &pool,
            ISSUER,
            &suspected_signal("bafy-edit", SafetyCategory::Csam),
        )
        .await?;

        // operator レビューが無効（既定）のままでは編集できない（optional の明示的有効化）。
        let edit = RiskSignalMetadataEdit {
            category: Some(SafetyCategory::Nsfw),
            severity: Some(Severity::Low),
            confidence: Some(20),
            expires_at: None,
        };
        assert!(
            edit_risk_signal_detection_metadata(&pool, &stored.id, &edit, false)
                .await
                .is_err(),
            "operator review must be explicitly enabled"
        );

        // 有効化すると検知メタデータを直接編集できる（node-local advisory の是正。
        // user canonical state は変更しない = risk_signals 行の更新のみ）。
        let edited = edit_risk_signal_detection_metadata(&pool, &stored.id, &edit, true).await?;
        assert_eq!(edited.signal.category, SafetyCategory::Nsfw);
        assert_eq!(edited.signal.severity, Severity::Low);
        assert_eq!(edited.signal.confidence, Some(20));

        // 訂正 signal の再発行: 旧 signal は失効し、訂正版が同じ issuer で新規発行される。
        let correction = RiskSignalCorrection {
            category: Some(SafetyCategory::Spam),
            severity: None,
            confidence: Some(10),
            visibility: None,
        };
        let reissued =
            reissue_corrected_risk_signal(&pool, &stored.id, &correction, NOW, true).await?;
        assert_ne!(reissued.id, stored.id);
        assert_eq!(reissued.issuer_node_id, ISSUER);
        assert_eq!(reissued.signal.category, SafetyCategory::Spam);
        assert_eq!(reissued.signal.confidence, Some(10));
        assert!(reissued.signal.expires_at.is_none());

        let old = get_risk_signal(&pool, &stored.id).await?.expect("exists");
        assert_eq!(old.signal.expires_at.as_deref(), Some(NOW));

        // 失効した旧 signal は配布からも trust 入力からも消える（NOW より後で判定）。
        let later = "2026-07-30T10:00:00Z";
        let distributable = list_distributable_risk_signals(
            &pool,
            DistributionAudience::SubscribedNodes,
            later,
            50,
            0,
        )
        .await?;
        assert!(distributable.iter().all(|s| s.id != stored.id));

        Ok::<(), anyhow::Error>(())
    }
    .await;
    database.cleanup().await?;
    result
}
