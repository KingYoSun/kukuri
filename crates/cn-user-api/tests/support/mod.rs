//! contract 系テストの共有ヘルパ(WP-H4 で contract.rs から抽出)。
//! 各テストバイナリは使う部分だけ参照するため、未使用警告は許容する。
#![allow(dead_code)]

use std::net::SocketAddr;

use anyhow::{Context, Result};
use kukuri_cn_core::{JwtConfig, TestDatabase};
use kukuri_cn_protocol::build_auth_envelope_json;
use kukuri_cn_user_api::{UserApiConfig, app_router, build_state};
use kukuri_core::KukuriKeys;
use redis::AsyncCommands;
use reqwest::{Client, StatusCode};

pub const DEFAULT_ADMIN_DATABASE_URL: &str = "postgres://cn:cn_password@127.0.0.1:15432/cn";
pub const DEFAULT_RENDEZVOUS_REDIS_URL: &str = "redis://127.0.0.1:16379/";

pub struct TestServer {
    pub task: tokio::task::JoinHandle<()>,
    pub database: TestDatabase,
    pub base_url: String,
    pub rendezvous_redis_url: String,
    pub rendezvous_key_prefix: String,
}

impl TestServer {
    pub async fn spawn(admin_database_url: &str, prefix: &str) -> Result<Self> {
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
            channel_secret_key: None,
            index_query_enabled: false,
            trust_read_enabled: false,
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

    pub async fn shutdown(self) -> Result<()> {
        self.task.abort();
        self.database.cleanup().await
    }
}

pub async fn accept_required_consents(
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
        .json::<kukuri_cn_protocol::CommunityNodeConsentStatus>()
        .await?;
    assert!(accepted.all_required_accepted);
    Ok(())
}

pub async fn redis_keys(redis_url: &str, pattern: &str) -> Result<Vec<String>> {
    let client = redis::Client::open(redis_url)?;
    let mut connection = client.get_multiplexed_async_connection().await?;
    let mut keys: Vec<String> = connection.keys(pattern).await?;
    keys.sort();
    Ok(keys)
}

pub fn integration_test_admin_database_url() -> Option<String> {
    kukuri_test_support::gated_env_url(
        "KUKURI_CN_RUN_INTEGRATION_TESTS",
        "COMMUNITY_NODE_DATABASE_URL",
        DEFAULT_ADMIN_DATABASE_URL,
    )
}

pub fn integration_test_rendezvous_redis_url() -> String {
    std::env::var("COMMUNITY_NODE_RENDEZVOUS_REDIS_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_RENDEZVOUS_REDIS_URL.to_string())
}

pub async fn authenticate(
    client: &Client,
    base_url: &str,
    keys: &KukuriKeys,
    endpoint_id: &str,
    addr_hint: Option<&str>,
) -> Result<(String, serde_json::Value)> {
    authenticate_with_invite(client, base_url, keys, endpoint_id, addr_hint, None).await
}

pub async fn authenticate_with_invite(
    client: &Client,
    base_url: &str,
    keys: &KukuriKeys,
    endpoint_id: &str,
    addr_hint: Option<&str>,
    invite_code: Option<&str>,
) -> Result<(String, serde_json::Value)> {
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
            "auth_envelope_json": auth_envelope_json.clone(),
            "endpoint_id": endpoint_id,
            "addr_hint": addr_hint,
            "invite_code": invite_code,
        }))
        .send()
        .await?
        .error_for_status()?
        .json::<kukuri_cn_protocol::AuthVerifyResponse>()
        .await?;
    Ok((verify.access_token, auth_envelope_json))
}

/// auth/verify を生で叩き、HTTP status とボディ JSON を返す（拒否ケースの検証用）。
pub async fn raw_auth_verify(
    client: &Client,
    base_url: &str,
    keys: &KukuriKeys,
    invite_code: Option<&str>,
) -> Result<(StatusCode, serde_json::Value)> {
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
    let response = client
        .post(format!("{base_url}/v1/auth/verify"))
        .json(&serde_json::json!({
            "auth_envelope_json": auth_envelope_json,
            "invite_code": invite_code,
        }))
        .send()
        .await?;
    let status = response.status();
    let body = response.json::<serde_json::Value>().await?;
    Ok((status, body))
}
