#![allow(dead_code)]

use axum::http::HeaderMap;
use serde::Serialize;
use utoipa::openapi::server::ServerBuilder;
use utoipa::{OpenApi, ToSchema};

use crate::{ErrorResponse, HealthStatus};

#[derive(Serialize, ToSchema)]
pub struct BootstrapHintLatestResponse {
    pub seq: u64,
    pub received_at: i64,
    pub hint: serde_json::Value,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        healthz_doc,
        metrics_doc,
        openapi_doc,
        auth_challenge_doc,
        auth_verify_doc,
        policies_current_doc,
        policies_version_doc,
        consent_status_doc,
        consent_accept_doc,
        bootstrap_nodes_doc,
        bootstrap_hint_latest_doc,
        bootstrap_services_doc,
        subscription_request_doc,
        subscription_list_doc,
        subscription_delete_doc,
        community_suggest_doc,
        search_doc,
        trending_doc,
        report_doc,
        labels_doc,
        trust_report_doc,
        trust_communication_doc,
        personal_export_create_doc,
        personal_export_get_doc,
        personal_export_download_doc,
        personal_delete_create_doc,
        personal_delete_get_doc
    ),
    components(schemas(HealthStatus, ErrorResponse, BootstrapHintLatestResponse)),
    tags(
        (name = "user-api", description = "Kukuri community node user API")
    )
)]
pub struct UserApiDoc;

pub fn document(server_url: Option<&str>) -> utoipa::openapi::OpenApi {
    let mut doc = UserApiDoc::openapi();
    if let Some(url) = server_url {
        doc.servers = Some(vec![ServerBuilder::new().url(url).build()]);
    }
    doc
}

pub fn infer_server_url(headers: &HeaderMap) -> Option<String> {
    let host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get("host"))
        .and_then(|value| value.to_str().ok())?;
    let proto = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("http");
    Some(format!("{proto}://{host}"))
}

#[utoipa::path(
    get,
    path = "/healthz",
    responses((status = 200, body = HealthStatus), (status = 503, body = HealthStatus))
)]
fn healthz_doc() {}

#[utoipa::path(
    get,
    path = "/metrics",
    responses((status = 200, content_type = "text/plain", body = String))
)]
fn metrics_doc() {}

#[utoipa::path(
    get,
    path = "/v1/openapi.json",
    responses((status = 200, body = serde_json::Value))
)]
fn openapi_doc() {}

#[utoipa::path(
    post,
    path = "/v1/auth/challenge",
    request_body = serde_json::Value,
    responses((status = 200, body = serde_json::Value), (status = 400, body = ErrorResponse))
)]
fn auth_challenge_doc() {}

#[utoipa::path(
    post,
    path = "/v1/auth/verify",
    request_body = serde_json::Value,
    responses((status = 200, body = serde_json::Value), (status = 400, body = ErrorResponse))
)]
fn auth_verify_doc() {}

#[utoipa::path(
    get,
    path = "/v1/policies/current",
    responses((status = 200, body = serde_json::Value))
)]
fn policies_current_doc() {}

#[utoipa::path(
    get,
    path = "/v1/policies/{policy_type}/{version}",
    params(
        ("policy_type" = String, Path, description = "Policy type"),
        ("version" = String, Path, description = "Policy version"),
        ("locale" = Option<String>, Query, description = "Policy locale")
    ),
    responses((status = 200, body = serde_json::Value), (status = 404, body = ErrorResponse))
)]
fn policies_version_doc() {}

#[utoipa::path(
    get,
    path = "/v1/consents/status",
    responses((status = 200, body = serde_json::Value), (status = 401, body = ErrorResponse))
)]
fn consent_status_doc() {}

#[utoipa::path(
    post,
    path = "/v1/consents",
    request_body = serde_json::Value,
    responses((status = 200, body = serde_json::Value), (status = 412, body = ErrorResponse))
)]
fn consent_accept_doc() {}

#[utoipa::path(
    get,
    path = "/v1/bootstrap/nodes",
    responses((status = 200, body = serde_json::Value), (status = 401, body = ErrorResponse))
)]
fn bootstrap_nodes_doc() {}

#[utoipa::path(
    get,
    path = "/v1/bootstrap/hints/latest",
    params((
        "since" = Option<u64>,
        Query,
        description = "Return a hint only when latest seq is greater than this value"
    )),
    responses(
        (status = 200, body = BootstrapHintLatestResponse),
        (status = 204),
        (status = 401, body = ErrorResponse),
        (status = 428, body = ErrorResponse),
        (status = 429, body = ErrorResponse)
    )
)]
fn bootstrap_hint_latest_doc() {}

