use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use axum_extra::extract::cookie::CookieJar;
use regex::RegexBuilder;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{Postgres, QueryBuilder, Row};
use utoipa::ToSchema;

use cn_core::moderation::{LabelInput, RuleAction, RuleCondition};

use crate::auth::require_admin;
use crate::{ApiError, ApiResult, AppState};

#[derive(Deserialize)]
pub struct RuleQuery {
    pub enabled: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct RuleResponse {
    pub rule_id: String,
    pub name: String,
    pub description: Option<String>,
    pub is_enabled: bool,
    pub priority: i32,
    pub conditions: Value,
    pub action: Value,
    pub created_at: i64,
    pub updated_at: i64,
    pub updated_by: String,
}

#[derive(Deserialize, ToSchema)]
pub struct RulePayload {
    pub name: String,
    pub description: Option<String>,
    pub is_enabled: Option<bool>,
    pub priority: Option<i32>,
    #[schema(value_type = serde_json::Value)]
    pub conditions: RuleCondition,
    #[schema(value_type = serde_json::Value)]
    pub action: RuleAction,
}

#[derive(Deserialize, ToSchema)]
pub struct RuleTestSampleEvent {
    pub event_id: Option<String>,
    pub pubkey: String,
    pub kind: i32,
    pub content: String,
    pub tags: Vec<Vec<String>>,
}

#[derive(Deserialize, ToSchema)]
pub struct RuleTestRequest {
    #[schema(value_type = serde_json::Value)]
    pub conditions: RuleCondition,
    #[schema(value_type = serde_json::Value)]
    pub action: RuleAction,
    pub sample: RuleTestSampleEvent,
}

#[derive(Serialize, ToSchema)]
pub struct RuleTestLabelPreview {
    pub target: String,
    pub label: String,
    pub confidence: Option<f64>,
    pub exp: i64,
    pub policy_url: String,
    pub policy_ref: String,
}

#[derive(Serialize, ToSchema)]
pub struct RuleTestResponse {
    pub matched: bool,
    pub reasons: Vec<String>,
    pub preview: Option<RuleTestLabelPreview>,
}

#[derive(Deserialize)]
pub struct ReportQuery {
    pub target: Option<String>,
    pub reporter_pubkey: Option<String>,
    pub since: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct ReportRow {
    pub report_id: String,
    pub reporter_pubkey: String,
    pub target: String,
    pub reason: String,
    pub created_at: i64,
}

#[derive(Deserialize)]
pub struct LabelQuery {
    pub target: Option<String>,
    pub topic: Option<String>,
    pub since: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct LabelRow {
    pub label_id: String,
    pub target: String,
    pub topic_id: Option<String>,
    pub label: String,
    pub confidence: Option<f64>,
    pub policy_url: String,
    pub policy_ref: String,
    pub exp: i64,
    pub issuer_pubkey: String,
    pub rule_id: Option<String>,
    pub source: String,
    pub issued_at: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct ManualLabelRequest {
    pub target: String,
    pub label: String,
    pub confidence: Option<f64>,
    pub exp: i64,
    pub policy_url: String,
    pub policy_ref: String,
    pub topic_id: Option<String>,
}

pub async fn list_rules(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<RuleQuery>,
) -> ApiResult<Json<Vec<RuleResponse>>> {
    require_admin(&state, &jar).await?;

    let mut builder = QueryBuilder::<Postgres>::new(
        "SELECT rule_id, name, description, is_enabled, priority, conditions_json, action_json, created_at, updated_at, updated_by FROM cn_moderation.rules WHERE 1=1",
    );
    if let Some(enabled) = query.enabled {
        builder.push(" AND is_enabled = ");
        builder.push_bind(enabled);
    }
    builder.push(" ORDER BY priority DESC, updated_at DESC");
    let limit = query.limit.unwrap_or(200).clamp(1, 1000);
    builder.push(" LIMIT ");
    builder.push(limit.to_string());
    if let Some(offset) = query.offset {
        builder.push(" OFFSET ");
        builder.push(offset.max(0).to_string());
    }

    let rows = builder
        .build()
        .fetch_all(&state.pool)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    let mut rules = Vec::new();
    for row in rows {
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
        let updated_at: chrono::DateTime<chrono::Utc> = row.try_get("updated_at")?;
        rules.push(RuleResponse {
            rule_id: row.try_get("rule_id")?,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            is_enabled: row.try_get("is_enabled")?,
            priority: row.try_get("priority")?,
            conditions: row.try_get("conditions_json")?,
            action: row.try_get("action_json")?,
            created_at: created_at.timestamp(),
            updated_at: updated_at.timestamp(),
            updated_by: row.try_get("updated_by")?,
        });
    }

    Ok(Json(rules))
}

pub async fn create_rule(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<RulePayload>,
) -> ApiResult<Json<RuleResponse>> {
    let admin = require_admin(&state, &jar).await?;
    validate_rule_payload(&payload)?;

    let rule_id = uuid::Uuid::new_v4().to_string();
    let is_enabled = payload.is_enabled.unwrap_or(true);
    let priority = payload.priority.unwrap_or(0);
    let conditions_json = serde_json::to_value(&payload.conditions)
        .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_RULE", err.to_string()))?;
    let action_json = serde_json::to_value(&payload.action)
        .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_RULE", err.to_string()))?;

    sqlx::query(
        "INSERT INTO cn_moderation.rules          (rule_id, name, description, is_enabled, priority, conditions_json, action_json, updated_by)          VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(&rule_id)
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(is_enabled)
    .bind(priority)
    .bind(&conditions_json)
    .bind(&action_json)
    .bind(&admin.admin_user_id)
    .execute(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    crate::log_admin_audit(
        &state.pool,
        &admin.admin_user_id,
        "moderation_rule.create",
        &format!("rule:{rule_id}"),
        Some(json!({
            "name": payload.name,
            "description": payload.description,
            "is_enabled": is_enabled,
            "priority": priority,
            "conditions": conditions_json,
            "action": action_json
        })),
        None,
    )
    .await?;

    fetch_rule(&state.pool, &rule_id).await.map(Json)
}

pub async fn update_rule(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(rule_id): Path<String>,
    Json(payload): Json<RulePayload>,
) -> ApiResult<Json<RuleResponse>> {
    let admin = require_admin(&state, &jar).await?;
    validate_rule_payload(&payload)?;

    let is_enabled = payload.is_enabled.unwrap_or(true);
    let priority = payload.priority.unwrap_or(0);
    let conditions_json = serde_json::to_value(&payload.conditions)
        .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_RULE", err.to_string()))?;
    let action_json = serde_json::to_value(&payload.action)
        .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_RULE", err.to_string()))?;

    let result = sqlx::query(
        "UPDATE cn_moderation.rules          SET name = $1, description = $2, is_enabled = $3, priority = $4, conditions_json = $5, action_json = $6, updated_at = NOW(), updated_by = $7          WHERE rule_id = $8",
    )
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(is_enabled)
    .bind(priority)
    .bind(&conditions_json)
    .bind(&action_json)
    .bind(&admin.admin_user_id)
    .bind(&rule_id)
    .execute(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "rule not found",
        ));
    }

    crate::log_admin_audit(
        &state.pool,
        &admin.admin_user_id,
        "moderation_rule.update",
        &format!("rule:{rule_id}"),
        Some(json!({
            "name": payload.name,
            "description": payload.description,
            "is_enabled": is_enabled,
            "priority": priority,
            "conditions": conditions_json,
            "action": action_json
        })),
        None,
    )
    .await?;

    fetch_rule(&state.pool, &rule_id).await.map(Json)
}

pub async fn delete_rule(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(rule_id): Path<String>,
) -> ApiResult<Json<Value>> {
    let admin = require_admin(&state, &jar).await?;

    let result = sqlx::query("DELETE FROM cn_moderation.rules WHERE rule_id = $1")
        .bind(&rule_id)
        .execute(&state.pool)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    if result.rows_affected() == 0 {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "rule not found",
        ));
    }

