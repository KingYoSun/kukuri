use std::net::SocketAddr;

use anyhow::{Context, Result};
use kukuri_cn_core::{JwtConfig, TestDatabase, USER_API_BEARER_CHALLENGE, build_auth_event_json};
use kukuri_cn_user_api::{UserApiConfig, app_router, build_state};
use nostr_sdk::prelude::Keys;
use reqwest::{Client, StatusCode};

const DEFAULT_ADMIN_DATABASE_URL: &str = "postgres://cn:cn_password@127.0.0.1:55432/cn";

struct TestServer {
    task: tokio::task::JoinHandle<()>,
    database: TestDatabase,
    base_url: String,
}

impl TestServer {
    async fn spawn(admin_database_url: &str, prefix: &str) -> Result<Self> {
        let database = TestDatabase::create(admin_database_url, prefix).await?;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .context("failed to bind test user-api listener")?;
        let addr = listener.local_addr()?;
        let base_url = format!("http://{addr}");
        let state = build_state(&UserApiConfig {
            bind_addr: addr,
            database_url: database.database_url.clone(),
            base_url: base_url.clone(),
            public_base_url: base_url.clone(),
            relay_ws_url: "ws://127.0.0.1:18081/relay".to_string(),
            iroh_relay_urls: vec!["http://127.0.0.1:13340".to_string()],
            jwt_config: JwtConfig::new("kukuri-cn-tests", "test-secret", 3600),
        })
        .await?;
        let app = app_router(state);
        let task = tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .expect("user-api server");
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

async fn authenticate(
    client: &Client,
    base_url: &str,
    keys: &Keys,
) -> Result<(String, serde_json::Value)> {
    let pubkey = keys.public_key().to_hex();
    let challenge = client
        .post(format!("{base_url}/v1/auth/challenge"))
        .json(&serde_json::json!({ "pubkey": pubkey }))
        .send()
        .await?
        .error_for_status()?
        .json::<kukuri_cn_core::AuthChallengeResponse>()
        .await?;
    let auth_event_json = build_auth_event_json(keys, challenge.challenge.as_str(), base_url)?;
    let verify = client
        .post(format!("{base_url}/v1/auth/verify"))
        .json(&serde_json::json!({ "auth_event_json": auth_event_json.clone() }))
        .send()
        .await?
        .error_for_status()?
        .json::<kukuri_cn_core::AuthVerifyResponse>()
        .await?;
    Ok((verify.access_token, auth_event_json))
}

#[tokio::test]
async fn bootstrap_requires_bearer_then_consents() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server = TestServer::spawn(admin_database_url.as_str(), "cn_user_api_contract").await?;
    let client = Client::new();

    let unauthenticated = client
        .get(format!("{}/v1/bootstrap/nodes", server.base_url))
        .send()
        .await?;
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        unauthenticated
            .headers()
            .get("www-authenticate")
            .and_then(|value| value.to_str().ok()),
        Some(USER_API_BEARER_CHALLENGE)
    );
    let unauthenticated_body = unauthenticated.json::<serde_json::Value>().await?;
    assert_eq!(unauthenticated_body["code"], "AUTH_REQUIRED");

    let keys = Keys::generate();
    let (access_token, auth_event_json) = authenticate(&client, &server.base_url, &keys).await?;

    let reused = client
        .post(format!("{}/v1/auth/verify", server.base_url))
        .json(&serde_json::json!({ "auth_event_json": auth_event_json }))
        .send()
        .await?;
    assert_eq!(reused.status(), StatusCode::UNAUTHORIZED);

    let consent_required = client
        .get(format!("{}/v1/bootstrap/nodes", server.base_url))
        .bearer_auth(access_token.as_str())
        .send()
        .await?;
    assert_eq!(consent_required.status(), StatusCode::FORBIDDEN);
    let consent_required_body = consent_required.json::<serde_json::Value>().await?;
    assert_eq!(consent_required_body["code"], "CONSENT_REQUIRED");

    let consent_status = client
        .get(format!("{}/v1/consents/status", server.base_url))
        .bearer_auth(access_token.as_str())
        .send()
        .await?
        .error_for_status()?
        .json::<kukuri_cn_core::CommunityNodeConsentStatus>()
        .await?;
    assert!(!consent_status.all_required_accepted);
    assert!(
        consent_status
            .items
            .iter()
            .all(|item| item.accepted_at.is_none())
    );

    let accepted = client
        .post(format!("{}/v1/consents", server.base_url))
        .bearer_auth(access_token.as_str())
        .json(&serde_json::json!({ "policy_slugs": [] }))
        .send()
        .await?
        .error_for_status()?
        .json::<kukuri_cn_core::CommunityNodeConsentStatus>()
        .await?;
    assert!(accepted.all_required_accepted);

    let bootstrap = client
        .get(format!("{}/v1/bootstrap/nodes", server.base_url))
        .bearer_auth(access_token.as_str())
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;
    assert_eq!(bootstrap["nodes"][0]["base_url"], server.base_url);
    assert_eq!(
        bootstrap["nodes"][0]["resolved_urls"]["relay_ws_url"],
        "ws://127.0.0.1:18081/relay"
    );
    assert_eq!(
        bootstrap["nodes"][0]["resolved_urls"]["iroh_relay_urls"][0],
        "http://127.0.0.1:13340"
    );

    server.shutdown().await
}

#[tokio::test]
async fn auth_verify_rejects_wrong_relay_tag() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server = TestServer::spawn(admin_database_url.as_str(), "cn_user_api_relay_tag").await?;
    let client = Client::new();
    let keys = Keys::generate();
    let challenge = client
        .post(format!("{}/v1/auth/challenge", server.base_url))
        .json(&serde_json::json!({ "pubkey": keys.public_key().to_hex() }))
        .send()
        .await?
        .error_for_status()?
        .json::<kukuri_cn_core::AuthChallengeResponse>()
        .await?;
    let auth_event_json =
        build_auth_event_json(&keys, challenge.challenge.as_str(), "http://wrong.example")?;
    let response = client
        .post(format!("{}/v1/auth/verify", server.base_url))
        .json(&serde_json::json!({ "auth_event_json": auth_event_json }))
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = response.json::<serde_json::Value>().await?;
    assert_eq!(body["code"], "AUTH_FAILED");

    server.shutdown().await
}
