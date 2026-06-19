use std::net::SocketAddr;

use anyhow::{Context, Result};
use kukuri_cn_core::{
    JwtConfig, TestDatabase, USER_API_BEARER_CHALLENGE, build_auth_envelope_json,
};
use kukuri_cn_user_api::{UserApiConfig, app_router, build_state};
use kukuri_core::{
    KukuriKeys, TopicId, generate_keys, private_topic_rendezvous_key_hex_secret,
    public_topic_rendezvous_key,
};
use redis::AsyncCommands;
use reqwest::{Client, StatusCode};
use sqlx::postgres::PgPool;

const DEFAULT_ADMIN_DATABASE_URL: &str = "postgres://cn:cn_password@127.0.0.1:55432/cn";
const DEFAULT_RENDEZVOUS_REDIS_URL: &str = "redis://127.0.0.1:56379/";

struct TestServer {
    task: tokio::task::JoinHandle<()>,
    database: TestDatabase,
    base_url: String,
    rendezvous_redis_url: String,
    rendezvous_key_prefix: String,
}

impl TestServer {
    async fn spawn(admin_database_url: &str, prefix: &str) -> Result<Self> {
        let rendezvous_redis_url = integration_test_rendezvous_redis_url();
        let rendezvous_key_prefix = format!("cn:test:{prefix}");
        let database = TestDatabase::create(admin_database_url, prefix).await?;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .context("failed to bind test user-api listener")?;
        let addr = listener.local_addr()?;
        let base_url = format!("http://{addr}");
        let state = build_state(&UserApiConfig {
            bind_addr: addr,
            database_url: database.database_url.clone(),
            rendezvous_redis_url: rendezvous_redis_url.clone(),
            rendezvous_key_prefix: rendezvous_key_prefix.clone(),
            base_url: base_url.clone(),
            public_base_url: base_url.clone(),
            connectivity_urls: vec!["http://127.0.0.1:13340".to_string()],
            jwt_config: JwtConfig::new("kukuri-cn-tests", "test-secret", 3600),
            operator_config_path: None,
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
            rendezvous_redis_url,
            rendezvous_key_prefix,
        })
    }

    async fn shutdown(self) -> Result<()> {
        self.task.abort();
        self.database.cleanup().await
    }
}

async fn accept_required_consents(
    client: &Client,
    base_url: &str,
    access_token: &str,
) -> Result<()> {
    let accepted = client
        .post(format!("{base_url}/v1/consents"))
        .bearer_auth(access_token)
        .json(&serde_json::json!({ "policy_slugs": [] }))
        .send()
        .await?
        .error_for_status()?
        .json::<kukuri_cn_core::CommunityNodeConsentStatus>()
        .await?;
    assert!(accepted.all_required_accepted);
    Ok(())
}

