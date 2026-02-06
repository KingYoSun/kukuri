#![allow(dead_code)]

use axum::http::HeaderMap;
use serde::Serialize;
use utoipa::openapi::server::ServerBuilder;
use utoipa::{OpenApi, ToSchema};

use crate::access_control::{RevokeRequest, RevokeResponse, RotateRequest, RotateResponse};
use crate::auth::{AdminUser, LoginRequest, LoginResponse};
use crate::moderation::{
    LabelRow, ManualLabelRequest, ReportRow, RulePayload, RuleResponse,
};
use crate::policies::{
    PolicyRequest, PolicyResponse, PolicyUpdateRequest, PublishRequest,
};
use crate::reindex::{ReindexRequest, ReindexResponse};
use crate::services::{
    AuditLog, ServiceConfigResponse, ServiceHealth, ServiceInfo, UpdateServiceConfigRequest,
};
use crate::subscriptions::{
    NodeSubscription, NodeSubscriptionUpdate, Plan, PlanLimit, PlanRequest, ReviewRequest,
    SubscriptionRequestRow, SubscriptionRow, SubscriptionUpdate, UsageRow,
};
use crate::trust::{TrustJobRequest, TrustJobRow, TrustScheduleRow, TrustScheduleUpdate};
use crate::{ErrorResponse, HealthStatus};

#[derive(Serialize, ToSchema)]
pub struct StatusResponse {
    pub status: String,
}

#[derive(Serialize, ToSchema)]
pub struct ManualLabelResponse {
    pub label_id: String,
    pub status: String,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        healthz_doc,
        metrics_doc,
        openapi_doc,
        auth_login_doc,
        auth_logout_doc,
        auth_me_doc,
        services_list_doc,
        services_get_config_doc,
        services_update_config_doc,
        policies_list_doc,
        policies_create_doc,
        policies_update_doc,
        policies_publish_doc,
        policies_make_current_doc,
        moderation_rules_list_doc,
        moderation_rules_create_doc,
        moderation_rules_update_doc,
        moderation_rules_delete_doc,
        moderation_reports_list_doc,
        moderation_labels_list_doc,
        moderation_labels_create_doc,
        subscription_requests_list_doc,
        subscription_requests_approve_doc,
        subscription_requests_reject_doc,
        node_subscriptions_list_doc,
        node_subscriptions_update_doc,
        plans_list_doc,
        plans_create_doc,
        plans_update_doc,
        subscriptions_list_doc,
        subscriptions_upsert_doc,
        usage_list_doc,
        audit_logs_list_doc,
        access_control_rotate_doc,
        access_control_revoke_doc,
        trust_jobs_list_doc,
        trust_jobs_create_doc,
        trust_schedules_list_doc,
        trust_schedules_update_doc,
        reindex_doc
    ),
    components(
        schemas(
            HealthStatus,
            ErrorResponse,
            StatusResponse,
            ManualLabelResponse,
            LoginRequest,
            LoginResponse,
            AdminUser,
            ServiceInfo,
            ServiceHealth,
            ServiceConfigResponse,
            UpdateServiceConfigRequest,
            AuditLog,
            PolicyRequest,
            PolicyUpdateRequest,
            PublishRequest,
            PolicyResponse,
            RuleResponse,
            RulePayload,
            ReportRow,
            LabelRow,
            ManualLabelRequest,
            SubscriptionRequestRow,
            ReviewRequest,
            NodeSubscription,
            NodeSubscriptionUpdate,
            Plan,
            PlanLimit,
            PlanRequest,
            SubscriptionRow,
            SubscriptionUpdate,
            UsageRow,
            RotateRequest,
            RotateResponse,
            RevokeRequest,
            RevokeResponse,
            TrustJobRequest,
            TrustJobRow,
            TrustScheduleRow,
            TrustScheduleUpdate,
            ReindexRequest,
            ReindexResponse
        )
    ),
    tags(
        (name = "admin-api", description = "Kukuri community node admin API")
    )
)]
pub struct AdminApiDoc;

pub fn document(server_url: Option<&str>) -> utoipa::openapi::OpenApi {
    let mut doc = AdminApiDoc::openapi();
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
    path = "/v1/admin/auth/login",
    request_body = LoginRequest,
    responses((status = 200, body = LoginResponse), (status = 401, body = ErrorResponse))
)]
fn auth_login_doc() {}

#[utoipa::path(
    post,
    path = "/v1/admin/auth/logout",
    responses((status = 200, body = StatusResponse))
)]
fn auth_logout_doc() {}

#[utoipa::path(
    get,
    path = "/v1/admin/auth/me",
    responses((status = 200, body = AdminUser), (status = 401, body = ErrorResponse))
)]
fn auth_me_doc() {}

#[utoipa::path(
    get,
    path = "/v1/admin/services",
    responses((status = 200, body = [ServiceInfo]), (status = 401, body = ErrorResponse))
)]
fn services_list_doc() {}

#[utoipa::path(
    get,
    path = "/v1/admin/services/{service}/config",
    params(("service" = String, Path, description = "Service name")),
    responses((status = 200, body = ServiceConfigResponse), (status = 404, body = ErrorResponse))
)]
fn services_get_config_doc() {}

