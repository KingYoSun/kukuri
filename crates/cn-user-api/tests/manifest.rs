//! public manifest endpoint (#356) の contract test。
//!
//! manifest endpoint は DB を必要としないため、`manifest_routes` を単独でサーブして検証する。

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use kukuri_cn_operator::{build_manifest, load_and_validate};
use kukuri_cn_user_api::manifest_routes;
use reqwest::{Client, StatusCode};

async fn spawn_manifest_server(
    yaml: Option<&str>,
) -> Result<(String, tokio::task::JoinHandle<()>)> {
    let manifest = match yaml {
        Some(yaml) => {
            let resolved = load_and_validate(yaml).context("config must validate")?;
            Some(Arc::new(build_manifest(&resolved)))
        }
        None => None,
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .context("bind manifest test listener")?;
    let addr = listener.local_addr()?;
    let base_url = format!("http://{addr}");
    let app = manifest_routes(manifest);
    let task = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .expect("manifest server");
    });
    Ok((base_url, task))
}

const SAMPLE_YAML: &str = r#"server:
  domain: example-kukuri.net
  operator_name: Example Operator
  country: JP
profile: relay-enabled
features:
  cloudflare_proxy: true
acknowledge_planned_capabilities: true
"#;

#[tokio::test]
async fn manifest_endpoint_serves_unauthenticated_json() -> Result<()> {
    let (base_url, task) = spawn_manifest_server(Some(SAMPLE_YAML)).await?;
    let client = Client::new();

    for path in [
        "/.well-known/kukuri/community-node.json",
        "/v1/node/manifest",
    ] {
        let resp = client.get(format!("{base_url}{path}")).send().await?;
        assert_eq!(resp.status(), StatusCode::OK, "path {path}");

        // client が cache できる。
        let cache_control = resp
            .headers()
            .get(reqwest::header::CACHE_CONTROL)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();
        assert!(cache_control.contains("max-age"), "cache header on {path}");

        let body: serde_json::Value = resp.json().await?;
        // authority scope / P2P boundary / capability scope を含む。
        assert_eq!(body["p2p_boundary"]["network_wide_authority"], false);
        assert!(
            body["authority_scope"]["does_not_apply_to"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v == "user_identity")
        );
        assert!(body["capability_scope"]["available_enabled"].is_array());
        // policy URLs / abuse contact を含む。
        assert_eq!(body["terms_url"], "https://example-kukuri.net/terms");
        assert!(body["abuse_contact"].as_str().unwrap().contains("@"));
        // private secret を含まない。
        assert!(body.get("jwt_secret").is_none());
        assert!(body.get("database_url").is_none());
    }

    task.abort();
    Ok(())
}

#[tokio::test]
async fn manifest_endpoint_returns_404_when_not_configured() -> Result<()> {
    let (base_url, task) = spawn_manifest_server(None).await?;
    let client = Client::new();
    let resp = client
        .get(format!("{base_url}/v1/node/manifest"))
        .send()
        .await?;
    // 設定されていない場合は 404。client は default node へ fallback せず別経路を使う。
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["error"], "manifest_not_configured");
    task.abort();
    Ok(())
}
