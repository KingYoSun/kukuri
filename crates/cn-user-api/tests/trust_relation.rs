//! #415 trust / relation read surface（`CommunityLocalTrust`）の contract test。
//!
//! `GET /v1/trust/users/{pubkey}` / `GET /v1/trust/pull/{pubkey}` /
//! `GET /v1/relation/users/{target}` / `GET /v1/relation/neighbors` /
//! `PUT|DELETE /v1/relation/optout` を ADR 0026 の contract に沿って固定する。
//! 機能未構成（既定。`CommunityLocalTrust` = `Availability::Planned`）の node は 404。
//!
//! Postgres + Redis を要するため `KUKURI_CN_RUN_INTEGRATION_TESTS=1` で gate する。
//! relation graph は in-memory（`MemoryRelationStore`）を `with_trust_read` で注入する
//! （proximity の合成は ArcadeDB 実装と共通の `proximity_from_features`）。

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use kukuri_cn_core::{
    JwtConfig, NewCommunityNodeReport, TestDatabase, connect_postgres,
    insert_community_node_report, persist_risk_signal,
};
use kukuri_cn_protocol::build_auth_envelope_json;
use kukuri_cn_safety::{
    Basis, RiskSignalTarget, SafetyCategory, SafetyRiskSignal, Severity, Visibility,
};
use kukuri_cn_trust::{
    EdgeFeatures, FEATURE_SHARED_TOPICS, MemoryRelationStore, RelationStore, TrustParams,
};
use kukuri_cn_user_api::{
    RelationVisibilityState, TrustReadState, UserApiConfig, app_router, build_state,
};
use kukuri_core::{KukuriKeys, generate_keys};
use reqwest::{Client, StatusCode};

mod support;
use support::{integration_test_admin_database_url, integration_test_rendezvous_redis_url};

struct TestServer {
    task: tokio::task::JoinHandle<()>,
    database: TestDatabase,
    base_url: String,
}

impl TestServer {
    /// trust / relation read（in-memory relation）を注入した user-api server を起動する。
    /// `trust` が None の場合は機能未構成の node（`/v1/trust/*` / `/v1/relation/*` は 404）。
    async fn spawn(
        admin_database_url: &str,
        prefix: &str,
        trust: Option<Arc<TrustReadState>>,
    ) -> Result<Self> {
        let database = TestDatabase::create(admin_database_url, prefix).await?;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .context("failed to bind test trust read listener")?;
        let addr = listener.local_addr()?;
        let base_url = format!("http://{addr}");
        let mut state = build_state(&UserApiConfig {
            bind_addr: addr,
            database_url: database.database_url.clone(),
            rendezvous_redis_url: integration_test_rendezvous_redis_url(),
            rendezvous_key_prefix: format!("cn:test:{prefix}"),
            base_url: base_url.clone(),
            public_base_url: base_url.clone(),
            connectivity_urls: vec!["http://127.0.0.1:13340".to_string()],
            jwt_config: JwtConfig::new("kukuri-cn-tests", "test-secret", 3600),
            operator_config_path: None,
            channel_secret_key: None,
            index_query_enabled: false,
            trust_read_enabled: false,
            relation_distance_optout_min_proximity: None,
            deployment_revision: "test-deployment-v1".to_string(),
            readiness_activation_max_age_secs: 3600,
        })
        .await?;
        if let Some(trust) = trust {
            let relation_visibility =
                Arc::new(RelationVisibilityState::new(trust.relation.clone(), 0.5)?);
            state = state
                .with_trust_read(trust)
                .with_relation_visibility(relation_visibility);
        }
        let app = app_router(state);
        let task = tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .expect("trust read server");
        });
        Ok(Self {
            task,
            database,
            base_url,
        })
    }

    async fn shutdown(self) -> Result<()> {
        self.task.abort();
        self.database.cleanup().await
    }
}