#[utoipa::path(
    put,
    path = "/v1/admin/services/{service}/config",
    params(("service" = String, Path, description = "Service name")),
    request_body = UpdateServiceConfigRequest,
    responses(
        (status = 200, body = ServiceConfigResponse),
        (status = 404, body = ErrorResponse),
        (status = 409, body = ErrorResponse)
    )
)]
fn services_update_config_doc() {}

#[utoipa::path(
    get,
    path = "/v1/admin/policies",
    params(
        ("policy_type" = Option<String>, Query, description = "Filter by policy type"),
        ("locale" = Option<String>, Query, description = "Filter by locale")
    ),
    responses((status = 200, body = [PolicyResponse]))
)]
fn policies_list_doc() {}

#[utoipa::path(
    post,
    path = "/v1/admin/policies",
    request_body = PolicyRequest,
    responses((status = 200, body = PolicyResponse))
)]
fn policies_create_doc() {}

#[utoipa::path(
    put,
    path = "/v1/admin/policies/{policy_id}",
    params(("policy_id" = String, Path, description = "Policy identifier")),
    request_body = PolicyUpdateRequest,
    responses((status = 200, body = PolicyResponse), (status = 404, body = ErrorResponse))
)]
fn policies_update_doc() {}

#[utoipa::path(
    post,
    path = "/v1/admin/policies/{policy_id}/publish",
    params(("policy_id" = String, Path, description = "Policy identifier")),
    request_body = PublishRequest,
    responses((status = 200, body = PolicyResponse), (status = 404, body = ErrorResponse))
)]
fn policies_publish_doc() {}

#[utoipa::path(
    post,
    path = "/v1/admin/policies/{policy_id}/make-current",
    params(("policy_id" = String, Path, description = "Policy identifier")),
    responses((status = 200, body = PolicyResponse), (status = 404, body = ErrorResponse))
)]
fn policies_make_current_doc() {}

#[utoipa::path(
    get,
    path = "/v1/admin/moderation/rules",
    params(
        ("enabled" = Option<bool>, Query, description = "Enabled flag filter"),
        ("limit" = Option<i64>, Query, description = "Max rows"),
        ("offset" = Option<i64>, Query, description = "Offset rows")
    ),
    responses((status = 200, body = [RuleResponse]))
)]
fn moderation_rules_list_doc() {}

#[utoipa::path(
    post,
    path = "/v1/admin/moderation/rules",
    request_body = RulePayload,
    responses((status = 200, body = RuleResponse), (status = 400, body = ErrorResponse))
)]
fn moderation_rules_create_doc() {}

#[utoipa::path(
    put,
    path = "/v1/admin/moderation/rules/{rule_id}",
    params(("rule_id" = String, Path, description = "Rule identifier")),
    request_body = RulePayload,
    responses((status = 200, body = RuleResponse), (status = 404, body = ErrorResponse))
)]
fn moderation_rules_update_doc() {}

#[utoipa::path(
    delete,
    path = "/v1/admin/moderation/rules/{rule_id}",
    params(("rule_id" = String, Path, description = "Rule identifier")),
    responses((status = 200, body = StatusResponse), (status = 404, body = ErrorResponse))
)]
fn moderation_rules_delete_doc() {}

#[utoipa::path(
    get,
    path = "/v1/admin/moderation/reports",
    params(
        ("target" = Option<String>, Query, description = "Target filter"),
        ("reporter_pubkey" = Option<String>, Query, description = "Reporter pubkey filter"),
        ("since" = Option<i64>, Query, description = "UNIX seconds lower bound"),
        ("limit" = Option<i64>, Query, description = "Max rows")
    ),
    responses((status = 200, body = [ReportRow]))
)]
fn moderation_reports_list_doc() {}

#[utoipa::path(
    get,
    path = "/v1/admin/moderation/labels",
    params(
        ("target" = Option<String>, Query, description = "Target filter"),
        ("topic" = Option<String>, Query, description = "Topic filter"),
        ("since" = Option<i64>, Query, description = "UNIX seconds lower bound"),
        ("limit" = Option<i64>, Query, description = "Max rows")
    ),
    responses((status = 200, body = [LabelRow]))
)]
fn moderation_labels_list_doc() {}

#[utoipa::path(
    post,
    path = "/v1/admin/moderation/labels",
    request_body = ManualLabelRequest,
    responses((status = 200, body = ManualLabelResponse), (status = 400, body = ErrorResponse))
)]
fn moderation_labels_create_doc() {}

#[utoipa::path(
    get,
    path = "/v1/admin/subscription-requests",
    params(("status" = Option<String>, Query, description = "Request status filter")),
    responses((status = 200, body = [SubscriptionRequestRow]))
)]
fn subscription_requests_list_doc() {}

#[utoipa::path(
    post,
    path = "/v1/admin/subscription-requests/{request_id}/approve",
    params(("request_id" = String, Path, description = "Subscription request identifier")),
    request_body = ReviewRequest,
    responses((status = 200, body = StatusResponse), (status = 404, body = ErrorResponse))
)]
fn subscription_requests_approve_doc() {}