async fn redis_keys(redis_url: &str, pattern: &str) -> Result<Vec<String>> {
    let client = redis::Client::open(redis_url)?;
    let mut connection = client.get_multiplexed_async_connection().await?;
    let mut keys: Vec<String> = connection.keys(pattern).await?;
    keys.sort();
    Ok(keys)
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

async fn authenticate(
    client: &Client,
    base_url: &str,
    keys: &KukuriKeys,
    endpoint_id: &str,
    addr_hint: Option<&str>,
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
            "addr_hint": addr_hint,
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
        authenticate(&client, &server.base_url, &keys, "peer-a", None).await?;

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
    let (access_token_a, _) = authenticate(
        &client,
        &server.base_url,
        &keys_a,
        "peer-a",
        Some("127.0.0.1:44001"),
    )
    .await?;
    let (access_token_b, _) = authenticate(
        &client,
        &server.base_url,
        &keys_b,
        "peer-b",
        Some("127.0.0.1:44002"),
    )
    .await?;

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
    assert_eq!(
        bootstrap_a["nodes"][0]["resolved_urls"]["seed_peers"][0]["addr_hint"],
        "127.0.0.1:44002"
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
    assert_eq!(
        bootstrap_b["nodes"][0]["resolved_urls"]["seed_peers"][0]["addr_hint"],
        "127.0.0.1:44001"
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
    let (access_token_a1, _) =
        authenticate(&client, &server.base_url, &keys, "peer-a-1", None).await?;
    let (access_token_a2, _) =
        authenticate(&client, &server.base_url, &keys, "peer-a-2", None).await?;

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
    let (token_a_initial, _) = authenticate(
        &client,
        &server.base_url,
        &keys_a,
        "peer-a-1",
        Some("127.0.0.1:45001"),
    )
    .await?;
    let (token_a, _) = authenticate(
        &client,
        &server.base_url,
        &keys_a,
        "peer-a-2",
        Some("127.0.0.1:45002"),
    )
    .await?;
    let (token_b, _) = authenticate(
        &client,
        &server.base_url,
        &keys_b,
        "peer-b",
        Some("127.0.0.1:45003"),
    )
    .await?;

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
        .json(&serde_json::json!({
            "endpoint_id": "peer-a-1",
            "addr_hint": "127.0.0.1:45011",
        }))
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
    let peer_a1 = seed_peers_after
        .iter()
        .find(|peer| peer["endpoint_id"] == "peer-a-1")
        .context("peer-a-1 restored seed peer missing")?;
    assert_eq!(peer_a1["addr_hint"], "127.0.0.1:45011");

    server.shutdown().await
}

#[tokio::test]
async fn topic_rendezvous_batch_heartbeat_returns_fresh_peer_candidates() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server = TestServer::spawn(admin_database_url.as_str(), "cn_user_api_rendezvous").await?;
    let client = Client::new();

    let keys_a = generate_keys();
    let keys_b = generate_keys();
    let (token_a, _) = authenticate(&client, &server.base_url, &keys_a, "peer-a", None).await?;
    let (token_b, _) = authenticate(
        &client,
        &server.base_url,
        &keys_b,
        "peer-b",
        Some("127.0.0.1:46002"),
    )
    .await?;
    accept_required_consents(&client, &server.base_url, token_a.as_str()).await?;
    accept_required_consents(&client, &server.base_url, token_b.as_str()).await?;

    let raw_topic = TopicId::new("kukuri:topic:rendezvous-public");
    let topic_key = public_topic_rendezvous_key(&raw_topic);

    let first = client
        .post(format!(
            "{}/v1/rendezvous/topics/heartbeat",
            server.base_url
        ))
        .bearer_auth(token_a.as_str())
        .json(&serde_json::json!({
            "endpoint_id": "peer-a",
            "addr_hint": null,
            "joins": [topic_key],
            "refreshes": [],
            "leaves": []
        }))
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;
    assert_eq!(
        first["topics"][0]["peers"].as_array().map(Vec::len),
        Some(0)
    );

    let second = client
        .post(format!(
            "{}/v1/rendezvous/topics/heartbeat",
            server.base_url
        ))
        .bearer_auth(token_b.as_str())
        .json(&serde_json::json!({
            "endpoint_id": "peer-b",
            "addr_hint": "127.0.0.1:46002",
            "joins": [topic_key],
            "refreshes": [],
            "leaves": []
        }))
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;
    assert_eq!(second["expires_in_seconds"], 45);
    assert_eq!(second["topics"][0]["topic_key"], topic_key);
    assert_eq!(second["topics"][0]["peers"][0]["endpoint_id"], "peer-a");
    assert_eq!(
        second["topics"][0]["peers"][0]["addr_hint"],
        serde_json::Value::Null
    );
    assert_eq!(
        second["topics"][0]["peers"][0]["relay_urls"][0],
        "http://127.0.0.1:13340"
    );

    let refreshed = client
        .post(format!(
            "{}/v1/rendezvous/topics/heartbeat",
            server.base_url
        ))
        .bearer_auth(token_a.as_str())
        .json(&serde_json::json!({
            "endpoint_id": "peer-a",
            "addr_hint": null,
            "joins": [],
            "refreshes": [topic_key],
            "leaves": []
        }))
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;
    assert_eq!(refreshed["topics"][0]["peers"][0]["endpoint_id"], "peer-b");
    assert_eq!(
        refreshed["topics"][0]["peers"][0]["addr_hint"],
        "127.0.0.1:46002"
    );

    client
        .post(format!(
            "{}/v1/rendezvous/topics/heartbeat",
            server.base_url
        ))
        .bearer_auth(token_b.as_str())
        .json(&serde_json::json!({
            "endpoint_id": "peer-b",
            "addr_hint": "127.0.0.1:46002",
            "joins": [],
            "refreshes": [],
            "leaves": [topic_key]
        }))
        .send()
        .await?
        .error_for_status()?;

    let after_leave = client
        .post(format!(
            "{}/v1/rendezvous/topics/heartbeat",
            server.base_url
        ))
        .bearer_auth(token_a.as_str())
        .json(&serde_json::json!({
            "endpoint_id": "peer-a",
            "addr_hint": null,
            "joins": [],
            "refreshes": [topic_key],
            "leaves": []
        }))
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;
    assert_eq!(
        after_leave["topics"][0]["peers"].as_array().map(Vec::len),
        Some(0)
    );

    server.shutdown().await
}

#[tokio::test]
async fn topic_rendezvous_keys_do_not_expose_raw_topic_ids() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server = TestServer::spawn(
        admin_database_url.as_str(),
        "cn_user_api_rendezvous_privacy",
    )
    .await?;
    let client = Client::new();

    let keys = generate_keys();
    let (token, _) = authenticate(&client, &server.base_url, &keys, "peer-a", None).await?;
    accept_required_consents(&client, &server.base_url, token.as_str()).await?;

    let raw_public_topic = TopicId::new("kukuri:topic:dictionary-visible");
    let raw_private_topic = TopicId::new("kukuri:private:super-secret-channel");
    let public_key = public_topic_rendezvous_key(&raw_public_topic);
    let private_key = private_topic_rendezvous_key_hex_secret(
        "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
        &raw_private_topic,
    )?;

    assert!(!public_key.contains(raw_public_topic.as_str()));
    assert!(!private_key.contains(raw_private_topic.as_str()));
    assert_ne!(public_key, private_key);

    client
        .post(format!(
            "{}/v1/rendezvous/topics/heartbeat",
            server.base_url
        ))
        .bearer_auth(token.as_str())
        .json(&serde_json::json!({
            "endpoint_id": "peer-a",
            "addr_hint": null,
            "joins": [public_key, private_key],
            "refreshes": [],
            "leaves": []
        }))
        .send()
        .await?
        .error_for_status()?;

    let keys = redis_keys(
        server.rendezvous_redis_url.as_str(),
        format!("{}*", server.rendezvous_key_prefix).as_str(),
    )
    .await?;
    assert!(!keys.is_empty());
    let serialized_keys = keys.join("\n");
    assert!(!serialized_keys.contains(raw_public_topic.as_str()));
    assert!(!serialized_keys.contains(raw_private_topic.as_str()));

    server.shutdown().await
}

#[tokio::test]
async fn auth_challenge_prunes_expired_challenges() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server = TestServer::spawn(admin_database_url.as_str(), "cn_user_api_challenge_gc").await?;
    let client = Client::new();
    let pool = PgPool::connect(server.database.database_url.as_str()).await?;

    let keys = generate_keys();
    // Issue a challenge and force it past its expiry without consuming it.
    let stale = client
        .post(format!("{}/v1/auth/challenge", server.base_url))
        .json(&serde_json::json!({ "pubkey": keys.public_key_hex() }))
        .send()
        .await?
        .error_for_status()?
        .json::<kukuri_cn_core::AuthChallengeResponse>()
        .await?;
    sqlx::query(
        "UPDATE cn_auth.auth_challenges
         SET expires_at = NOW() - INTERVAL '1 second'
         WHERE challenge = $1",
    )
    .bind(stale.challenge.as_str())
    .execute(&pool)
    .await?;

    let expired_before: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cn_auth.auth_challenges WHERE expires_at <= NOW()",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(expired_before, 1);

    // A fresh challenge request must opportunistically prune the expired row.
    client
        .post(format!("{}/v1/auth/challenge", server.base_url))
        .json(&serde_json::json!({ "pubkey": generate_keys().public_key_hex() }))
        .send()
        .await?
        .error_for_status()?;

    let expired_after: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cn_auth.auth_challenges WHERE expires_at <= NOW()",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(expired_after, 0);
    let stale_remaining: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cn_auth.auth_challenges WHERE challenge = $1")
            .bind(stale.challenge.as_str())
            .fetch_one(&pool)
            .await?;
    assert_eq!(stale_remaining, 0);

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
