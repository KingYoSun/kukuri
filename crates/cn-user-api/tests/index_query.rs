//! #404 ユーザー向け index query endpoint（search / discovery / recommendation）の contract test。
//!
//! `GET /v1/index/search` / `/v1/index/discovery` / `/v1/index/recommendations` は
//! 認証済み + consent 済み user にのみ、fail-closed query gate を通った `allow` verdict の entry を
//! 返す。機能未構成（既定。`CommunityIndex` = `Availability::Planned`）の node は 404。
//!
//! Postgres + Redis を要するため `KUKURI_CN_RUN_INTEGRATION_TESTS=1` で gate する。
//! query 境界は in-memory 実装（`MemoryIndexProjection` + `MemoryIndexEntryStore`）を
//! `with_index_query` で注入する（gate のセマンティクスは ArcadeDB 実装と共通の
//! `FailClosedIndexQuery`）。

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use kukuri_cn_core::{
    IndexEntryStore, IndexScopeKind, JwtConfig, MemoryIndexEntryStore, NewIndexEntry, TestDatabase,
    build_auth_envelope_json,
};
use kukuri_cn_indexer::projection::{IndexProjection, IndexedEntry, MemoryIndexProjection};
use kukuri_cn_indexer::query::FailClosedIndexQuery;
use kukuri_cn_safety::provider::SubjectKind;
use kukuri_cn_safety::{ReasonCode, SafetyAction, SafetyVerdict};
use kukuri_cn_safety_runtime::{MemorySafetyArtifactStore, SafetyArtifactStore};
use kukuri_cn_user_api::{UserApiConfig, app_router, build_state};
use kukuri_core::{KukuriKeys, generate_keys};
use reqwest::{Client, StatusCode};

mod support;
use support::{integration_test_admin_database_url, integration_test_rendezvous_redis_url};

/// テスト用の in-memory query 境界一式（seed 用の書き込みハンドルつき）。
struct MemoryIndex {
    store: Arc<MemorySafetyArtifactStore>,
    entries: Arc<MemoryIndexEntryStore>,
    projection: Arc<MemoryIndexProjection>,
    query: Arc<FailClosedIndexQuery>,
}

fn memory_index() -> MemoryIndex {
    let store = Arc::new(MemorySafetyArtifactStore::new());
    let entries = Arc::new(MemoryIndexEntryStore::new(store.clone()));
    let projection = Arc::new(MemoryIndexProjection::new());
    let query = Arc::new(FailClosedIndexQuery::new(
        projection.clone(),
        entries.clone(),
    ));
    MemoryIndex {
        store,
        entries,
        projection,
        query,
    }
}

fn verdict(action: SafetyAction, critical: bool) -> SafetyVerdict {
    SafetyVerdict {
        action,
        labels: Vec::new(),
        critical,
        reason_code: if action == SafetyAction::Allow {
            ReasonCode::NoKnownMatch
        } else {
            ReasonCode::CsamConfirmed
        },
        confidence: None,
        provider: Some("mock-known-csam".to_string()),
        provider_capability: None,
        policy_version: "policy-v1-test".to_string(),
        scanned_at: "2026-07-02T09:00:00Z".to_string(),
    }
}

impl MemoryIndex {
    /// allow verdict つきの entry を真実源 + 投影の両方へ seed する（ingest の allow 経路と同型）。
    async fn seed_allow(&self, scope_id: &str, object_id: &str, text: &str) -> Result<()> {
        let verdict_id = self
            .store
            .persist_verdict(
                SubjectKind::Post,
                object_id,
                &verdict(SafetyAction::Allow, false),
            )
            .await?;
        self.entries
            .upsert_entry(&NewIndexEntry {
                scope_kind: IndexScopeKind::PublicTopic,
                scope_id: scope_id.to_string(),
                object_id: object_id.to_string(),
                author_pubkey: "author".to_string(),
                created_at: 1_700_000_000,
                source_replica_id: format!("topic::{scope_id}"),
                verdict_id,
                verdict_action: "allow".to_string(),
                critical: false,
            })
            .await?;
        self.projection
            .upsert_entry(&IndexedEntry {
                scope_kind: IndexScopeKind::PublicTopic,
                scope_id: scope_id.to_string(),
                object_id: object_id.to_string(),
                author_pubkey: "author".to_string(),
                text: text.to_string(),
                created_at: 1_700_000_000,
                source_replica_id: format!("topic::{scope_id}"),
            })
            .await?;
        Ok(())
    }

    /// index 済み entry の verdict を非 allow / critical に更新する（de-index 前の状態を模す）。
    async fn flip_to_excluded(&self, object_id: &str) -> Result<()> {
        self.store
            .persist_verdict(
                SubjectKind::Post,
                object_id,
                &verdict(SafetyAction::Exclude, true),
            )
            .await?;
        Ok(())
    }
}

struct TestServer {
    task: tokio::task::JoinHandle<()>,
    database: TestDatabase,
    base_url: String,
}

