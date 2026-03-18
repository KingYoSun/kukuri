use std::net::SocketAddr;

use anyhow::{Context, Result};
use kukuri_cn_core::{
    JwtConfig, TestDatabase, USER_API_BEARER_CHALLENGE, build_auth_envelope_json,
};
use kukuri_cn_user_api::{UserApiConfig, app_router, build_state};
use kukuri_core::{KukuriKeys, generate_keys};
use reqwest::{Client, StatusCode};
use sqlx::postgres::PgPool;

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
            connectivity_urls: vec!["http://127.0.0.1:13340".to_string()],
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
    keys: &KukuriKeys,
    endpoint_id: &str,
) -> Result<(String, serde_json::Value)> {
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
            "auth_envelope_json": auth_envelope_json.clone(),
            "endpoint_id": endpoint_id,
        }))
        .send()
        .await?
        .error_for_status()?
        .json::<kukuri_cn_core::AuthVerifyResponse>()
        .await?;
    Ok((verify.access_token, auth_envelope_json))
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

    let keys = generate_keys();
    let (access_token, auth_envelope_json) =
        authenticate(&client, &server.base_url, &keys, "peer-a").await?;

    let reused = client
        .post(format!("{}/v1/auth/verify", server.base_url))
        .json(&serde_json::json!({ "auth_envelope_json": auth_envelope_json }))
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
        bootstrap["nodes"][0]["resolved_urls"]["connectivity_urls"][0],
        "http://127.0.0.1:13340"
    );
    assert_eq!(
        bootstrap["nodes"][0]["resolved_urls"]["seed_peers"]
            .as_array()
            .map(Vec::len),
        Some(0)
    );

    server.shutdown().await
}

#[tokio::test]
async fn bootstrap_exposes_other_registered_seed_peers() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server = TestServer::spawn(admin_database_url.as_str(), "cn_user_api_seed_peers").await?;
    let client = Client::new();

    let keys_a = generate_keys();
    let keys_b = generate_keys();
    let (access_token_a, _) = authenticate(&client, &server.base_url, &keys_a, "peer-a").await?;
    let (access_token_b, _) = authenticate(&client, &server.base_url, &keys_b, "peer-b").await?;

    for access_token in [&access_token_a, &access_token_b] {
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
    }

    let bootstrap_a = client
        .get(format!("{}/v1/bootstrap/nodes", server.base_url))
        .bearer_auth(access_token_a.as_str())
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;
    assert_eq!(
        bootstrap_a["nodes"][0]["resolved_urls"]["seed_peers"][0]["endpoint_id"],
        "peer-b"
    );

    let bootstrap_b = client
        .get(format!("{}/v1/bootstrap/nodes", server.base_url))
        .bearer_auth(access_token_b.as_str())
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;
    assert_eq!(
        bootstrap_b["nodes"][0]["resolved_urls"]["seed_peers"][0]["endpoint_id"],
        "peer-a"
    );

    server.shutdown().await
}

