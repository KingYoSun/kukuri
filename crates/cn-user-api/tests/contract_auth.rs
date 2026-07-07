//! auth / admission の contract テスト(WP-H4 で contract.rs から分割)。

mod support;

use anyhow::Result;
use kukuri_cn_core::{
    AdmissionMode, add_allowlist, ban_subscriber, build_auth_envelope_json, issue_invite_code,
    set_admission_mode, unban_subscriber,
};
use kukuri_core::generate_keys;
use reqwest::{Client, StatusCode};
use sqlx::postgres::PgPool;

use support::*;

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

#[tokio::test]
async fn admission_open_mode_admits_new_subscriber() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    // 既定 open mode では invite/allowlist なしの新規 subscriber を従来通り admit する（後方互換）。
    let server = TestServer::spawn(admin_database_url.as_str(), "cn_admission_open").await?;
    let client = Client::new();
    let keys = generate_keys();
    let (access_token, _) = authenticate(&client, &server.base_url, &keys, "peer-a", None).await?;
    assert!(!access_token.is_empty());
    server.shutdown().await
}

#[tokio::test]
async fn admission_invite_mode_admits_with_valid_code() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server = TestServer::spawn(admin_database_url.as_str(), "cn_admission_invite_ok").await?;
    let client = Client::new();
    let pool = PgPool::connect(server.database.database_url.as_str()).await?;
    set_admission_mode(&pool, AdmissionMode::Invite).await?;
    let code = issue_invite_code(&pool, Some("test"), None, None).await?;

    let keys = generate_keys();
    let (access_token, _) = authenticate_with_invite(
        &client,
        &server.base_url,
        &keys,
        "peer-a",
        None,
        Some(&code),
    )
    .await?;
    assert!(!access_token.is_empty());
    server.shutdown().await
}

#[tokio::test]
async fn admission_invite_mode_requires_code() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server =
        TestServer::spawn(admin_database_url.as_str(), "cn_admission_invite_required").await?;
    let client = Client::new();
    let pool = PgPool::connect(server.database.database_url.as_str()).await?;
    set_admission_mode(&pool, AdmissionMode::Invite).await?;

    let keys = generate_keys();
    let (status, body) = raw_auth_verify(&client, &server.base_url, &keys, None).await?;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "INVITE_REQUIRED");
    server.shutdown().await
}

#[tokio::test]
async fn admission_invite_mode_rejects_invalid_code() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server =
        TestServer::spawn(admin_database_url.as_str(), "cn_admission_invite_invalid").await?;
    let client = Client::new();
    let pool = PgPool::connect(server.database.database_url.as_str()).await?;
    set_admission_mode(&pool, AdmissionMode::Invite).await?;

    let keys = generate_keys();
    let (status, body) =
        raw_auth_verify(&client, &server.base_url, &keys, Some("not-a-real-code")).await?;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "INVITE_INVALID");
    server.shutdown().await
}

#[tokio::test]
async fn admission_invite_mode_rejects_expired_code() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server =
        TestServer::spawn(admin_database_url.as_str(), "cn_admission_invite_expired").await?;
    let client = Client::new();
    let pool = PgPool::connect(server.database.database_url.as_str()).await?;
    set_admission_mode(&pool, AdmissionMode::Invite).await?;
    let past = chrono::Utc::now() - chrono::Duration::seconds(60);
    let code = issue_invite_code(&pool, None, None, Some(past)).await?;

    let keys = generate_keys();
    let (status, body) = raw_auth_verify(&client, &server.base_url, &keys, Some(&code)).await?;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "INVITE_EXPIRED");
    server.shutdown().await
}