#[utoipa::path(
    post,
    path = "/v1/admin/subscription-requests/{request_id}/reject",
    params(("request_id" = String, Path, description = "Subscription request identifier")),
    request_body = ReviewRequest,
    responses((status = 200, body = StatusResponse), (status = 404, body = ErrorResponse))
)]
fn subscription_requests_reject_doc() {}

#[utoipa::path(
    get,
    path = "/v1/admin/node-subscriptions",
    responses((status = 200, body = [NodeSubscription]))
)]
fn node_subscriptions_list_doc() {}

#[utoipa::path(
    put,
    path = "/v1/admin/node-subscriptions/{topic_id}",
    params(("topic_id" = String, Path, description = "Topic identifier")),
    request_body = NodeSubscriptionUpdate,
    responses((status = 200, body = NodeSubscription), (status = 404, body = ErrorResponse))
)]
fn node_subscriptions_update_doc() {}

#[utoipa::path(
    get,
    path = "/v1/admin/plans",
    responses((status = 200, body = [Plan]))
)]
fn plans_list_doc() {}

#[utoipa::path(
    post,
    path = "/v1/admin/plans",
    request_body = PlanRequest,
    responses((status = 200, body = Plan))
)]
fn plans_create_doc() {}

#[utoipa::path(
    put,
    path = "/v1/admin/plans/{plan_id}",
    params(("plan_id" = String, Path, description = "Plan identifier")),
    request_body = PlanRequest,
    responses((status = 200, body = Plan))
)]
fn plans_update_doc() {}

#[utoipa::path(
    get,
    path = "/v1/admin/subscriptions",
    params(("pubkey" = Option<String>, Query, description = "Subscriber pubkey filter")),
    responses((status = 200, body = [SubscriptionRow]))
)]
fn subscriptions_list_doc() {}

#[utoipa::path(
    put,
    path = "/v1/admin/subscriptions/{subscriber_pubkey}",
    params(("subscriber_pubkey" = String, Path, description = "Subscriber pubkey")),
    request_body = SubscriptionUpdate,
    responses((status = 200, body = StatusResponse))
)]
fn subscriptions_upsert_doc() {}

#[utoipa::path(
    get,
    path = "/v1/admin/usage",
    params(
        ("pubkey" = String, Query, description = "Subscriber pubkey"),
        ("metric" = Option<String>, Query, description = "Metric filter"),
        ("days" = Option<i64>, Query, description = "Range in days")
    ),
    responses((status = 200, body = [UsageRow]))
)]
fn usage_list_doc() {}

#[utoipa::path(
    get,
    path = "/v1/admin/audit-logs",
    params(
        ("action" = Option<String>, Query, description = "Action filter"),
        ("target" = Option<String>, Query, description = "Target filter"),
        ("since" = Option<i64>, Query, description = "UNIX seconds lower bound"),
        ("limit" = Option<i64>, Query, description = "Max rows")
    ),
    responses((status = 200, body = [AuditLog]))
)]
fn audit_logs_list_doc() {}

#[utoipa::path(
    post,
    path = "/v1/admin/access-control/rotate",
    request_body = RotateRequest,
    responses((status = 200, body = RotateResponse), (status = 400, body = ErrorResponse))
)]
fn access_control_rotate_doc() {}

#[utoipa::path(
    post,
    path = "/v1/admin/access-control/revoke",
    request_body = RevokeRequest,
    responses(
        (status = 200, body = RevokeResponse),
        (status = 400, body = ErrorResponse),
        (status = 404, body = ErrorResponse)
    )
)]
fn access_control_revoke_doc() {}

#[utoipa::path(
    get,
    path = "/v1/admin/trust/jobs",
    params(
        ("status" = Option<String>, Query, description = "Job status filter"),
        ("job_type" = Option<String>, Query, description = "Job type filter"),
        ("subject_pubkey" = Option<String>, Query, description = "Subject pubkey filter"),
        ("limit" = Option<i64>, Query, description = "Max rows")
    ),
    responses((status = 200, body = [TrustJobRow]))
)]
fn trust_jobs_list_doc() {}

#[utoipa::path(
    post,
    path = "/v1/admin/trust/jobs",
    request_body = TrustJobRequest,
    responses((status = 200, body = TrustJobRow), (status = 400, body = ErrorResponse))
)]
fn trust_jobs_create_doc() {}

#[utoipa::path(
    get,
    path = "/v1/admin/trust/schedules",
    responses((status = 200, body = [TrustScheduleRow]))
)]
fn trust_schedules_list_doc() {}

#[utoipa::path(
    put,
    path = "/v1/admin/trust/schedules/{job_type}",
    params(("job_type" = String, Path, description = "Job type")),
    request_body = TrustScheduleUpdate,
    responses((status = 200, body = TrustScheduleRow), (status = 400, body = ErrorResponse))
)]
fn trust_schedules_update_doc() {}

#[utoipa::path(
    post,
    path = "/v1/reindex",
    request_body = ReindexRequest,
    responses((status = 200, body = ReindexResponse), (status = 400, body = ErrorResponse))
)]
fn reindex_doc() {}
