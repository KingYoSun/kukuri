//! signed moderation event / risk signal の永続化と配布境界の Postgres integration テスト（#405）。
//!
//! `KUKURI_CN_RUN_INTEGRATION_TESTS=1` のときだけ実 DB に接続して実行する。
//! - signed moderation event の保存・取得・冪等・ロード後署名検証。
//! - risk signal の保存・取得・対象別一覧。
//! - visibility 配布境界（local 除外 / subscribed_nodes / public）と expires_at 失効。

use anyhow::Result;
use kukuri_cn_core::{
    DistributionAudience, TestDatabase, connect_postgres, get_signed_moderation_event,
    initialize_database, list_distributable_moderation_events, list_distributable_risk_signals,
    list_risk_signals_for_target, persist_risk_signal, persist_signed_moderation_event,
};
use kukuri_cn_safety::event::{ModerationEventBody, SignedModerationEvent, issue_signed_event};
use kukuri_cn_safety::provider::SubjectKind;
use kukuri_cn_safety::{
    AppealStatus, Basis, ModerationAction, ModerationEventSigner, ReasonCode, RiskSignalTarget,
    SafetyCategory, SafetyLabel, SafetyRiskSignal, Severity, Visibility,
};
use kukuri_cn_safety_runtime::{Secp256k1ModerationEventSigner, verify_signed_event};

const DEFAULT_ADMIN_DATABASE_URL: &str = "postgres://cn:cn_password@127.0.0.1:15432/cn";
const TEST_SECRET: &str = "0000000000000000000000000000000000000000000000000000000000000001";

fn integration_test_admin_database_url() -> Option<String> {
    let enabled = std::env::var("KUKURI_CN_RUN_INTEGRATION_TESTS")
        .ok()
        .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false);
    if !enabled {
        return None;
    }
    Some(
        std::env::var("COMMUNITY_NODE_DATABASE_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_ADMIN_DATABASE_URL.to_string()),
    )
}

fn signer() -> Secp256k1ModerationEventSigner {
    Secp256k1ModerationEventSigner::from_secret(TEST_SECRET).unwrap()
}

fn event_body(
    issuer: &str,
    id: &str,
    target_id: &str,
    visibility: Visibility,
) -> ModerationEventBody {
    ModerationEventBody {
        id: id.to_string(),
        issuer_node_id: issuer.to_string(),
        target_type: SubjectKind::Blob,
        target_id: target_id.to_string(),
        action: ModerationAction::Exclude,
        labels: vec![SafetyLabel::new(SafetyCategory::Csam)],
        reason_code: ReasonCode::CsamConfirmed,
        severity: Severity::Critical,
        confidence: Some(98),
        basis: Basis::KnownHashMatch,
        visibility,
        policy_version: "2026-06-public-node-v1".to_string(),
        created_at: "2026-06-29T00:00:00Z".to_string(),
    }
}

fn risk_signal(
    target_id: &str,
    visibility: Visibility,
    expires_at: Option<&str>,
) -> SafetyRiskSignal {
    SafetyRiskSignal {
        target: RiskSignalTarget::BlobCid,
        target_id: target_id.to_string(),
        category: SafetyCategory::Csam,
        severity: Severity::Critical,
        basis: Basis::KnownHashMatch,
        confidence: None,
        visibility,
        expires_at: expires_at.map(str::to_string),
        appeal_status: Some(AppealStatus::None),
    }
}

#[tokio::test]
async fn signed_moderation_event_persists_and_verifies_after_load() -> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!(
            "skipping cn-core safety integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1"
        );
        return Ok(());
    };
    let database = TestDatabase::create(admin_url.as_str(), "cn_core_safety_events").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    let result = async {
        initialize_database(&pool).await?;
        let signer = signer();
        let issuer = signer.issuer_node_id().to_string();

        let signed: SignedModerationEvent = issue_signed_event(
            event_body(&issuer, "evt-1", "bafy-1", Visibility::SubscribedNodes),
            &signer,
        );
        verify_signed_event(&signed).expect("freshly signed event verifies");

        let stored = persist_signed_moderation_event(&pool, &signed).await?;
        // 保存した event は body / signature ごとロードでき、署名検証が通る
        // （event_created_at が原文のまま保持され canonical digest が一致する）。
        assert_eq!(stored.event, signed);
        verify_signed_event(&stored.event).expect("loaded event still verifies");

        let fetched = get_signed_moderation_event(&pool, "evt-1")
            .await?
            .expect("event exists");
        assert_eq!(fetched.event, signed);

        // 同一 id の再 insert は冪等（最初の writer が権威。重複保存しない）。
        let again = persist_signed_moderation_event(&pool, &signed).await?;
        assert_eq!(again.event, signed);

        // body を改竄した event は保存前署名検証で拒否し、DB に残さない。
        let mut tampered = signed.clone();
        tampered.body.id = "evt-tampered".to_string();
        tampered.body.target_id = "bafy-tampered".to_string();
        assert!(
            persist_signed_moderation_event(&pool, &tampered)
                .await
                .is_err()
        );
        assert!(
            get_signed_moderation_event(&pool, "evt-tampered")
                .await?
                .is_none()
        );

        Ok::<(), anyhow::Error>(())
    }
    .await;
    database.cleanup().await?;
    result
}