#[tokio::test]
async fn admission_invite_mode_rejects_exhausted_code() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server =
        TestServer::spawn(admin_database_url.as_str(), "cn_admission_invite_exhausted").await?;
    let client = Client::new();
    let pool = PgPool::connect(server.database.database_url.as_str()).await?;
    set_admission_mode(&pool, AdmissionMode::Invite).await?;
    // max_uses=1 の単回コードを 1 人が使い切り、2 人目を拒否する。
    let code = issue_invite_code(&pool, None, Some(1), None).await?;

    let keys_a = generate_keys();
    let (token_a, _) = authenticate_with_invite(
        &client,
        &server.base_url,
        &keys_a,
        "peer-a",
        None,
        Some(&code),
    )
    .await?;
    assert!(!token_a.is_empty());

    let keys_b = generate_keys();
    let (status, body) = raw_auth_verify(&client, &server.base_url, &keys_b, Some(&code)).await?;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "INVITE_EXHAUSTED");
    server.shutdown().await
}

#[tokio::test]
async fn admission_invite_mode_rejects_revoked_code() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server =
        TestServer::spawn(admin_database_url.as_str(), "cn_admission_invite_revoked").await?;
    let client = Client::new();
    let pool = PgPool::connect(server.database.database_url.as_str()).await?;
    set_admission_mode(&pool, AdmissionMode::Invite).await?;
    let code = issue_invite_code(&pool, None, None, None).await?;
    assert!(kukuri_cn_core::revoke_invite_code(&pool, code.as_str()).await?);

    let keys = generate_keys();
    let (status, body) = raw_auth_verify(&client, &server.base_url, &keys, Some(&code)).await?;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "INVITE_REVOKED");
    server.shutdown().await
}

#[tokio::test]
async fn admission_invite_mode_allowlisted_pubkey_bypasses_code() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server =
        TestServer::spawn(admin_database_url.as_str(), "cn_admission_invite_bypass").await?;
    let client = Client::new();
    let pool = PgPool::connect(server.database.database_url.as_str()).await?;
    set_admission_mode(&pool, AdmissionMode::Invite).await?;

    let keys = generate_keys();
    add_allowlist(&pool, keys.public_key_hex().as_str(), Some("vip")).await?;
    // allowlist 該当はコード不要で admit される。
    let (access_token, _) = authenticate(&client, &server.base_url, &keys, "peer-a", None).await?;
    assert!(!access_token.is_empty());
    server.shutdown().await
}

#[tokio::test]
async fn admission_whitelist_mode_admits_allowlisted() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server =
        TestServer::spawn(admin_database_url.as_str(), "cn_admission_whitelist_ok").await?;
    let client = Client::new();
    let pool = PgPool::connect(server.database.database_url.as_str()).await?;
    set_admission_mode(&pool, AdmissionMode::Whitelist).await?;

    let keys = generate_keys();
    add_allowlist(&pool, keys.public_key_hex().as_str(), None).await?;
    let (access_token, _) = authenticate(&client, &server.base_url, &keys, "peer-a", None).await?;
    assert!(!access_token.is_empty());
    server.shutdown().await
}

#[tokio::test]
async fn admission_whitelist_mode_rejects_unlisted() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server =
        TestServer::spawn(admin_database_url.as_str(), "cn_admission_whitelist_no").await?;
    let client = Client::new();
    let pool = PgPool::connect(server.database.database_url.as_str()).await?;
    set_admission_mode(&pool, AdmissionMode::Whitelist).await?;

    let keys = generate_keys();
    let (status, body) = raw_auth_verify(&client, &server.base_url, &keys, None).await?;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "NOT_ALLOWLISTED");
    server.shutdown().await
}

#[tokio::test]
async fn admission_pre_ban_rejects_verify() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server = TestServer::spawn(admin_database_url.as_str(), "cn_admission_pre_ban").await?;
    let client = Client::new();
    let pool = PgPool::connect(server.database.database_url.as_str()).await?;
    // 未登録 pubkey を事前 ban すると、open mode でも verify を拒否する。
    let keys = generate_keys();
    ban_subscriber(&pool, keys.public_key_hex().as_str()).await?;

    let (status, body) = raw_auth_verify(&client, &server.base_url, &keys, None).await?;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "BANNED");
    server.shutdown().await
}