#[tokio::test]
async fn bootstrap_exposes_other_endpoints_for_same_subscriber() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server =
        TestServer::spawn(admin_database_url.as_str(), "cn_user_api_same_subscriber").await?;
    let client = Client::new();

    let keys = generate_keys();
    let (access_token_a1, _) = authenticate(&client, &server.base_url, &keys, "peer-a-1").await?;
    let (access_token_a2, _) = authenticate(&client, &server.base_url, &keys, "peer-a-2").await?;

    let accepted = client
        .post(format!("{}/v1/consents", server.base_url))
        .bearer_auth(access_token_a1.as_str())
        .json(&serde_json::json!({ "policy_slugs": [] }))
        .send()
        .await?
        .error_for_status()?
        .json::<kukuri_cn_core::CommunityNodeConsentStatus>()
        .await?;
    assert!(accepted.all_required_accepted);

    let bootstrap_a1 = client
        .get(format!("{}/v1/bootstrap/nodes", server.base_url))
        .bearer_auth(access_token_a1.as_str())
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;
    let seed_peers_a1 = bootstrap_a1["nodes"][0]["resolved_urls"]["seed_peers"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(seed_peers_a1.len(), 1);
    assert_eq!(seed_peers_a1[0]["endpoint_id"], "peer-a-2");

    let bootstrap_a2 = client
        .get(format!("{}/v1/bootstrap/nodes", server.base_url))
        .bearer_auth(access_token_a2.as_str())
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;
    let seed_peers_a2 = bootstrap_a2["nodes"][0]["resolved_urls"]["seed_peers"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(seed_peers_a2.len(), 1);
    assert_eq!(seed_peers_a2[0]["endpoint_id"], "peer-a-1");

    server.shutdown().await
}

#[tokio::test]
async fn bootstrap_filters_expired_peer_registrations_and_heartbeat_restores_them() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server = TestServer::spawn(
        admin_database_url.as_str(),
        "cn_user_api_peer_registration_ttl",
    )
    .await?;
    let client = Client::new();
    let pool = PgPool::connect(server.database.database_url.as_str()).await?;

    let keys_a = generate_keys();
    let keys_b = generate_keys();
    let (token_a_initial, _) = authenticate(&client, &server.base_url, &keys_a, "peer-a-1").await?;
    let (token_a, _) = authenticate(&client, &server.base_url, &keys_a, "peer-a-2").await?;
    let (token_b, _) = authenticate(&client, &server.base_url, &keys_b, "peer-b").await?;

    for access_token in [&token_a, &token_b] {
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
    }

    sqlx::query(
        "UPDATE cn_bootstrap.peer_registrations
         SET expires_at = NOW() - INTERVAL '1 second'
         WHERE subscriber_pubkey = $1
           AND endpoint_id = 'peer-a-1'",
    )
    .bind(keys_a.public_key_hex())
    .execute(&pool)
    .await?;

    let bootstrap_before = client
        .get(format!("{}/v1/bootstrap/nodes", server.base_url))
        .bearer_auth(token_b.as_str())
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;
    let seed_peers_before = bootstrap_before["nodes"][0]["resolved_urls"]["seed_peers"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(seed_peers_before.len(), 1);
    assert_eq!(seed_peers_before[0]["endpoint_id"], "peer-a-2");

    client
        .post(format!("{}/v1/bootstrap/heartbeat", server.base_url))
        .bearer_auth(token_a_initial.as_str())
        .json(&serde_json::json!({ "endpoint_id": "peer-a-1" }))
        .send()
        .await?
        .error_for_status()?;

    let bootstrap_after = client
        .get(format!("{}/v1/bootstrap/nodes", server.base_url))
        .bearer_auth(token_b.as_str())
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;
    let seed_peers_after = bootstrap_after["nodes"][0]["resolved_urls"]["seed_peers"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(seed_peers_after.len(), 2);
    let endpoint_ids = seed_peers_after
        .iter()
        .filter_map(|peer| peer["endpoint_id"].as_str())
        .collect::<Vec<_>>();
    assert!(endpoint_ids.contains(&"peer-a-1"));
    assert!(endpoint_ids.contains(&"peer-a-2"));

    server.shutdown().await
}

#[tokio::test]
async fn auth_verify_rejects_capability_url_mismatch() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server =
        TestServer::spawn(admin_database_url.as_str(), "cn_user_api_capability_url").await?;
    let client = Client::new();
    let keys = generate_keys();
    let challenge = client
        .post(format!("{}/v1/auth/challenge", server.base_url))
        .json(&serde_json::json!({ "pubkey": keys.public_key_hex() }))
        .send()
        .await?
        .error_for_status()?
        .json::<kukuri_cn_core::AuthChallengeResponse>()
        .await?;
    let auth_envelope_json =
        build_auth_envelope_json(&keys, challenge.challenge.as_str(), "http://wrong.example")?;
    let response = client
        .post(format!("{}/v1/auth/verify", server.base_url))
        .json(&serde_json::json!({ "auth_envelope_json": auth_envelope_json }))
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = response.json::<serde_json::Value>().await?;
    assert_eq!(body["code"], "AUTH_FAILED");

    server.shutdown().await
}
