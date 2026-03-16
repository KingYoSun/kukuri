use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use kukuri_cn_core::{
    AuthMode, AuthRolloutConfig, CommunityNodeBootstrapNode, CommunityNodeResolvedUrls,
    TestDatabase, build_auth_event_json, connect_postgres, store_auth_rollout,
    upsert_bootstrap_node,
};
use kukuri_cn_relay::{RelayConfig, app_router, build_state};
use nostr_sdk::prelude::Keys;
use reqwest::Client;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const DEFAULT_ADMIN_DATABASE_URL: &str = "postgres://cn:cn_password@127.0.0.1:55432/cn";

struct TestServer {
    task: tokio::task::JoinHandle<()>,
    database: TestDatabase,
    base_url: String,
    public_base_url: String,
    relay_ws_url: String,
}

impl TestServer {
    async fn spawn(
        admin_database_url: &str,
        prefix: &str,
        rollout: Option<AuthRolloutConfig>,
    ) -> Result<Self> {
        let database = TestDatabase::create(admin_database_url, prefix).await?;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .context("failed to bind test relay listener")?;
        let addr = listener.local_addr()?;
        let relay_ws_url = format!("ws://{addr}/relay");
        let public_base_url = format!("http://127.0.0.1:{}", addr.port() + 1000);
        if let Some(rollout) = rollout {
            let pool = connect_postgres(database.database_url.as_str()).await?;
            kukuri_cn_core::initialize_database(&pool).await?;
            store_auth_rollout(&pool, kukuri_cn_core::RELAY_SERVICE_NAME, &rollout).await?;
        }
        let state = build_state(&RelayConfig {
            bind_addr: addr,
            database_url: database.database_url.clone(),
            base_url: public_base_url.clone(),
            public_base_url: public_base_url.clone(),
            relay_ws_url: relay_ws_url.clone(),
            iroh_relay_urls: vec!["http://127.0.0.1:13340".to_string()],
        })
        .await?;
        let app = app_router(state);
        let task = tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .expect("relay server");
        });
        Ok(Self {
            task,
            database,
            base_url: format!("http://{addr}"),
            public_base_url,
            relay_ws_url,
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

async fn next_text_message(
    socket: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> Result<serde_json::Value> {
    let message = tokio::time::timeout(Duration::from_secs(15), socket.next())
        .await
        .context("websocket receive timeout")?
        .context("websocket closed")??;
    match message {
        Message::Text(text) => Ok(serde_json::from_str(text.as_str())?),
        other => anyhow::bail!("unexpected websocket message: {other:?}"),
    }
}

#[tokio::test]
async fn p2p_info_returns_explicit_urls_and_bootstrap_nodes() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-relay integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server = TestServer::spawn(admin_database_url.as_str(), "cn_relay_p2p_info", None).await?;
    let pool = connect_postgres(server.database.database_url.as_str()).await?;
    upsert_bootstrap_node(
        &pool,
        &CommunityNodeBootstrapNode {
            base_url: "https://bootstrap.example".to_string(),
            resolved_urls: CommunityNodeResolvedUrls::new(
                "https://public.bootstrap.example",
                "wss://public.bootstrap.example/relay",
                vec!["https://relay.bootstrap.example".to_string()],
            )?,
        },
    )
    .await?;

    let body = Client::new()
        .get(format!("{}/v1/p2p/info", server.base_url))
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;
    assert_eq!(body["public_base_url"], server.public_base_url);
    assert_eq!(body["relay_ws_url"], server.relay_ws_url);
    assert_eq!(body["iroh_relay_urls"][0], "http://127.0.0.1:13340");
    assert_eq!(body["relay_urls"][0], "http://127.0.0.1:13340");
    assert!(
        body["bootstrap_nodes"]
            .as_array()
            .expect("bootstrap nodes")
            .iter()
            .any(|value| value
                == &serde_json::Value::String("https://bootstrap.example".to_string()))
    );
    assert!(
        body["bootstrap_nodes"]
            .as_array()
            .expect("bootstrap nodes")
            .iter()
            .any(|value| value == &serde_json::Value::String(server.public_base_url.clone()))
    );

    server.shutdown().await
}

#[tokio::test]
async fn auth_rollout_requires_auth_and_accepts_valid_auth_event() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-relay integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server = TestServer::spawn(
        admin_database_url.as_str(),
        "cn_relay_auth_required",
        Some(AuthRolloutConfig {
            mode: AuthMode::Required,
            enforce_at: Some(chrono::Utc::now().timestamp()),
            grace_seconds: 5,
            ws_auth_timeout_seconds: 5,
        }),
    )
    .await?;
    let (mut socket, _) = connect_async(server.relay_ws_url.as_str()).await?;

    let auth_request = next_text_message(&mut socket).await?;
    assert_eq!(auth_request[0], "AUTH");
    let challenge = auth_request[1].as_str().expect("challenge");

    let keys = Keys::generate();
    let auth_event_json = build_auth_event_json(&keys, challenge, server.public_base_url.as_str())?;
    socket
        .send(Message::Text(
            serde_json::json!(["AUTH", auth_event_json])
                .to_string()
                .into(),
        ))
        .await?;
    let authenticated = next_text_message(&mut socket).await?;
    assert_eq!(authenticated[0], "NOTICE");
    assert_eq!(authenticated[1], "authenticated");

    socket
        .send(Message::Text(
            serde_json::json!(["REQ", "sub-1", { "#t": ["kukuri:topic:relay"] }])
                .to_string()
                .into(),
        ))
        .await?;
    let eose = next_text_message(&mut socket).await?;
    assert_eq!(eose[0], "EOSE");
    assert_eq!(eose[1], "sub-1");

    server.shutdown().await
}

#[tokio::test]
async fn auth_rollout_disconnects_existing_connection_after_grace_period() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-relay integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server = TestServer::spawn(
        admin_database_url.as_str(),
        "cn_relay_auth_grace",
        Some(AuthRolloutConfig {
            mode: AuthMode::Required,
            enforce_at: Some(chrono::Utc::now().timestamp() + 5),
            grace_seconds: 1,
            ws_auth_timeout_seconds: 10,
        }),
    )
    .await?;
    let (mut socket, _) = connect_async(server.relay_ws_url.as_str()).await?;

    let auth_request = next_text_message(&mut socket).await?;
    assert_eq!(auth_request[0], "AUTH");
    let notice = next_text_message(&mut socket).await?;
    assert_eq!(notice[0], "NOTICE");
    assert!(
        notice[1]
            .as_str()
            .expect("notice body")
            .contains("grace deadline reached")
    );
    let disconnected = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match socket.next().await {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => {
                    return Ok::<bool, anyhow::Error>(true);
                }
                Some(Ok(Message::Text(text))) => {
                    let value: serde_json::Value = serde_json::from_str(text.as_str())?;
                    if value.get(0) == Some(&serde_json::Value::String("NOTICE".to_string())) {
                        continue;
                    }
                    return Ok(false);
                }
                Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => continue,
                Some(Ok(other)) => {
                    anyhow::bail!("unexpected websocket message after grace: {other:?}")
                }
            }
        }
    })
    .await
    .context("websocket close timeout")??;
    assert!(disconnected);

    server.shutdown().await
}