#[utoipa::path(
    get,
    path = "/v1/bootstrap/topics/{topic_id}/services",
    params(("topic_id" = String, Path, description = "Topic identifier")),
    responses((status = 200, body = serde_json::Value), (status = 401, body = ErrorResponse))
)]
fn bootstrap_services_doc() {}

#[utoipa::path(
    post,
    path = "/v1/topic-subscription-requests",
    request_body = serde_json::Value,
    responses(
        (status = 200, body = serde_json::Value),
        (status = 401, body = ErrorResponse),
        (status = 402, body = ErrorResponse),
        (status = 428, body = ErrorResponse),
        (status = 429, body = ErrorResponse)
    )
)]
fn subscription_request_doc() {}

#[utoipa::path(
    get,
    path = "/v1/topic-subscriptions",
    responses((status = 200, body = serde_json::Value))
)]
fn subscription_list_doc() {}

#[utoipa::path(
    delete,
    path = "/v1/topic-subscriptions/{topic_id}",
    params(("topic_id" = String, Path, description = "Topic identifier")),
    responses((status = 200, body = serde_json::Value), (status = 404, body = ErrorResponse))
)]
fn subscription_delete_doc() {}

#[utoipa::path(
    get,
    path = "/v1/communities/suggest",
    params(
        ("q" = String, Query, description = "Normalized suggest query"),
        ("limit" = Option<i64>, Query, description = "Result limit")
    ),
    responses((status = 200, body = serde_json::Value))
)]
fn community_suggest_doc() {}

#[utoipa::path(
    get,
    path = "/v1/search",
    params(
        ("topic" = String, Query, description = "Topic identifier"),
        ("q" = Option<String>, Query, description = "Search query"),
        ("limit" = Option<i64>, Query, description = "Result limit"),
        ("cursor" = Option<String>, Query, description = "Pagination cursor")
    ),
    responses((status = 200, body = serde_json::Value))
)]
fn search_doc() {}

#[utoipa::path(
    get,
    path = "/v1/trending",
    params(("topic" = String, Query, description = "Topic identifier")),
    responses((status = 200, body = serde_json::Value))
)]
fn trending_doc() {}

#[utoipa::path(
    post,
    path = "/v1/reports",
    request_body = serde_json::Value,
    responses((status = 200, body = serde_json::Value))
)]
fn report_doc() {}

#[utoipa::path(
    get,
    path = "/v1/labels",
    params(
        ("target" = Option<String>, Query, description = "Target filter"),
        ("topic" = Option<String>, Query, description = "Topic filter"),
        ("limit" = Option<i64>, Query, description = "Result limit"),
        ("cursor" = Option<String>, Query, description = "Pagination cursor")
    ),
    responses((status = 200, body = serde_json::Value))
)]
fn labels_doc() {}

#[utoipa::path(
    get,
    path = "/v1/trust/report-based",
    params(("subject" = String, Query, description = "Trust subject")),
    responses((status = 200, body = serde_json::Value))
)]
fn trust_report_doc() {}

#[utoipa::path(
    get,
    path = "/v1/trust/communication-density",
    params(("subject" = String, Query, description = "Trust subject")),
    responses((status = 200, body = serde_json::Value))
)]
fn trust_communication_doc() {}

#[utoipa::path(
    post,
    path = "/v1/personal-data-export-requests",
    responses((status = 200, body = serde_json::Value))
)]
fn personal_export_create_doc() {}

#[utoipa::path(
    get,
    path = "/v1/personal-data-export-requests/{export_request_id}",
    params(("export_request_id" = String, Path, description = "Export request identifier")),
    responses((status = 200, body = serde_json::Value), (status = 404, body = ErrorResponse))
)]
fn personal_export_get_doc() {}

#[utoipa::path(
    get,
    path = "/v1/personal-data-export-requests/{export_request_id}/download",
    params(
        ("export_request_id" = String, Path, description = "Export request identifier"),
        ("token" = String, Query, description = "Download token")
    ),
    responses((status = 200, content_type = "application/json", body = serde_json::Value))
)]
fn personal_export_download_doc() {}

#[utoipa::path(
    post,
    path = "/v1/personal-data-deletion-requests",
    responses((status = 200, body = serde_json::Value))
)]
fn personal_delete_create_doc() {}

#[utoipa::path(
    get,
    path = "/v1/personal-data-deletion-requests/{deletion_request_id}",
    params(("deletion_request_id" = String, Path, description = "Deletion request identifier")),
    responses((status = 200, body = serde_json::Value), (status = 404, body = ErrorResponse))
)]
fn personal_delete_get_doc() {}