/// 認証 + consent を通し、bearer access token を返す。
async fn authenticate_and_consent(
    client: &Client,
    base_url: &str,
    keys: &KukuriKeys,
) -> Result<String> {
    let pubkey = keys.public_key_hex();
    let challenge = client
        .post(format!("{base_url}/v1/auth/challenge"))
        .json(&serde_json::json!({ "pubkey": pubkey }))
        .send()
        .await?
        .error_for_status()?
        .json::<kukuri_cn_protocol::AuthChallengeResponse>()
        .await?;
    let auth_envelope_json =
        build_auth_envelope_json(keys, challenge.challenge.as_str(), base_url)?;
    let verify = client
        .post(format!("{base_url}/v1/auth/verify"))
        .json(&serde_json::json!({
            "auth_envelope_json": auth_envelope_json,
            "endpoint_id": "peer-a",
        }))
        .send()
        .await?
        .error_for_status()?
        .json::<kukuri_cn_protocol::AuthVerifyResponse>()
        .await?;
    client
        .post(format!("{base_url}/v1/consents"))
        .bearer_auth(verify.access_token.as_str())
        .json(&serde_json::json!({ "policy_slugs": [] }))
        .send()
        .await?
        .error_for_status()?;
    Ok(verify.access_token)
}

fn memory_trust_state() -> (Arc<TrustReadState>, Arc<MemoryRelationStore>) {
    let relation = Arc::new(MemoryRelationStore::new());
    let state = Arc::new(TrustReadState {
        params: TrustParams::default(),
        relation: relation.clone(),
    });
    (state, relation)
}

fn risk_signal(
    target_id: &str,
    category: SafetyCategory,
    severity: Severity,
    basis: Basis,
    visibility: Visibility,
) -> SafetyRiskSignal {
    SafetyRiskSignal {
        target: RiskSignalTarget::UserPubkey,
        target_id: target_id.to_string(),
        category,
        severity,
        basis,
        confidence: Some(100),
        visibility,
        expires_at: None,
        appeal_status: None,
    }
}

#[tokio::test]
async fn trust_read_is_not_found_when_not_configured() -> Result<()> {
    // 既定（機能無効 = `CommunityLocalTrust` Planned 相当）の node は trust / relation read を
    // 公開しない。
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api trust read test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server = TestServer::spawn(admin_database_url.as_str(), "cn_trust_off", None).await?;
    let client = Client::new();
    let keys = generate_keys();
    let token = authenticate_and_consent(&client, server.base_url.as_str(), &keys).await?;
    let target = generate_keys().public_key_hex();

    for path in [
        format!("/v1/trust/users/{target}"),
        format!("/v1/trust/pull/{target}"),
        format!("/v1/relation/users/{target}"),
        "/v1/relation/neighbors".to_string(),
    ] {
        let response = client
            .get(format!("{}{path}", server.base_url))
            .bearer_auth(token.as_str())
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
    }
    server.shutdown().await
}

#[tokio::test]
async fn viewer_relative_read_requires_authenticated_viewer() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api trust read test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let (trust, _relation) = memory_trust_state();
    let server =
        TestServer::spawn(admin_database_url.as_str(), "cn_trust_auth", Some(trust)).await?;
    let client = Client::new();
    let target = generate_keys().public_key_hex();

    // 未認証の viewer 相対 read（trust / relation）は拒否される。
    for path in [
        format!("/v1/trust/users/{target}"),
        format!("/v1/relation/users/{target}"),
        "/v1/relation/neighbors".to_string(),
    ] {
        let response = client
            .get(format!("{}{path}", server.base_url))
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{path}");
    }

    // 認証済み read の viewer は bearer identity（鍵署名で検証済みの pubkey）に固定される:
    // 「viewer=A」を騙って read する手段が存在しない（なりすまし防止, §6.3）。
    let viewer_keys = generate_keys();
    let token = authenticate_and_consent(&client, server.base_url.as_str(), &viewer_keys).await?;
    let body: serde_json::Value = client
        .get(format!("{}/v1/trust/users/{target}", server.base_url))
        .bearer_auth(token.as_str())
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(
        body["viewer_pubkey"].as_str(),
        Some(viewer_keys.public_key_hex().as_str())
    );
    server.shutdown().await
}

