//! indexing request endpoint (#413 / ADR 0025 §2.2 / §6.3) の contract test。
//!
//! `POST /v1/indexing/requests` は認証済み + consent 済み user の indexing request を受ける。
//! - public topic request は保存され pending になる（index を保証しない）。
//! - private channel request は channel secret（capability）提示が必須で、それ自体が権限の証明。
//!   secret 無しは 400、channel secret 暗号鍵未設定 node は 404。
//!
//! Postgres + Redis を要するため `KUKURI_CN_RUN_INTEGRATION_TESTS=1` で gate する。

use std::net::SocketAddr;

use anyhow::{Context, Result};
use chrono::Utc;
use kukuri_cn_core::{
    JwtConfig, TestDatabase, connect_postgres, readiness_context_fingerprint,
    record_readiness_activation,
};
use kukuri_cn_operator::READINESS_CHECK_IDS;
use kukuri_cn_protocol::build_auth_envelope_json;
use kukuri_cn_user_api::{UserApiConfig, app_router, build_state};
use kukuri_core::{KukuriKeys, generate_keys};
use reqwest::{Client, StatusCode};

mod support;
use support::{
    accept_required_consents, integration_test_admin_database_url,
    integration_test_rendezvous_redis_url,
};

const TEST_CHANNEL_SECRET_KEY: &str = "cn-user-api-indexing-test-channel-secret-key-0123456789";

struct TestServer {
    task: tokio::task::JoinHandle<()>,
    database: TestDatabase,
    base_url: String,
}

/// 索引参照の提供状態(#713 の受付門)。申請の受付は Activated のときだけ許される。
#[derive(Clone, Copy, PartialEq)]
enum IndexGate {
    /// 索引参照が未構成(このノードは索引を提供しない)。
    NotConfigured,
    /// 構成済みだが有効化(準備完了記録)が無い・失効している。
    Inactive,
    /// 構成済みかつ有効化済み。
    Activated,
}

impl TestServer {
    async fn spawn(
        admin_database_url: &str,
        prefix: &str,
        channel_secret_key: Option<String>,
    ) -> Result<Self> {
        Self::spawn_with_index_gate(
            admin_database_url,
            prefix,
            channel_secret_key,
            IndexGate::Activated,
        )
        .await
    }

    async fn spawn_with_index_gate(
        admin_database_url: &str,
        prefix: &str,
        channel_secret_key: Option<String>,
        gate: IndexGate,
    ) -> Result<Self> {
        let database = TestDatabase::create(admin_database_url, prefix).await?;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .context("failed to bind test indexing listener")?;
        let addr = listener.local_addr()?;
        let base_url = format!("http://{addr}");
        let state = build_state(&UserApiConfig {
            bind_addr: addr,
            database_url: database.database_url.clone(),
            rendezvous_redis_url: integration_test_rendezvous_redis_url(),
            rendezvous_key_prefix: format!("cn:test:{prefix}"),
            base_url: base_url.clone(),
            public_base_url: base_url.clone(),
            connectivity_urls: vec!["http://127.0.0.1:13340".to_string()],
            jwt_config: JwtConfig::new("kukuri-cn-tests", "test-secret", 3600),
            operator_config_path: None,
            channel_secret_key,
            legal_data_key: None,
            index_query_enabled: gate != IndexGate::NotConfigured,
            trust_read_enabled: false,
            relation_distance_optout_min_proximity: (gate != IndexGate::NotConfigured)
                .then_some(0.5),
            deployment_revision: "test-deployment-v1".to_string(),
            readiness_activation_max_age_secs: 3600,
            expected_issuer_node_id: None,
        })
        .await?;
        if gate == IndexGate::Activated {
            let pool = connect_postgres(database.database_url.as_str()).await?;
            record_readiness_activation(
                &pool,
                Utc::now(),
                "public-node",
                &READINESS_CHECK_IDS,
                &readiness_context_fingerprint("public-node", "test-deployment-v1", b""),
                &serde_json::json!([]),
            )
            .await?;
        }
        let app = app_router(state);
        let task = tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .expect("indexing server");
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
    accept_required_consents(client, base_url, verify.access_token.as_str()).await?;
    Ok(verify.access_token)
}

#[tokio::test]
async fn indexing_request_requires_auth() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api indexing test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server = TestServer::spawn(
        admin_database_url.as_str(),
        "cn_indexing_auth",
        Some(TEST_CHANNEL_SECRET_KEY.to_string()),
    )
    .await?;
    let client = Client::new();

