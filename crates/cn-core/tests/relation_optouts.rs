//! relation opt-out「見えない」（ADR 0026 §2.6 / §6.3, #415）の Postgres integration テスト。
//!
//! `KUKURI_CN_RUN_INTEGRATION_TESTS=1` のときだけ実 DB に接続して実行する。
//! - `relation_visibility_choice_is_user_controlled_and_reversible`: set → clear の往復・冪等性。
//! - opt-out は trust 入力（`list_trust_risk_inputs`）へ影響しない（troll 判定回避の手段にしない）。

use anyhow::Result;

use kukuri_cn_core::{
    TestDatabase, clear_relation_optout, connect_postgres, filter_relation_visible,
    get_relation_optout, initialize_database, is_relation_opted_out, list_trust_risk_inputs,
    persist_risk_signal, set_relation_optout,
};
use kukuri_cn_safety::{
    Basis, RiskSignalTarget, SafetyCategory, SafetyRiskSignal, Severity, Visibility,
};

const DEFAULT_ADMIN_DATABASE_URL: &str = "postgres://cn:cn_password@127.0.0.1:15432/cn";

fn integration_test_admin_database_url() -> Option<String> {
    kukuri_test_support::gated_env_url(
        "KUKURI_CN_RUN_INTEGRATION_TESTS",
        "COMMUNITY_NODE_DATABASE_URL",
        DEFAULT_ADMIN_DATABASE_URL,
    )
}

const PUBKEY_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const PUBKEY_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[tokio::test]
async fn relation_visibility_choice_is_user_controlled_and_reversible() -> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-core relation optout test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let database = TestDatabase::create(admin_url.as_str(), "cn_relation_optouts").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    let result = async {
        initialize_database(&pool).await?;

        // 既定は opt-out していない（既定で隠さない, §2.6）。
        assert!(!is_relation_opted_out(&pool, PUBKEY_A).await?);

        // set は冪等で、opted_out_at（いつから見えないか）を説明できる。
        set_relation_optout(&pool, PUBKEY_A).await?;
        set_relation_optout(&pool, PUBKEY_A).await?;
        assert!(is_relation_opted_out(&pool, PUBKEY_A).await?);
        assert!(get_relation_optout(&pool, PUBKEY_A).await?.is_some());

        // discovery / relation read の候補 filter から opt-out 済みが落ちる。
        let visible =
            filter_relation_visible(&pool, &[PUBKEY_A.to_string(), PUBKEY_B.to_string()]).await?;
        assert_eq!(visible, vec![PUBKEY_B.to_string()]);

        // 可逆: clear で見え直す（canonical 削除ではなく node-local な選択の解除）。
        clear_relation_optout(&pool, PUBKEY_A).await?;
        clear_relation_optout(&pool, PUBKEY_A).await?; // 冪等
        assert!(!is_relation_opted_out(&pool, PUBKEY_A).await?);
        let visible =
            filter_relation_visible(&pool, &[PUBKEY_A.to_string(), PUBKEY_B.to_string()]).await?;
        assert_eq!(visible.len(), 2);

        // 不正 pubkey は拒否（黙って通さない）。
        assert!(set_relation_optout(&pool, "not-a-pubkey").await.is_err());
        anyhow::Ok(())
    }
    .await;
    database.cleanup().await?;
    result
}

#[tokio::test]
async fn relation_optout_does_not_affect_trust_inputs() -> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-core relation optout test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let database = TestDatabase::create(admin_url.as_str(), "cn_optout_trust").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    let result = async {
        initialize_database(&pool).await?;

        persist_risk_signal(
            &pool,
            "issuer-node",
            &SafetyRiskSignal {
                target: RiskSignalTarget::UserPubkey,
                target_id: PUBKEY_A.to_string(),
                category: SafetyCategory::Spam,
                severity: Severity::Medium,
                basis: Basis::ClassifierScore,
                confidence: Some(80),
                visibility: Visibility::Local,
                expires_at: None,
                appeal_status: None,
            },
        )
        .await?;

        let before = list_trust_risk_inputs(
            &pool,
            RiskSignalTarget::UserPubkey,
            PUBKEY_A,
            "2026-07-02T09:00:00Z",
        )
        .await?;
        assert_eq!(before.relative.len(), 1);

        // opt-out しても trust 入力は 1 件も減らない（troll 判定回避の手段にしない, §6.3）。
        set_relation_optout(&pool, PUBKEY_A).await?;
        let after = list_trust_risk_inputs(
            &pool,
            RiskSignalTarget::UserPubkey,
            PUBKEY_A,
            "2026-07-02T09:00:00Z",
        )
        .await?;
        assert_eq!(before, after);
        anyhow::Ok(())
    }
    .await;
    database.cleanup().await?;
    result
}