#[tokio::test]
async fn trust_read_returns_components_with_basis_and_ignores_reports() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api trust read test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let (trust, _relation) = memory_trust_state();
    let server =
        TestServer::spawn(admin_database_url.as_str(), "cn_trust_read", Some(trust)).await?;
    let pool = connect_postgres(server.database.database_url.as_str()).await?;
    let client = Client::new();
    let viewer_keys = generate_keys();
    let token = authenticate_and_consent(&client, server.base_url.as_str(), &viewer_keys).await?;
    let target = generate_keys().public_key_hex();

    // CSAM（critical safety, known-hash confirmed）→ 絶対成分。nsfw → 相対成分。
    persist_risk_signal(
        &pool,
        "issuer-node",
        &risk_signal(
            target.as_str(),
            SafetyCategory::Csam,
            Severity::Critical,
            Basis::KnownHashMatch,
            Visibility::Public,
        ),
    )
    .await?;
    persist_risk_signal(
        &pool,
        "issuer-node",
        &risk_signal(
            target.as_str(),
            SafetyCategory::Nsfw,
            Severity::High,
            Basis::ClassifierScore,
            Visibility::Local,
        ),
    )
    .await?;

    let read_trust = |token: String| {
        let client = client.clone();
        let url = format!("{}/v1/trust/users/{target}", server.base_url);
        async move {
            anyhow::Ok(
                client
                    .get(url)
                    .bearer_auth(token)
                    .send()
                    .await?
                    .error_for_status()?
                    .json::<serde_json::Value>()
                    .await?,
            )
        }
    };
    let body = read_trust(token.clone()).await?;

    // 単一絶対スカラーではなく、絶対 / 相対成分 + 合成 + 根拠を返す。
    let absolute = body["absolute"].as_f64().unwrap();
    let relative = body["relative"].as_f64().unwrap();
    let trust_value = body["trust"].as_f64().unwrap();
    assert!(absolute <= -1.0 + 1e-9, "CSAM confirmed で絶対成分は最低値");
    assert!(relative < 0.0);
    assert_eq!(body["w_abs_applied"].as_f64(), Some(2.0));
    assert!((-1.0..=1.0).contains(&trust_value), "最終クランプ");
    assert!(trust_value <= -0.5, "絶対マイナスは相対で薄まらない");
    let basis = body["basis"].as_array().unwrap();
    assert_eq!(basis.len(), 2);
    for entry in basis {
        // 根拠つき advisory: issuer / basis / confidence / visibility を説明できる。
        assert!(entry["issuer_node_id"].is_string());
        assert!(entry["basis"].is_string());
        assert!(entry["visibility"].is_string());
        assert!(entry["contribution"].is_number());
    }

    // 通報（reporter identity を持たない, #370）は何件積まれても trust に影響しない
    // （report-bombing 耐性の構造的保証。trust の入力は verdict / risk signal のみで、
    //  `cn_admin.reports` から trust への経路が存在しない）。intake 経路（HTTP / 管理操作）に
    //  依存しない性質なので、通報の真実源へ直接大量投入して固定する。
    for i in 0..25 {
        insert_community_node_report(
            &pool,
            &NewCommunityNodeReport {
                subject_kind: "profile".to_string(),
                subject_id: target.clone(),
                capability: "community_index".to_string(),
                reason: format!("mass-report-{i}"),
                details: None,
                reporter_contact: None,
            },
        )
        .await?;
    }
    let after = read_trust(token.clone()).await?;
    assert_eq!(after["absolute"], body["absolute"]);
    assert_eq!(after["relative"], body["relative"]);
    assert_eq!(after["trust"], body["trust"]);

    server.shutdown().await
}