    let unauthenticated = client
        .post(format!("{}/v1/indexing/requests", server.base_url))
        .json(&serde_json::json!({ "kind": "public_topic", "target_id": "rust" }))
        .send()
        .await?;
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

    server.shutdown().await
}

#[tokio::test]
async fn public_topic_indexing_request_is_accepted_pending() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api indexing test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server = TestServer::spawn(
        admin_database_url.as_str(),
        "cn_indexing_public",
        Some(TEST_CHANNEL_SECRET_KEY.to_string()),
    )
    .await?;
    let client = Client::new();
    let keys = generate_keys();
    let token = authenticate_and_consent(&client, &server.base_url, &keys).await?;

    let accepted = client
        .post(format!("{}/v1/indexing/requests", server.base_url))
        .bearer_auth(token.as_str())
        .json(&serde_json::json!({ "kind": "public_topic", "target_id": "rust" }))
        .send()
        .await?;
    assert_eq!(accepted.status(), StatusCode::OK);
    let body = accepted.json::<serde_json::Value>().await?;
    assert_eq!(body["status"], "pending");
    assert!(body["request_id"].as_str().is_some_and(|id| !id.is_empty()));

    let repeated = client
        .post(format!("{}/v1/indexing/requests", server.base_url))
        .bearer_auth(token.as_str())
        .json(&serde_json::json!({ "kind": "public_topic", "target_id": "rust" }))
        .send()
        .await?;
    assert_eq!(repeated.status(), StatusCode::OK);
    let repeated_body = repeated.json::<serde_json::Value>().await?;
    assert_eq!(repeated_body["request_id"], body["request_id"]);
    assert_eq!(repeated_body["status"], body["status"]);

    server.shutdown().await
}

#[tokio::test]
async fn private_channel_request_without_secret_is_rejected() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api indexing test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server = TestServer::spawn(
        admin_database_url.as_str(),
        "cn_indexing_no_secret",
        Some(TEST_CHANNEL_SECRET_KEY.to_string()),
    )
    .await?;
    let client = Client::new();
    let keys = generate_keys();
    let token = authenticate_and_consent(&client, &server.base_url, &keys).await?;

    let rejected = client
        .post(format!("{}/v1/indexing/requests", server.base_url))
        .bearer_auth(token.as_str())
        .json(&serde_json::json!({
            "kind": "private_channel",
            "target_id": "secret-room"
        }))
        .send()
        .await?;
    assert_eq!(rejected.status(), StatusCode::BAD_REQUEST);
    let body = rejected.json::<serde_json::Value>().await?;
    assert_eq!(body["code"], "CHANNEL_SECRET_REQUIRED");

    server.shutdown().await
}

#[tokio::test]
async fn private_channel_request_with_secret_is_accepted() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api indexing test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server = TestServer::spawn(
        admin_database_url.as_str(),
        "cn_indexing_with_secret",
        Some(TEST_CHANNEL_SECRET_KEY.to_string()),
    )
    .await?;
    let client = Client::new();
    let keys = generate_keys();
    let token = authenticate_and_consent(&client, &server.base_url, &keys).await?;

    let secret_hex = hex::encode([5u8; 32]);
    let accepted = client
        .post(format!("{}/v1/indexing/requests", server.base_url))
        .bearer_auth(token.as_str())
        .json(&serde_json::json!({
            "kind": "private_channel",
            "target_id": "secret-room",
            "channel_secret_hex": secret_hex,
        }))
        .send()
        .await?;
    assert_eq!(accepted.status(), StatusCode::OK);
    let body = accepted.json::<serde_json::Value>().await?;
    assert_eq!(body["status"], "pending");

    server.shutdown().await
}