    crate::log_admin_audit(
        &state.pool,
        &admin.admin_user_id,
        "moderation_rule.delete",
        &format!("rule:{rule_id}"),
        None,
        None,
    )
    .await?;

    Ok(Json(json!({ "status": "deleted" })))
}

pub async fn test_rule(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<RuleTestRequest>,
) -> ApiResult<Json<RuleTestResponse>> {
    let admin = require_admin(&state, &jar).await?;
    payload
        .conditions
        .validate()
        .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_RULE", err.to_string()))?;
    payload
        .action
        .validate()
        .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_RULE", err.to_string()))?;
    validate_test_sample(&payload.sample)?;

    let (matched, reasons) = evaluate_rule_test(&payload.conditions, &payload.sample)?;
    let preview = if matched {
        Some(RuleTestLabelPreview {
            target: sample_target(&payload.sample),
            label: payload.action.label.clone(),
            confidence: payload.action.confidence,
            exp: chrono::Utc::now()
                .timestamp()
                .saturating_add(payload.action.exp_seconds),
            policy_url: payload.action.policy_url.clone(),
            policy_ref: payload.action.policy_ref.clone(),
        })
    } else {
        None
    };

    crate::log_admin_audit(
        &state.pool,
        &admin.admin_user_id,
        "moderation_rule.test",
        &format!(
            "rule-test:{}",
            payload
                .sample
                .event_id
                .as_deref()
                .unwrap_or("manual-sample")
        ),
        Some(json!({
            "kind": payload.sample.kind,
            "matched": matched,
            "label": payload.action.label
        })),
        None,
    )
    .await?;

    Ok(Json(RuleTestResponse {
        matched,
        reasons,
        preview,
    }))
}

pub async fn list_reports(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<ReportQuery>,
) -> ApiResult<Json<Vec<ReportRow>>> {
    require_admin(&state, &jar).await?;

    let mut builder = QueryBuilder::<Postgres>::new(
        "SELECT report_id, reporter_pubkey, target, reason, created_at FROM cn_user.reports WHERE 1=1",
    );
    if let Some(target) = query.target {
        builder.push(" AND target = ");
        builder.push_bind(target);
    }
    if let Some(pubkey) = query.reporter_pubkey {
        builder.push(" AND reporter_pubkey = ");
        builder.push_bind(pubkey);
    }
    if let Some(since) = query.since {
        builder.push(" AND created_at >= to_timestamp(");
        builder.push_bind(since);
        builder.push(")");
    }
    builder.push(" ORDER BY created_at DESC");
    let limit = query.limit.unwrap_or(200).clamp(1, 1000);
    builder.push(" LIMIT ");
    builder.push(limit.to_string());

    let rows = builder
        .build()
        .fetch_all(&state.pool)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    let mut reports = Vec::new();
    for row in rows {
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
        reports.push(ReportRow {
            report_id: row.try_get("report_id")?,
            reporter_pubkey: row.try_get("reporter_pubkey")?,
            target: row.try_get("target")?,
            reason: row.try_get("reason")?,
            created_at: created_at.timestamp(),
        });
    }

    Ok(Json(reports))
}

pub async fn list_labels(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<LabelQuery>,
) -> ApiResult<Json<Vec<LabelRow>>> {
    require_admin(&state, &jar).await?;

    let mut builder = QueryBuilder::<Postgres>::new(
        "SELECT label_id, target, topic_id, label, confidence, policy_url, policy_ref, exp, issuer_pubkey, rule_id, source, issued_at FROM cn_moderation.labels WHERE 1=1",
    );
    if let Some(target) = query.target {
        builder.push(" AND target = ");
        builder.push_bind(target);
    }
    if let Some(topic) = query.topic {
        builder.push(" AND topic_id = ");
        builder.push_bind(topic);
    }
    if let Some(since) = query.since {
        builder.push(" AND issued_at >= to_timestamp(");
        builder.push_bind(since);
        builder.push(")");
    }
    builder.push(" ORDER BY issued_at DESC");
    let limit = query.limit.unwrap_or(200).clamp(1, 1000);
    builder.push(" LIMIT ");
    builder.push(limit.to_string());

    let rows = builder
        .build()
        .fetch_all(&state.pool)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    let mut labels = Vec::new();
    for row in rows {
        let issued_at: chrono::DateTime<chrono::Utc> = row.try_get("issued_at")?;
        labels.push(LabelRow {
            label_id: row.try_get("label_id")?,
            target: row.try_get("target")?,
            topic_id: row.try_get("topic_id")?,
            label: row.try_get("label")?,
            confidence: row.try_get("confidence")?,
            policy_url: row.try_get("policy_url")?,
            policy_ref: row.try_get("policy_ref")?,
            exp: row.try_get("exp")?,
            issuer_pubkey: row.try_get("issuer_pubkey")?,
            rule_id: row.try_get("rule_id")?,
            source: row.try_get("source")?,
            issued_at: issued_at.timestamp(),
        });
    }

    Ok(Json(labels))
}

pub async fn create_label(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<ManualLabelRequest>,
) -> ApiResult<Json<Value>> {
    let admin = require_admin(&state, &jar).await?;

    if payload.exp <= 0 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_LABEL",
            "exp must be positive",
        ));
    }
    if let Some(confidence) = payload.confidence {
        if !(0.0..=1.0).contains(&confidence) {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_LABEL",
                "confidence must be between 0 and 1",
            ));
        }
    }

    let input = LabelInput {
        target: payload.target.clone(),
        label: payload.label.clone(),
        confidence: payload.confidence,
        exp: payload.exp,
        policy_url: payload.policy_url.clone(),
        policy_ref: payload.policy_ref.clone(),
        topic_id: payload.topic_id.clone(),
    };
    input
        .validate()
        .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_LABEL", err.to_string()))?;

    let label_event =
        cn_core::moderation::build_label_event(&state.node_keys, &input).map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "LABEL_ERROR",
                err.to_string(),
            )
        })?;

    let label_json = serde_json::to_value(&label_event).map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "LABEL_ERROR",
            err.to_string(),
        )
    })?;
    let source_event_id = payload
        .target
        .strip_prefix("event:")
        .map(|value| value.to_string());

    sqlx::query(
        "INSERT INTO cn_moderation.labels          (label_id, source_event_id, target, topic_id, label, confidence, policy_url, policy_ref, exp, issuer_pubkey, rule_id, source, label_event_json)          VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NULL, $11, $12)          ON CONFLICT (label_id) DO NOTHING",
    )
    .bind(&label_event.id)
    .bind(source_event_id)
    .bind(&input.target)
    .bind(&input.topic_id)
    .bind(&input.label)
    .bind(input.confidence)
    .bind(&input.policy_url)
    .bind(&input.policy_ref)
    .bind(input.exp)
    .bind(&label_event.pubkey)
    .bind("manual")
    .bind(label_json)
    .execute(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    crate::log_admin_audit(
        &state.pool,
        &admin.admin_user_id,
        "moderation_label.manual",
        &format!("label:{}", label_event.id),
        Some(json!({
            "target": input.target,
            "label": input.label,
            "confidence": input.confidence,
            "exp": input.exp,
            "policy_url": input.policy_url,
            "policy_ref": input.policy_ref,
            "topic_id": input.topic_id
        })),
        None,
    )
    .await?;

    Ok(Json(
        json!({ "label_id": label_event.id, "status": "created" }),
    ))
}