#[tokio::test]
async fn cross_node_pull_discloses_only_confirmed_absolute_component() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api trust read test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let (trust, _relation) = memory_trust_state();
    let server =
        TestServer::spawn(admin_database_url.as_str(), "cn_trust_pull", Some(trust)).await?;
    let pool = connect_postgres(server.database.database_url.as_str()).await?;
    let client = Client::new();
    let target = generate_keys().public_key_hex();

    // Public + confirmed（開示される）/ Local + confirmed（返さない）/
    // Public + suspected（返さない）/ 相対成分（返さない）/ SubscribedNodes + confirmed
    // （subscriber 認証時のみ）。
    let public_confirmed = persist_risk_signal(
        &pool,
        "issuer-node",
        &risk_signal(
            target.as_str(),
            SafetyCategory::Csam,
            Severity::Critical,
            Basis::KnownHashMatch,
            Visibility::Public,
        ),
    )
    .await?;
    persist_risk_signal(
        &pool,
        "issuer-node",
        &risk_signal(
            target.as_str(),
            SafetyCategory::Cse,
            Severity::Critical,
            Basis::ProviderVerdict,
            Visibility::Local,
        ),
    )
    .await?;
    persist_risk_signal(
        &pool,
        "issuer-node",
        &risk_signal(
            target.as_str(),
            SafetyCategory::Grooming,
            Severity::High,
            Basis::ClassifierScore,
            Visibility::Public,
        ),
    )
    .await?;
    persist_risk_signal(
        &pool,
        "issuer-node",
        &risk_signal(
            target.as_str(),
            SafetyCategory::Nsfw,
            Severity::High,
            Basis::KnownHashMatch,
            Visibility::Public,
        ),
    )
    .await?;
    let subscribed_confirmed = persist_risk_signal(
        &pool,
        "issuer-node",
        &risk_signal(
            target.as_str(),
            SafetyCategory::Csam,
            Severity::Critical,
            Basis::ProviderVerdict,
            Visibility::SubscribedNodes,
        ),
    )
    .await?;

    let pull_url = format!("{}/v1/trust/pull/{target}", server.base_url);
    let signal_ids = |body: &serde_json::Value| -> Vec<String> {
        body["basis"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["signal_id"].as_str().unwrap_or_default().to_string())
            .collect()
    };

    // 匿名 pull: Public + confirmed + 絶対成分のみ。
    let anonymous: serde_json::Value = client
        .get(pull_url.as_str())
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(signal_ids(&anonymous), vec![public_confirmed.id.clone()]);
    assert!(anonymous.get("relative").is_none(), "相対成分は返さない");
    assert!(anonymous.get("trust").is_none(), "合成値も返さない");

    // subscriber 認証つき pull: SubscribedNodes visibility の confirmed も開示される。
    let subscriber_keys = generate_keys();
    let token =
        authenticate_and_consent(&client, server.base_url.as_str(), &subscriber_keys).await?;
    let subscribed: serde_json::Value = client
        .get(pull_url.as_str())
        .bearer_auth(token.as_str())
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let mut ids = signal_ids(&subscribed);
    ids.sort();
    let mut expected = vec![public_confirmed.id, subscribed_confirmed.id];
    expected.sort();
    assert_eq!(ids, expected);

    server.shutdown().await
}

