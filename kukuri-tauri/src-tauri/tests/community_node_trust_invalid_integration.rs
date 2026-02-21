use async_trait::async_trait;
use chrono::Utc;
use kukuri_lib::test_support::application::ports::group_key_store::GroupKeyStore;
use kukuri_lib::test_support::application::ports::key_manager::KeyManager;
use kukuri_lib::test_support::infrastructure::crypto::DefaultKeyManager;
use kukuri_lib::test_support::infrastructure::storage::{
    SecureGroupKeyStore, secure_storage::SecureStorage,
};
use kukuri_lib::test_support::presentation::dto::community_node_dto::{
    CommunityNodeAuthRequest, CommunityNodeConfigNodeRequest, CommunityNodeConfigRequest,
    CommunityNodeRoleConfig, CommunityNodeTrustAlgorithm, CommunityNodeTrustProviderRequest,
    CommunityNodeTrustRequest,
};
use kukuri_lib::test_support::presentation::handlers::CommunityNodeHandler;
use nostr_sdk::prelude::{EventBuilder, Keys, Kind, Tag};
use reqwest::Url;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration as StdDuration;
use tiny_http::{Header, Response, Server};

const TRUST_ASSERTION_KIND_PUBKEY: u16 = 30382;
const TRUST_ASSERTION_KIND_EVENT: u16 = 30383;
const TRUST_CLAIM_REPORT_BASED: &str = "moderation.risk";
const TRUST_CLAIM_COMMUNICATION_DENSITY: &str = "reputation";

#[derive(Debug)]
struct MockHttpResponse {
    status: u16,
    body: Option<Value>,
}

impl MockHttpResponse {
    fn json(status: u16, body: Value) -> Self {
        Self {
            status,
            body: Some(body),
        }
    }
}

#[derive(Debug, Clone)]
struct CapturedRequest {
    path: String,
    params: HashMap<String, String>,
}

#[derive(Default)]
struct TestSecureStorage {
    values: tokio::sync::RwLock<HashMap<String, String>>,
}

#[async_trait]
impl SecureStorage for TestSecureStorage {
    async fn store(
        &self,
        key: &str,
        value: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut values = self.values.write().await;
        values.insert(key.to_string(), value.to_string());
        Ok(())
    }

    async fn retrieve(
        &self,
        key: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        let values = self.values.read().await;
        Ok(values.get(key).cloned())
    }

    async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut values = self.values.write().await;
        values.remove(key);
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let values = self.values.read().await;
        Ok(values.contains_key(key))
    }

    async fn list_keys(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let values = self.values.read().await;
        Ok(values.keys().cloned().collect())
    }

    async fn clear(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut values = self.values.write().await;
        values.clear();
        Ok(())
    }
}

fn spawn_json_sequence_server(
    responses: Vec<MockHttpResponse>,
) -> (String, Receiver<CapturedRequest>, thread::JoinHandle<()>) {
    let server = Server::http("127.0.0.1:0").expect("mock server");
    let base_url = format!("http://{}", server.server_addr());
    let (tx, rx) = mpsc::channel();

    let handle = thread::spawn(move || {
        for response_spec in responses {
            let request = match server.recv_timeout(StdDuration::from_secs(8)) {
                Ok(Some(request)) => request,
                Ok(None) => break,
                Err(_) => break,
            };

            let parsed =
                Url::parse(&format!("http://localhost{}", request.url())).expect("request url");
            let params = parsed
                .query_pairs()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect::<HashMap<String, String>>();
            let captured = CapturedRequest {
                path: parsed.path().to_string(),
                params,
            };
            let _ = tx.send(captured);

            let mut response = match response_spec.body {
                Some(body) => {
                    let mut response = Response::from_string(body.to_string());
                    response.add_header(
                        Header::from_bytes("Content-Type", "application/json")
                            .expect("content-type header"),
                    );
                    response
                }
                None => Response::from_string(String::new()),
            };
            response = response.with_status_code(response_spec.status);
            let _ = request.respond(response);
        }
    });

    (base_url, rx, handle)
}

fn join_with_timeout(handle: thread::JoinHandle<()>, timeout: StdDuration) {
    let start = std::time::Instant::now();
    while !handle.is_finished() {
        assert!(
            start.elapsed() < timeout,
            "mock server join timed out after {:?}",
            timeout
        );
        thread::sleep(StdDuration::from_millis(10));
    }
    handle.join().expect("mock server thread panicked");
}

