use std::net::SocketAddr;

use anyhow::Result;
use axum::Router;
use axum::routing::get;
use kukuri_cn_user_api::{RateLimitConfig, apply_rate_limit};
use reqwest::{Client, StatusCode};

async fn spawn_router(config: RateLimitConfig) -> Result<String> {
    let app = apply_rate_limit(
        Router::new().route("/healthz", get(|| async { "ok" })),
        &config,
    )?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .expect("rate limit test server");
    });
    Ok(format!("http://{addr}"))
}

#[tokio::test]
async fn rate_limit_rejects_requests_beyond_burst() -> Result<()> {
    let base_url = spawn_router(RateLimitConfig {
        enabled: true,
        per_second: 1,
        burst: 2,
        trust_forwarded_for: false,
    })
    .await?;
    let client = Client::new();

    let mut statuses = Vec::new();
    for _ in 0..10 {
        let status = client
            .get(format!("{base_url}/healthz"))
            .send()
            .await?
            .status();
        statuses.push(status);
    }

    assert!(
        statuses.contains(&StatusCode::OK),
        "expected the burst allowance to let some requests through: {statuses:?}"
    );
    assert!(
        statuses.contains(&StatusCode::TOO_MANY_REQUESTS),
        "expected requests beyond the burst to be rate limited: {statuses:?}"
    );
    Ok(())
}

#[tokio::test]
async fn rate_limit_disabled_passes_all_requests() -> Result<()> {
    let base_url = spawn_router(RateLimitConfig {
        enabled: false,
        per_second: 1,
        burst: 1,
        trust_forwarded_for: false,
    })
    .await?;
    let client = Client::new();

    for _ in 0..10 {
        let status = client
            .get(format!("{base_url}/healthz"))
            .send()
            .await?
            .status();
        assert_eq!(
            status,
            StatusCode::OK,
            "disabled rate limit must not throttle"
        );
    }
    Ok(())
}