#[tokio::test]
async fn relation_distance_optout_is_explicit_symmetric_and_reversible() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api trust read test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let (trust, relation) = memory_trust_state();
    let server = TestServer::spawn(admin_database_url.as_str(), "cn_relation", Some(trust)).await?;
    let pool = connect_postgres(server.database.database_url.as_str()).await?;
    let client = Client::new();

    let viewer_keys = generate_keys();
    let target_keys = generate_keys();
    let other_keys = generate_keys();
    let viewer = viewer_keys.public_key_hex();
    let target = target_keys.public_key_hex();
    let other = other_keys.public_key_hex();
    let viewer_token =
        authenticate_and_consent(&client, server.base_url.as_str(), &viewer_keys).await?;
    let target_token =
        authenticate_and_consent(&client, server.base_url.as_str(), &target_keys).await?;

    // target は遠距離、other は閾値以上として relation graph を seedする。
    relation
        .upsert_edge(
            viewer.as_str(),
            target.as_str(),
            &EdgeFeatures::new().with(FEATURE_SHARED_TOPICS, 0.1),
        )
        .await?;
    relation
        .upsert_edge(
            viewer.as_str(),
            other.as_str(),
            &EdgeFeatures::new().with(FEATURE_SHARED_TOPICS, 3.0),
        )
        .await?;

    // pairwise read: 根拠つき proximity が返る。
    let relation_url = format!("{}/v1/relation/users/{target}", server.base_url);
    let body: serde_json::Value = client
        .get(relation_url.as_str())
        .bearer_auth(viewer_token.as_str())
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert!(body["score"].as_f64().unwrap() > 0.0);
    assert!(!body["basis"].as_array().unwrap().is_empty(), "根拠つき");

    // neighbors: proximity 降順で返る。
    let neighbors_url = format!("{}/v1/relation/neighbors", server.base_url);
    let neighbors: serde_json::Value = client
        .get(neighbors_url.as_str())
        .bearer_auth(viewer_token.as_str())
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(
        neighbors["neighbors"].as_array().unwrap().len(),
        2,
        "opt-out 前は両方見える"
    );

    // 未選択時は遠距離でも自動抑制しない。
    let initial_status: serde_json::Value = client
        .get(format!("{}/v1/relation/optout", server.base_url))
        .bearer_auth(target_token.as_str())
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(initial_status["opted_out"].as_bool(), Some(false));
    assert_eq!(initial_status["min_proximity"].as_f64(), Some(0.5));

    // target がdistance opt-outを選ぶ（自分自身のみ・user-controlled）。
    let optout: serde_json::Value = client
        .put(format!("{}/v1/relation/optout", server.base_url))
        .bearer_auth(target_token.as_str())
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(optout["opted_out"].as_bool(), Some(true));
    assert_eq!(optout["pubkey"].as_str(), Some(target.as_str()));
    assert_eq!(optout["min_proximity"].as_f64(), Some(0.5));

    // 遠距離のtargetだけがrelation read / neighborsから消え、近距離のotherは残る。
    let hidden = client
        .get(relation_url.as_str())
        .bearer_auth(viewer_token.as_str())
        .send()
        .await?;
    assert_eq!(hidden.status(), StatusCode::NOT_FOUND);
    let neighbors_after: serde_json::Value = client
        .get(neighbors_url.as_str())
        .bearer_auth(viewer_token.as_str())
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let visible: Vec<&str> = neighbors_after["neighbors"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert_eq!(visible, vec![other.as_str()]);

    // opt-out は trust には影響しない（troll 判定回避の手段にしない）: opt-out した target の
    // trust read は変わらず取得でき、risk signal の寄与も変わらない。
    persist_risk_signal(
        &pool,
        "issuer-node",
        &risk_signal(
            target.as_str(),
            SafetyCategory::Spam,
            Severity::Medium,
            Basis::ClassifierScore,
            Visibility::Local,
        ),
    )
    .await?;
    let trust_body: serde_json::Value = client
        .get(format!("{}/v1/trust/users/{target}", server.base_url))
        .bearer_auth(viewer_token.as_str())
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert!(trust_body["relative"].as_f64().unwrap() < 0.0);

    // 可逆: 解除すれば見え直す（canonical 削除ではない node-local の選択）。
    client
        .delete(format!("{}/v1/relation/optout", server.base_url))
        .bearer_auth(target_token.as_str())
        .send()
        .await?
        .error_for_status()?;
    let restored = client
        .get(relation_url.as_str())
        .bearer_auth(viewer_token.as_str())
        .send()
        .await?;
    assert_eq!(restored.status(), StatusCode::OK);

    // viewer側の選択も同じpairを抑制する（相互・向きに依存しない）。
    client
        .put(format!("{}/v1/relation/optout", server.base_url))
        .bearer_auth(viewer_token.as_str())
        .send()
        .await?
        .error_for_status()?;
    let hidden_by_viewer = client
        .get(relation_url.as_str())
        .bearer_auth(viewer_token.as_str())
        .send()
        .await?;
    assert_eq!(hidden_by_viewer.status(), StatusCode::NOT_FOUND);
    client
        .delete(format!("{}/v1/relation/optout", server.base_url))
        .bearer_auth(viewer_token.as_str())
        .send()
        .await?
        .error_for_status()?;

    // edge の無い相手へのreadは従来どおりrelation未観測の404。
    let stranger = generate_keys().public_key_hex();
    let cross = client
        .get(format!("{}/v1/relation/users/{stranger}", server.base_url))
        .bearer_auth(viewer_token.as_str())
        .send()
        .await?;
    assert_eq!(cross.status(), StatusCode::NOT_FOUND);

    server.shutdown().await
}