#[tokio::test]
async fn moderation_event_distribution_excludes_local() -> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!(
            "skipping cn-core safety integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1"
        );
        return Ok(());
    };
    let database = TestDatabase::create(admin_url.as_str(), "cn_core_safety_event_dist").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    let result = async {
        initialize_database(&pool).await?;
        let signer = signer();
        let issuer = signer.issuer_node_id().to_string();

        for (id, target, visibility) in [
            ("evt-local", "bafy-local", Visibility::Local),
            (
                "evt-subscribed",
                "bafy-subscribed",
                Visibility::SubscribedNodes,
            ),
            ("evt-public", "bafy-public", Visibility::Public),
        ] {
            let signed = issue_signed_event(event_body(&issuer, id, target, visibility), &signer);
            persist_signed_moderation_event(&pool, &signed).await?;
        }

        // subscribed audience は subscribed_nodes + public を見るが local は見ない。
        let subscribed = list_distributable_moderation_events(
            &pool,
            DistributionAudience::SubscribedNodes,
            50,
            0,
        )
        .await?;
        let ids: Vec<&str> = subscribed
            .iter()
            .map(|e| e.event.body.id.as_str())
            .collect();
        assert!(ids.contains(&"evt-subscribed"));
        assert!(ids.contains(&"evt-public"));
        assert!(!ids.contains(&"evt-local"), "local must not distribute");

        // public audience は public のみ。
        let public =
            list_distributable_moderation_events(&pool, DistributionAudience::Public, 50, 0)
                .await?;
        let public_ids: Vec<&str> = public.iter().map(|e| e.event.body.id.as_str()).collect();
        assert_eq!(public_ids, vec!["evt-public"]);

        Ok::<(), anyhow::Error>(())
    }
    .await;
    database.cleanup().await?;
    result
}

#[tokio::test]
async fn risk_signal_persists_and_distribution_respects_visibility_and_expiry() -> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!(
            "skipping cn-core safety integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1"
        );
        return Ok(());
    };
    let database = TestDatabase::create(admin_url.as_str(), "cn_core_safety_risk").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    let result = async {
        initialize_database(&pool).await?;
        let issuer = "node-issuer";

        // local: 配布しない。
        persist_risk_signal(
            &pool,
            issuer,
            &risk_signal("bafy-local", Visibility::Local, None),
        )
        .await?;
        // subscribed: 配布する（無期限）。
        let subscribed_stored = persist_risk_signal(
            &pool,
            issuer,
            &risk_signal("bafy-sub", Visibility::SubscribedNodes, None),
        )
        .await?;
        // public だが失効済み: 配布しない。
        persist_risk_signal(
            &pool,
            issuer,
            &risk_signal(
                "bafy-expired",
                Visibility::Public,
                Some("2020-01-01T00:00:00Z"),
            ),
        )
        .await?;
        // public かつ未失効: 配布する。
        persist_risk_signal(
            &pool,
            issuer,
            &risk_signal(
                "bafy-future",
                Visibility::Public,
                Some("2999-01-01T00:00:00Z"),
            ),
        )
        .await?;

        // 対象別一覧（visibility 非依存・node-local 参照）。
        let by_target =
            list_risk_signals_for_target(&pool, RiskSignalTarget::BlobCid, "bafy-sub").await?;
        assert_eq!(by_target.len(), 1);
        assert_eq!(by_target[0].id, subscribed_stored.id);
        assert_eq!(by_target[0].issuer_node_id, issuer);

        // 配布境界（now=2026 時点）。
        let now = "2026-06-29T00:00:00Z";
        let subscribed = list_distributable_risk_signals(
            &pool,
            DistributionAudience::SubscribedNodes,
            now,
            50,
            0,
        )
        .await?;
        let targets: Vec<&str> = subscribed
            .iter()
            .map(|s| s.signal.target_id.as_str())
            .collect();
        assert!(targets.contains(&"bafy-sub"));
        assert!(targets.contains(&"bafy-future"));
        assert!(
            !targets.contains(&"bafy-local"),
            "local must not distribute"
        );
        assert!(
            !targets.contains(&"bafy-expired"),
            "expired must not distribute"
        );

        // public audience は public のみ（未失効）。
        let public =
            list_distributable_risk_signals(&pool, DistributionAudience::Public, now, 50, 0)
                .await?;
        let public_targets: Vec<&str> =
            public.iter().map(|s| s.signal.target_id.as_str()).collect();
        assert_eq!(public_targets, vec!["bafy-future"]);

        Ok::<(), anyhow::Error>(())
    }
    .await;
    database.cleanup().await?;
    result
}

#[tokio::test]
async fn empty_target_id_is_rejected() -> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!(
            "skipping cn-core safety integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1"
        );
        return Ok(());
    };
    let database = TestDatabase::create(admin_url.as_str(), "cn_core_safety_empty").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    let result = async {
        initialize_database(&pool).await?;
        let signer = signer();
        let issuer = signer.issuer_node_id().to_string();

        // 空白 target_id の moderation event は保存しない。
        let signed = issue_signed_event(
            event_body(&issuer, "evt-empty", "   ", Visibility::Public),
            &signer,
        );
        assert!(
            persist_signed_moderation_event(&pool, &signed)
                .await
                .is_err()
        );

        // 空白 target_id の risk signal も保存しない。
        assert!(
            persist_risk_signal(&pool, &issuer, &risk_signal("  ", Visibility::Public, None))
                .await
                .is_err()
        );

        Ok::<(), anyhow::Error>(())
    }
    .await;
    database.cleanup().await?;
    result
}