#[tokio::test]
async fn admission_existing_active_subscriber_passes_in_invite_mode() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server =
        TestServer::spawn(admin_database_url.as_str(), "cn_admission_existing_active").await?;
    let client = Client::new();
    let pool = PgPool::connect(server.database.database_url.as_str()).await?;

    // open mode で先に active subscriber を作る。
    let keys = generate_keys();
    let (token_open, _) = authenticate(&client, &server.base_url, &keys, "peer-a", None).await?;
    assert!(!token_open.is_empty());

    // mode を invite に変えても、既存 active subscriber はコードなしで再認証を通す。
    set_admission_mode(&pool, AdmissionMode::Invite).await?;
    let (token_after, _) = authenticate(&client, &server.base_url, &keys, "peer-a", None).await?;
    assert!(!token_after.is_empty());
    server.shutdown().await
}

#[tokio::test]
async fn admission_banning_active_subscriber_revokes_existing_token() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server = TestServer::spawn(admin_database_url.as_str(), "cn_admission_ban_revokes").await?;
    let client = Client::new();
    let pool = PgPool::connect(server.database.database_url.as_str()).await?;

    let keys = generate_keys();
    let (access_token, _) = authenticate(&client, &server.base_url, &keys, "peer-a", None).await?;
    accept_required_consents(&client, &server.base_url, access_token.as_str()).await?;

    // 認証済みトークンで bootstrap を取得できることを確認する。
    let ok = client
        .get(format!("{}/v1/bootstrap/nodes", server.base_url))
        .bearer_auth(access_token.as_str())
        .send()
        .await?;
    assert_eq!(ok.status(), StatusCode::OK);

    // active subscriber を ban すると、既存トークンでの API 呼び出しが即時 401 になる。
    ban_subscriber(&pool, keys.public_key_hex().as_str()).await?;
    let revoked = client
        .get(format!("{}/v1/bootstrap/nodes", server.base_url))
        .bearer_auth(access_token.as_str())
        .send()
        .await?;
    assert_eq!(revoked.status(), StatusCode::UNAUTHORIZED);
    server.shutdown().await
}

#[tokio::test]
async fn admission_unban_of_pre_banned_pubkey_still_requires_invite() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server =
        TestServer::spawn(admin_database_url.as_str(), "cn_admission_unban_preban").await?;
    let client = Client::new();
    let pool = PgPool::connect(server.database.database_url.as_str()).await?;
    set_admission_mode(&pool, AdmissionMode::Invite).await?;

    // 未参加の pubkey を事前 ban → unban する。unban は admission を迂回してはならない。
    let keys = generate_keys();
    ban_subscriber(&pool, keys.public_key_hex().as_str()).await?;
    assert!(unban_subscriber(&pool, keys.public_key_hex().as_str()).await?);

    // unban しただけでは member ではないので、invite mode では依然コードが必要。
    let (status, body) = raw_auth_verify(&client, &server.base_url, &keys, None).await?;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "INVITE_REQUIRED");
    server.shutdown().await
}

#[tokio::test]
async fn admission_unban_of_existing_member_restores_access() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server =
        TestServer::spawn(admin_database_url.as_str(), "cn_admission_unban_member").await?;
    let client = Client::new();
    let pool = PgPool::connect(server.database.database_url.as_str()).await?;

    // open mode で参加して member になる。
    let keys = generate_keys();
    let (token, _) = authenticate(&client, &server.base_url, &keys, "peer-a", None).await?;
    assert!(!token.is_empty());

    // ban → unban した既存 member は、invite mode でもコードなしで再認証を通る。
    ban_subscriber(&pool, keys.public_key_hex().as_str()).await?;
    assert!(unban_subscriber(&pool, keys.public_key_hex().as_str()).await?);
    set_admission_mode(&pool, AdmissionMode::Invite).await?;
    let (token_after, _) = authenticate(&client, &server.base_url, &keys, "peer-a", None).await?;
    assert!(!token_after.is_empty());
    server.shutdown().await
}
