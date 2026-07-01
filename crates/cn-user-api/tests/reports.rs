//! 通報受信エンドポイント (#370) の contract test。
//!
//! `POST /v1/report` は report_endpoint capability を有効化した node でのみ受理し、authority scope
//! 内の対象に対する通報を保存する。受理（200 + reference_id）・必須欠落の拒否（400）・capability 無効
//! node での拒否（404）を再現する。Postgres を要するため `KUKURI_CN_RUN_INTEGRATION_TESTS=1` で gate する。

use std::net::SocketAddr;

use anyhow::{Context, Result};
use kukuri_cn_core::{JwtConfig, TestDatabase};
use kukuri_cn_user_api::{UserApiConfig, app_router, build_state};
use reqwest::{Client, StatusCode};

const DEFAULT_ADMIN_DATABASE_URL: &str = "postgres://cn:cn_password@127.0.0.1:15432/cn";
const DEFAULT_RENDEZVOUS_REDIS_URL: &str = "redis://127.0.0.1:16379/";

/// report_endpoint capability を有効化した operator config。
const REPORT_ENABLED_YAML: &str = r#"server:
  domain: example-kukuri.net
  operator_name: Example Operator
  country: JP
features:
  report_endpoint: true
acknowledge_planned_capabilities: true
"#;

/// report_endpoint capability を有効化しない operator config。
const REPORT_DISABLED_YAML: &str = r#"server:
  domain: example-kukuri.net
  operator_name: Example Operator
  country: JP
features:
  iroh_relay: true
"#;

struct TestServer {
    task: tokio::task::JoinHandle<()>,
    database: TestDatabase,
    base_url: String,
    // operator config の一時ファイルを test 期間中保持する。
    _operator_config: tempfile::NamedTempFile,
}

impl TestServer {
    async fn spawn(admin_database_url: &str, prefix: &str, operator_yaml: &str) -> Result<Self> {
        use std::io::Write;

        let database = TestDatabase::create(admin_database_url, prefix).await?;
        let mut operator_config = tempfile::Builder::new()
            .suffix(".yaml")
            .tempfile()
            .context("create temp operator config")?;
        operator_config
            .write_all(operator_yaml.as_bytes())
            .context("write temp operator config")?;
        operator_config.flush().ok();

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .context("failed to bind test report listener")?;
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
            operator_config_path: Some(operator_config.path().to_path_buf()),
            channel_secret_key: None,
        })
        .await?;
        let app = app_router(state);
        let task = tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .expect("report server");
        });
        Ok(Self {
            task,
            database,
            base_url,
            _operator_config: operator_config,
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

#[tokio::test]
async fn report_endpoint_accepts_stores_and_validates() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api report test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server = TestServer::spawn(
        admin_database_url.as_str(),
        "cn_user_api_report",
        REPORT_ENABLED_YAML,
    )
    .await?;
    let client = Client::new();

    // 受理：authority scope 内の対象への通報を保存し reference_id を返す。
    let accepted = client
        .post(format!("{}/v1/report", server.base_url))
        .json(&serde_json::json!({
            "subject_kind": "post",
            "subject_id": "object-123",
            "capability": "community_index",
            "reason": "spam",
            "details": "repeated spam",
        }))
        .send()
        .await?;
    assert_eq!(accepted.status(), StatusCode::OK);
    let body = accepted.json::<serde_json::Value>().await?;
    let reference_id = body["reference_id"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert!(!reference_id.is_empty(), "reference_id must be returned");

    // 匿名（reporter_contact 無し）でも受理する。
    let anonymous = client
        .post(format!("{}/v1/report", server.base_url))
        .json(&serde_json::json!({
            "subject_kind": "profile",
            "subject_id": "pubkey-abc",
            "capability": "moderation",
            "reason": "harassment",
        }))
        .send()
        .await?;
    assert_eq!(anonymous.status(), StatusCode::OK);

    // 必須欠落は 400。
    let invalid = client
        .post(format!("{}/v1/report", server.base_url))
        .json(&serde_json::json!({
            "subject_kind": "post",
            "subject_id": "",
            "capability": "community_index",
            "reason": "spam",
        }))
        .send()
        .await?;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    let invalid_body = invalid.json::<serde_json::Value>().await?;
    assert_eq!(invalid_body["code"], "INVALID_REPORT");

    server.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn report_endpoint_rejects_when_capability_disabled() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api report test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server = TestServer::spawn(
        admin_database_url.as_str(),
        "cn_user_api_report_off",
        REPORT_DISABLED_YAML,
    )
    .await?;
    let client = Client::new();

    // report_endpoint capability を有効化していない node は通報を受け付けない（404）。
    let rejected = client
        .post(format!("{}/v1/report", server.base_url))
        .json(&serde_json::json!({
            "subject_kind": "post",
            "subject_id": "object-123",
            "capability": "community_index",
            "reason": "spam",
        }))
        .send()
        .await?;
    assert_eq!(rejected.status(), StatusCode::NOT_FOUND);
    let body = rejected.json::<serde_json::Value>().await?;
    assert_eq!(body["code"], "REPORT_NOT_CONFIGURED");

    server.shutdown().await?;
    Ok(())
}