fn validate_rule_payload(payload: &RulePayload) -> ApiResult<()> {
    if payload.name.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RULE",
            "name is required",
        ));
    }
    payload
        .conditions
        .validate()
        .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_RULE", err.to_string()))?;
    payload
        .action
        .validate()
        .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_RULE", err.to_string()))?;
    Ok(())
}

fn validate_test_sample(sample: &RuleTestSampleEvent) -> ApiResult<()> {
    let pubkey = sample.pubkey.trim();
    if pubkey.len() != 64 || !pubkey.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_SAMPLE",
            "sample pubkey must be a 64-char hex string",
        ));
    }
    if sample.kind < 0 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_SAMPLE",
            "sample kind must be 0 or greater",
        ));
    }
    for tag in &sample.tags {
        if tag.is_empty() || tag[0].trim().is_empty() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_SAMPLE",
                "sample tags must have a non-empty tag name",
            ));
        }
    }
    Ok(())
}

fn evaluate_rule_test(
    condition: &RuleCondition,
    sample: &RuleTestSampleEvent,
) -> ApiResult<(bool, Vec<String>)> {
    let mut reasons = Vec::new();

    if let Some(kinds) = &condition.kinds {
        if kinds.contains(&sample.kind) {
            reasons.push(format!("kind={} matched", sample.kind));
        } else {
            reasons.push(format!("kind={} did not match allowed kinds", sample.kind));
            return Ok((false, reasons));
        }
    }

    if let Some(authors) = &condition.author_pubkeys {
        if authors
            .iter()
            .any(|author| author.eq_ignore_ascii_case(sample.pubkey.trim()))
        {
            reasons.push("author pubkey matched".to_string());
        } else {
            reasons.push("author pubkey did not match".to_string());
            return Ok((false, reasons));
        }
    }

    if let Some(keywords) = &condition.content_keywords {
        let content = sample.content.to_lowercase();
        if keywords
            .iter()
            .any(|keyword| content.contains(&keyword.to_lowercase()))
        {
            reasons.push("content keyword matched".to_string());
        } else {
            reasons.push("content keyword did not match".to_string());
            return Ok((false, reasons));
        }
    }

    if let Some(pattern) = &condition.content_regex {
        let regex = RegexBuilder::new(pattern)
            .case_insensitive(true)
            .build()
            .map_err(|err| {
                ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_RULE",
                    format!("invalid content_regex: {err}"),
                )
            })?;
        if regex.is_match(&sample.content) {
            reasons.push("content regex matched".to_string());
        } else {
            reasons.push("content regex did not match".to_string());
            return Ok((false, reasons));
        }
    }

    if let Some(filters) = &condition.tag_filters {
        for (tag, values) in filters {
            let tag_values = sample_tag_values(&sample.tags, tag);
            if tag_values.is_empty() {
                reasons.push(format!("tag '{tag}' was not present"));
                return Ok((false, reasons));
            }
            if !values.is_empty()
                && !tag_values
                    .iter()
                    .any(|actual| values.iter().any(|expected| expected == actual))
            {
                reasons.push(format!("tag '{tag}' values did not match expected values"));
                return Ok((false, reasons));
            }
            reasons.push(format!("tag '{tag}' matched"));
        }
    }

    if reasons.is_empty() {
        reasons.push("no rule conditions configured; treated as match".to_string());
    }

    Ok((true, reasons))
}