async fn build_test_handler() -> (Arc<CommunityNodeHandler>, String) {
    let key_manager = Arc::new(DefaultKeyManager::new());
    let keypair = key_manager.generate_keypair().await.expect("keypair");
    let secure_storage = Arc::new(TestSecureStorage::default());
    let group_key_store =
        Arc::new(SecureGroupKeyStore::new(secure_storage.clone())) as Arc<dyn GroupKeyStore>;

    (
        Arc::new(CommunityNodeHandler::new(
            key_manager,
            secure_storage,
            group_key_store,
        )),
        keypair.public_key,
    )
}

fn trust_roles() -> CommunityNodeRoleConfig {
    CommunityNodeRoleConfig {
        labels: false,
        trust: true,
        search: false,
        bootstrap: false,
    }
}

async fn configure_trust_nodes(handler: &CommunityNodeHandler, base_urls: Vec<String>) {
    handler.clear_config().await.expect("clear config");
    let nodes = base_urls
        .into_iter()
        .map(|base_url| CommunityNodeConfigNodeRequest {
            base_url,
            roles: Some(trust_roles()),
        })
        .collect();
    handler
        .set_config(CommunityNodeConfigRequest { nodes })
        .await
        .expect("set config");
}

async fn authenticate_node(handler: &CommunityNodeHandler, base_url: &str) {
    handler
        .authenticate(CommunityNodeAuthRequest {
            base_url: base_url.to_string(),
        })
        .await
        .expect("authenticate");
}

fn auth_challenge_response() -> MockHttpResponse {
    MockHttpResponse::json(
        200,
        json!({
            "challenge": "integration-trust-challenge",
            "expires_at": Utc::now().timestamp() + 300,
        }),
    )
}

fn auth_verify_response(pubkey: &str) -> MockHttpResponse {
    MockHttpResponse::json(
        200,
        json!({
            "access_token": "integration-token",
            "token_type": "Bearer",
            "expires_at": Utc::now().timestamp() + 600,
            "pubkey": pubkey,
        }),
    )
}

fn trust_response(score: f64, exp: i64, event_json: Value) -> MockHttpResponse {
    MockHttpResponse::json(
        200,
        json!({
            "score": score,
            "assertion": {
                "exp": exp,
                "event_json": event_json,
            }
        }),
    )
}

fn build_assertion_event_with_keys(
    keys: &Keys,
    kind: u16,
    subject: &str,
    d_tag: &str,
    exp: i64,
    claim: &str,
) -> Value {
    let exp_str = exp.to_string();
    let tags = vec![
        Tag::parse(["d", d_tag]).expect("d tag"),
        Tag::parse(["claim", claim]).expect("claim tag"),
        Tag::parse(["rank", "50"]).expect("rank tag"),
        Tag::parse(["expiration", exp_str.as_str()]).expect("expiration tag"),
    ];
    let content = json!({
        "subject": subject,
        "claim": claim,
        "score": 0.5,
        "exp": exp,
    })
    .to_string();
    let event = EventBuilder::new(Kind::Custom(kind), content)
        .tags(tags)
        .sign_with_keys(keys)
        .expect("sign assertion event");
    serde_json::to_value(event).expect("assertion event json")
}

