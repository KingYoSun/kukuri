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
use kukuri_cn_core::{JwtConfig, TestDatabase, build_auth_envelope_json};
use kukuri_cn_user_api::{UserApiConfig, app_router, build_state};
use kukuri_core::{KukuriKeys, generate_keys};
use reqwest::{Client, StatusCode};

const DEFAULT_ADMIN_DATABASE_URL: &str = "postgres://cn:cn_password@127.0.0.1:15432/cn";
const DEFAULT_RENDEZVOUS_REDIS_URL: &str = "redis://127.0.0.1:16379/";
const TEST_CHANNEL_SECRET_KEY: &str = "cn-user-api-indexing-test-channel-secret-key-0123456789";

struct TestServer {
    task: tokio::task::JoinHandle<()>,
    database: TestDatabase,
    base_url: String,
}

impl TestServer {
    async fn spawn(
        admin_database_url: &str,
        prefix: &str,
        channel_secret_key: Option<String>,
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
        })
        .await?;
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

fn integration_test_rendezvous_redis_url() -> String {
    std::env::var("COMMUNITY_NODE_RENDEZVOUS_REDIS_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_RENDEZVOUS_REDIS_URL.to_string())
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
        .json::<kukuri_cn_core::AuthChallengeResponse>()
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
        .json::<kukuri_cn_core::AuthVerifyResponse>()
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