fn sample_tag_values(tags: &[Vec<String>], name: &str) -> Vec<String> {
    tags.iter()
        .filter_map(|tag| {
            if tag.first().map(String::as_str) == Some(name) {
                tag.get(1).cloned()
            } else {
                None
            }
        })
        .collect()
}

fn sample_target(sample: &RuleTestSampleEvent) -> String {
    sample
        .event_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|event_id| format!("event:{event_id}"))
        .unwrap_or_else(|| format!("pubkey:{}", sample.pubkey.trim()))
}

async fn fetch_rule(pool: &sqlx::Pool<Postgres>, rule_id: &str) -> ApiResult<RuleResponse> {
    let row = sqlx::query(
        "SELECT rule_id, name, description, is_enabled, priority, conditions_json, action_json, created_at, updated_at, updated_by FROM cn_moderation.rules WHERE rule_id = $1",
    )
    .bind(rule_id)
    .fetch_optional(pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let Some(row) = row else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "rule not found",
        ));
    };

    let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
    let updated_at: chrono::DateTime<chrono::Utc> = row.try_get("updated_at")?;
    Ok(RuleResponse {
        rule_id: row.try_get("rule_id")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        is_enabled: row.try_get("is_enabled")?,
        priority: row.try_get("priority")?,
        conditions: row.try_get("conditions_json")?,
        action: row.try_get("action_json")?,
        created_at: created_at.timestamp(),
        updated_at: updated_at.timestamp(),
        updated_by: row.try_get("updated_by")?,
    })
}