#[tokio::test]
async fn private_channel_request_rejects_capability_takeover() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api indexing test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server = TestServer::spawn(
        admin_database_url.as_str(),
        "cn_indexing_takeover",
        Some(TEST_CHANNEL_SECRET_KEY.to_string()),
    )
    .await?;
    let client = Client::new();

    // requester A が capability を登録する。
    let keys_a = generate_keys();
    let token_a = authenticate_and_consent(&client, &server.base_url, &keys_a).await?;
    let secret_a = hex::encode([1u8; 32]);
    let accepted = client
        .post(format!("{}/v1/indexing/requests", server.base_url))
        .bearer_auth(token_a.as_str())
        .json(&serde_json::json!({
            "kind": "private_channel",
            "target_id": "secret-room",
            "channel_secret_hex": secret_a,
        }))
        .send()
        .await?;
    assert_eq!(accepted.status(), StatusCode::OK);

    // requester B が別 secret で同じ channel を乗っ取ろうとすると 409 で拒否される。
    let keys_b = generate_keys();
    let token_b = authenticate_and_consent(&client, &server.base_url, &keys_b).await?;
    let secret_b = hex::encode([2u8; 32]);
    let conflict = client
        .post(format!("{}/v1/indexing/requests", server.base_url))
        .bearer_auth(token_b.as_str())
        .json(&serde_json::json!({
            "kind": "private_channel",
            "target_id": "secret-room",
            "channel_secret_hex": secret_b,
        }))
        .send()
        .await?;
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
    let body = conflict.json::<serde_json::Value>().await?;
    assert_eq!(body["code"], "CHANNEL_SECRET_CONFLICT");

    server.shutdown().await
}

#[tokio::test]
async fn private_channel_request_rejected_when_encryption_key_unset() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api indexing test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    // channel secret 暗号鍵を設定しない node は private channel indexing を受け付けない。
    let server =
        TestServer::spawn(admin_database_url.as_str(), "cn_indexing_key_unset", None).await?;
    let client = Client::new();
    let keys = generate_keys();
    let token = authenticate_and_consent(&client, &server.base_url, &keys).await?;

    let secret_hex = hex::encode([5u8; 32]);
    let rejected = client
        .post(format!("{}/v1/indexing/requests", server.base_url))
        .bearer_auth(token.as_str())
        .json(&serde_json::json!({
            "kind": "private_channel",
            "target_id": "secret-room",
            "channel_secret_hex": secret_hex,
        }))
        .send()
        .await?;
    assert_eq!(rejected.status(), StatusCode::NOT_FOUND);
    let body = rejected.json::<serde_json::Value>().await?;
    assert_eq!(body["code"], "CHANNEL_INDEXING_NOT_CONFIGURED");

    server.shutdown().await
}

#[tokio::test]
async fn invalid_kind_is_rejected() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api indexing test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server = TestServer::spawn(
        admin_database_url.as_str(),
        "cn_indexing_bad_kind",
        Some(TEST_CHANNEL_SECRET_KEY.to_string()),
    )
    .await?;
    let client = Client::new();
    let keys = generate_keys();
    let token = authenticate_and_consent(&client, &server.base_url, &keys).await?;

    let rejected = client
        .post(format!("{}/v1/indexing/requests", server.base_url))
        .bearer_auth(token.as_str())
        .json(&serde_json::json!({ "kind": "nonsense", "target_id": "rust" }))
        .send()
        .await?;
    assert_eq!(rejected.status(), StatusCode::BAD_REQUEST);
    let body = rejected.json::<serde_json::Value>().await?;
    assert_eq!(body["code"], "INVALID_INDEXING_REQUEST");

    server.shutdown().await
}