fn expect_auth_and_trust_requests(rx: &Receiver<CapturedRequest>, subject: &str) {
    let challenge = rx
        .recv_timeout(StdDuration::from_secs(3))
        .expect("auth challenge request");
    assert_eq!(challenge.path, "/v1/auth/challenge");

    let verify = rx
        .recv_timeout(StdDuration::from_secs(3))
        .expect("auth verify request");
    assert_eq!(verify.path, "/v1/auth/verify");

    let trust = rx
        .recv_timeout(StdDuration::from_secs(3))
        .expect("trust request");
    assert_eq!(trust.path, "/v1/trust/report-based");
    assert_eq!(
        trust.params.get("subject").map(String::as_str),
        Some(subject)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn trust_report_based_rejects_invalid_subject() {
    let (handler, _) = build_test_handler().await;
    configure_trust_nodes(&handler, vec!["http://127.0.0.1:65500".to_string()]).await;

    let err = handler
        .trust_report_based(CommunityNodeTrustRequest {
            base_url: None,
            subject: "not-a-valid-subject".to_string(),
        })
        .await
        .expect_err("invalid subject should be rejected");
    assert!(err.to_string().contains("Invalid trust subject"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn trust_report_based_rejects_invalid_assertion_kind() {
    let (handler, user_pubkey) = build_test_handler().await;
    let subject_pubkey = Keys::generate().public_key().to_hex();
    let subject = format!("pubkey:{subject_pubkey}");
    let exp = Utc::now().timestamp() + 600;
    let invalid_kind_event = build_assertion_event_with_keys(
        &Keys::generate(),
        TRUST_ASSERTION_KIND_EVENT,
        &subject,
        &subject_pubkey,
        exp,
        TRUST_CLAIM_REPORT_BASED,
    );

    let (base_url, rx, handle) = spawn_json_sequence_server(vec![
        auth_challenge_response(),
        auth_verify_response(&user_pubkey),
        trust_response(0.91, exp, invalid_kind_event),
    ]);
    configure_trust_nodes(&handler, vec![base_url.clone()]).await;
    authenticate_node(&handler, &base_url).await;

    let err = handler
        .trust_report_based(CommunityNodeTrustRequest {
            base_url: None,
            subject: subject.clone(),
        })
        .await
        .expect_err("invalid assertion kind must be rejected");
    assert!(
        err.to_string()
            .contains("Community node trust assertion is invalid")
    );

    expect_auth_and_trust_requests(&rx, &subject);
    join_with_timeout(handle, StdDuration::from_secs(3));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn trust_report_based_rejects_invalid_assertion_tag_claim() {
    let (handler, user_pubkey) = build_test_handler().await;
    let subject_pubkey = Keys::generate().public_key().to_hex();
    let subject = format!("pubkey:{subject_pubkey}");
    let exp = Utc::now().timestamp() + 600;
    let invalid_claim_event = build_assertion_event_with_keys(
        &Keys::generate(),
        TRUST_ASSERTION_KIND_PUBKEY,
        &subject,
        &subject_pubkey,
        exp,
        TRUST_CLAIM_COMMUNICATION_DENSITY,
    );

    let (base_url, rx, handle) = spawn_json_sequence_server(vec![
        auth_challenge_response(),
        auth_verify_response(&user_pubkey),
        trust_response(0.72, exp, invalid_claim_event),
    ]);
    configure_trust_nodes(&handler, vec![base_url.clone()]).await;
    authenticate_node(&handler, &base_url).await;

    let err = handler
        .trust_report_based(CommunityNodeTrustRequest {
            base_url: None,
            subject: subject.clone(),
        })
        .await
        .expect_err("invalid assertion tag must be rejected");
    assert!(
        err.to_string()
            .contains("Community node trust assertion is invalid")
    );

    expect_auth_and_trust_requests(&rx, &subject);
    join_with_timeout(handle, StdDuration::from_secs(3));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn trust_report_based_rejects_malformed_assertion_event_json() {
    let (handler, user_pubkey) = build_test_handler().await;
    let subject_pubkey = Keys::generate().public_key().to_hex();
    let subject = format!("pubkey:{subject_pubkey}");
    let exp = Utc::now().timestamp() + 600;
    let malformed_event_json = json!({
        "kind": TRUST_ASSERTION_KIND_PUBKEY,
        "tags": "invalid-structure",
    });

    let (base_url, rx, handle) = spawn_json_sequence_server(vec![
        auth_challenge_response(),
        auth_verify_response(&user_pubkey),
        trust_response(0.67, exp, malformed_event_json),
    ]);
    configure_trust_nodes(&handler, vec![base_url.clone()]).await;
    authenticate_node(&handler, &base_url).await;

    let err = handler
        .trust_report_based(CommunityNodeTrustRequest {
            base_url: None,
            subject: subject.clone(),
        })
        .await
        .expect_err("malformed assertion data must be rejected");
    assert!(
        err.to_string()
            .contains("Community node trust assertion is invalid")
    );

    expect_auth_and_trust_requests(&rx, &subject);
    join_with_timeout(handle, StdDuration::from_secs(3));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn trust_report_based_rejects_assertion_signed_by_unexpected_provider_pubkey() {
    let (handler, user_pubkey) = build_test_handler().await;
    let subject_pubkey = Keys::generate().public_key().to_hex();
    let subject = format!("pubkey:{subject_pubkey}");
    let exp = Utc::now().timestamp() + 600;
    let expected_provider_keys = Keys::generate();
    let unexpected_provider_keys = Keys::generate();
    let assertion_event = build_assertion_event_with_keys(
        &unexpected_provider_keys,
        TRUST_ASSERTION_KIND_PUBKEY,
        &subject,
        &subject_pubkey,
        exp,
        TRUST_CLAIM_REPORT_BASED,
    );

    let (base_url, rx, handle) = spawn_json_sequence_server(vec![
        auth_challenge_response(),
        auth_verify_response(&user_pubkey),
        trust_response(0.55, exp, assertion_event),
    ]);
    configure_trust_nodes(&handler, vec![base_url.clone()]).await;
    authenticate_node(&handler, &base_url).await;
    handler
        .set_trust_provider(CommunityNodeTrustProviderRequest {
            provider_pubkey: expected_provider_keys.public_key().to_hex(),
            assertion_kind: Some(TRUST_ASSERTION_KIND_PUBKEY),
            relay_url: Some(base_url.clone()),
            algorithm: Some(CommunityNodeTrustAlgorithm::ReportBased),
        })
        .await
        .expect("set trust provider");

    let err = handler
        .trust_report_based(CommunityNodeTrustRequest {
            base_url: None,
            subject: subject.clone(),
        })
        .await
        .expect_err("unexpected provider pubkey must be rejected");
    assert!(
        err.to_string()
            .contains("Community node trust assertion is invalid")
    );

    expect_auth_and_trust_requests(&rx, &subject);
    join_with_timeout(handle, StdDuration::from_secs(3));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn trust_report_based_uses_only_valid_sources_when_invalid_source_is_present() {
    let (handler, user_pubkey) = build_test_handler().await;
    let subject_pubkey = Keys::generate().public_key().to_hex();
    let subject = format!("pubkey:{subject_pubkey}");
    let exp = Utc::now().timestamp() + 600;
    let invalid_event = build_assertion_event_with_keys(
        &Keys::generate(),
        TRUST_ASSERTION_KIND_EVENT,
        &subject,
        &subject_pubkey,
        exp,
        TRUST_CLAIM_REPORT_BASED,
    );
    let valid_event = build_assertion_event_with_keys(
        &Keys::generate(),
        TRUST_ASSERTION_KIND_PUBKEY,
        &subject,
        &subject_pubkey,
        exp,
        TRUST_CLAIM_REPORT_BASED,
    );

    let (invalid_base_url, invalid_rx, invalid_handle) = spawn_json_sequence_server(vec![
        auth_challenge_response(),
        auth_verify_response(&user_pubkey),
        trust_response(0.12, exp, invalid_event),
    ]);
    let (valid_base_url, valid_rx, valid_handle) = spawn_json_sequence_server(vec![
        auth_challenge_response(),
        auth_verify_response(&user_pubkey),
        trust_response(0.64, exp, valid_event),
    ]);

    configure_trust_nodes(
        &handler,
        vec![invalid_base_url.clone(), valid_base_url.clone()],
    )
    .await;
    authenticate_node(&handler, &invalid_base_url).await;
    authenticate_node(&handler, &valid_base_url).await;

    let response = handler
        .trust_report_based(CommunityNodeTrustRequest {
            base_url: None,
            subject: subject.clone(),
        })
        .await
        .expect("valid source should still produce trust score");

    let score = response
        .get("score")
        .and_then(Value::as_f64)
        .expect("score value");
    assert!((score - 0.64).abs() < 1e-9);

    let sources = response
        .get("sources")
        .and_then(Value::as_array)
        .cloned()
        .expect("sources array");
    assert_eq!(sources.len(), 1);
    assert_eq!(
        sources[0]
            .get("base_url")
            .and_then(Value::as_str)
            .map(str::to_string),
        Some(valid_base_url.clone())
    );
    assert_eq!(sources[0].get("score").and_then(Value::as_f64), Some(0.64));

    expect_auth_and_trust_requests(&invalid_rx, &subject);
    expect_auth_and_trust_requests(&valid_rx, &subject);
    join_with_timeout(invalid_handle, StdDuration::from_secs(3));
    join_with_timeout(valid_handle, StdDuration::from_secs(3));
}