impl TestServer {
    /// index query（in-memory）を注入した user-api server を起動する。
    /// `index` が None の場合は機能未構成の node（`/v1/index/*` は 404）。
    async fn spawn(
        admin_database_url: &str,
        prefix: &str,
        index: Option<Arc<FailClosedIndexQuery>>,
    ) -> Result<Self> {
        let database = TestDatabase::create(admin_database_url, prefix).await?;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .context("failed to bind test index query listener")?;
        let addr = listener.local_addr()?;
        let base_url = format!("http://{addr}");
        let mut state = build_state(&UserApiConfig {
            bind_addr: addr,
            database_url: database.database_url.clone(),
            rendezvous_redis_url: integration_test_rendezvous_redis_url(),
            rendezvous_key_prefix: format!("cn:test:{prefix}"),
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
        if let Some(index) = index {
            state = state.with_index_query(index);
        }
        let app = app_router(state);
        let task = tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .expect("index query server");
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

fn entry_ids(body: &serde_json::Value) -> Vec<String> {
    body["entries"]
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .map(|entry| entry["object_id"].as_str().unwrap_or_default().to_string())
                .collect()
        })
        .unwrap_or_default()
}

#[tokio::test]
async fn index_query_is_not_found_when_not_configured() -> Result<()> {
    // 既定（機能無効 = `CommunityIndex` Planned 相当）の node は index query を公開しない。
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api index query test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let server = TestServer::spawn(admin_database_url.as_str(), "cn_index_query_off", None).await?;
    let client = Client::new();
    for path in [
        "/v1/index/search?q=hello",
        "/v1/index/discovery",
        "/v1/index/recommendations",
    ] {
        let response = client
            .get(format!("{}{path}", server.base_url))
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
    }
    server.shutdown().await
}

#[tokio::test]
async fn index_query_requires_auth_and_consent() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api index query test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let index = memory_index();
    let server = TestServer::spawn(
        admin_database_url.as_str(),
        "cn_index_query_auth",
        Some(index.query.clone()),
    )
    .await?;
    let client = Client::new();

    let unauthenticated = client
        .get(format!("{}/v1/index/search?q=hello", server.base_url))
        .send()
        .await?;
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

    server.shutdown().await
}

#[tokio::test]
async fn search_discovery_recommendation_return_gated_allow_entries_only() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api index query test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let index = memory_index();
    // allow entry / verdict が後から exclude+critical に変わった entry / 投影残留（真実源なし）。
    index
        .seed_allow("rust", "post-kept", "tokio async runtime")
        .await?;
    index
        .seed_allow("rust", "post-flipped", "tokio flips later")
        .await?;
    index.flip_to_excluded("post-flipped").await?;
    index
        .projection
        .upsert_entry(&IndexedEntry {
            scope_kind: IndexScopeKind::PublicTopic,
            scope_id: "rust".to_string(),
            object_id: "post-ghost".to_string(),
            author_pubkey: "author".to_string(),
            text: "tokio ghost residue".to_string(),
            created_at: 1_700_000_001,
            source_replica_id: "topic::rust".to_string(),
        })
        .await?;

    let server = TestServer::spawn(
        admin_database_url.as_str(),
        "cn_index_query_gate",
        Some(index.query.clone()),
    )
    .await?;
    let client = Client::new();
    let keys = generate_keys();
    let token = authenticate_and_consent(&client, &server.base_url, &keys).await?;

    // topic 内検索: allow のみ（非 allow / critical / 投影残留は出ない）。
    let search = client
        .get(format!(
            "{}/v1/index/search?scope_kind=public_topic&scope_id=rust&q=tokio",
            server.base_url
        ))
        .bearer_auth(token.as_str())
        .send()
        .await?;
    assert_eq!(search.status(), StatusCode::OK);
    let body = search.json::<serde_json::Value>().await?;
    assert_eq!(entry_ids(&body), vec!["post-kept".to_string()]);

    // 横断検索も同じ gate を通る。
    let search_all = client
        .get(format!("{}/v1/index/search?q=tokio", server.base_url))
        .bearer_auth(token.as_str())
        .send()
        .await?;
    let body = search_all.json::<serde_json::Value>().await?;
    assert_eq!(entry_ids(&body), vec!["post-kept".to_string()]);

    // discovery / recommendation（新着列挙）にも非 allow / critical は入らない。
    for path in [
        "/v1/index/discovery?scope_kind=public_topic&scope_id=rust",
        "/v1/index/discovery",
        "/v1/index/recommendations",
    ] {
        let response = client
            .get(format!("{}{path}", server.base_url))
            .bearer_auth(token.as_str())
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        let body = response.json::<serde_json::Value>().await?;
        assert_eq!(entry_ids(&body), vec!["post-kept".to_string()], "{path}");
    }

    server.shutdown().await
}

#[tokio::test]
async fn index_query_validates_parameters() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-user-api index query test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let index = memory_index();
    let server = TestServer::spawn(
        admin_database_url.as_str(),
        "cn_index_query_params",
        Some(index.query.clone()),
    )
    .await?;
    let client = Client::new();
    let keys = generate_keys();
    let token = authenticate_and_consent(&client, &server.base_url, &keys).await?;

    // q 無しの検索は 400。
    let missing_q = client
        .get(format!("{}/v1/index/search", server.base_url))
        .bearer_auth(token.as_str())
        .send()
        .await?;
    assert_eq!(missing_q.status(), StatusCode::BAD_REQUEST);

    // scope_kind / scope_id は組で指定する（片方のみは 400）。
    let half_scope = client
        .get(format!(
            "{}/v1/index/search?q=hello&scope_kind=public_topic",
            server.base_url
        ))
        .bearer_auth(token.as_str())
        .send()
        .await?;
    assert_eq!(half_scope.status(), StatusCode::BAD_REQUEST);

    // 未知の scope_kind は 400。
    let bad_kind = client
        .get(format!(
            "{}/v1/index/discovery?scope_kind=unknown&scope_id=rust",
            server.base_url
        ))
        .bearer_auth(token.as_str())
        .send()
        .await?;
    assert_eq!(bad_kind.status(), StatusCode::BAD_REQUEST);

    server.shutdown().await
}