/// 拒否後にデータベースへ何も残っていないことを確認する(#713: 秘密値の不保存)。
async fn assert_nothing_stored(server: &TestServer) -> Result<()> {
    let pool = connect_postgres(server.database.database_url.as_str()).await?;
    let requests: i64 = sqlx::query_scalar("SELECT count(*) FROM cn_index.indexing_requests")
        .fetch_one(&pool)
        .await?;
    let secrets: i64 = sqlx::query_scalar("SELECT count(*) FROM cn_index.channel_secrets")
        .fetch_one(&pool)
        .await?;
    assert_eq!(requests, 0, "申請行が保存されてはならない");
    assert_eq!(secrets, 0, "channel secret が保存されてはならない");
    Ok(())
}

#[tokio::test]
async fn indexing_request_is_rejected_when_index_query_not_configured() -> Result<()> {
    // #713: 索引参照を提供しないノードは申請を受理・保存しない(サーバ側の門)。
    // 未提供の面は read 面と同じく認証前でも 404 で存在しない扱いにする。
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api indexing test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server = TestServer::spawn_with_index_gate(
        admin_database_url.as_str(),
        "cn_indexing_gate_off",
        Some(TEST_CHANNEL_SECRET_KEY.to_string()),
        IndexGate::NotConfigured,
    )
    .await?;
    let client = Client::new();

    let unauthenticated = client
        .post(format!("{}/v1/indexing/requests", server.base_url))
        .json(&serde_json::json!({ "kind": "public_topic", "target_id": "rust" }))
        .send()
        .await?;
    assert_eq!(unauthenticated.status(), StatusCode::NOT_FOUND);
    let body = unauthenticated.json::<serde_json::Value>().await?;
    assert_eq!(body["code"], "INDEXING_REQUEST_NOT_CONFIGURED");

    // 認証・同意済みで秘密値を提示しても、受理も保存もされない。
    let keys = generate_keys();
    let token = authenticate_and_consent(&client, &server.base_url, &keys).await?;
    let rejected = client
        .post(format!("{}/v1/indexing/requests", server.base_url))
        .bearer_auth(token.as_str())
        .json(&serde_json::json!({
            "kind": "private_channel",
            "target_id": "secret-room",
            "channel_secret_hex": hex::encode([7u8; 32]),
        }))
        .send()
        .await?;
    assert_eq!(rejected.status(), StatusCode::NOT_FOUND);
    let body = rejected.json::<serde_json::Value>().await?;
    assert_eq!(body["code"], "INDEXING_REQUEST_NOT_CONFIGURED");
    assert_nothing_stored(&server).await?;

    server.shutdown().await
}

#[tokio::test]
async fn indexing_request_is_rejected_when_activation_is_stale() -> Result<()> {
    // #713: 構成済みでも有効化(準備完了記録)が無い・失効しているノードは申請を受け付けない。
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api indexing test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server = TestServer::spawn_with_index_gate(
        admin_database_url.as_str(),
        "cn_indexing_gate_stale",
        Some(TEST_CHANNEL_SECRET_KEY.to_string()),
        IndexGate::Inactive,
    )
    .await?;
    let client = Client::new();
    let keys = generate_keys();
    let token = authenticate_and_consent(&client, &server.base_url, &keys).await?;

    let rejected = client
        .post(format!("{}/v1/indexing/requests", server.base_url))
        .bearer_auth(token.as_str())
        .json(&serde_json::json!({
            "kind": "private_channel",
            "target_id": "secret-room",
            "channel_secret_hex": hex::encode([7u8; 32]),
        }))
        .send()
        .await?;
    assert_eq!(rejected.status(), StatusCode::NOT_FOUND);
    let body = rejected.json::<serde_json::Value>().await?;
    assert_eq!(body["code"], "INDEXING_REQUEST_NOT_ACTIVATED");
    assert_nothing_stored(&server).await?;

    server.shutdown().await
}
