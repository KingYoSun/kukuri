use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{Postgres, Row, Transaction};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path as FsPath, PathBuf};

use crate::auth::require_auth;
use crate::policies::require_consents;
use crate::{ApiError, ApiResult, AppState};

const TRUST_GRAPH_NAME: &str = "kukuri_cn";
const TRUST_JOB_REPORT_BASED: &str = "report_based";
const TRUST_JOB_COMMUNICATION: &str = "communication_density";

#[derive(Serialize)]
pub struct ExportRequestResponse {
    pub export_request_id: String,
    pub status: String,
    pub download_token: Option<String>,
    pub download_expires_at: Option<i64>,
}

#[derive(Serialize)]
pub struct DeletionRequestResponse {
    pub deletion_request_id: String,
    pub status: String,
}

#[derive(Deserialize)]
pub struct DownloadQuery {
    pub token: String,
}

pub async fn create_export_request(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> ApiResult<Json<ExportRequestResponse>> {
    let auth = require_auth(&state, &headers).await?;
    require_consents(&state, &auth).await?;

    cleanup_expired_exports(&state).await?;

    let export_request_id = uuid::Uuid::new_v4().to_string();
    let download_token = uuid::Uuid::new_v4().to_string();
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);
    let path = build_export_path(&state.export_dir, &export_request_id);

    let payload = build_export_payload(&state, &auth.pubkey).await?;
    let contents = serde_json::to_vec_pretty(&payload).map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "EXPORT_ERROR",
            err.to_string(),
        )
    })?;
    fs::write(&path, contents).map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "EXPORT_ERROR",
            err.to_string(),
        )
    })?;

    sqlx::query(
        "INSERT INTO cn_user.personal_data_export_requests          (export_request_id, requester_pubkey, status, completed_at, download_token, download_expires_at, file_path)          VALUES ($1, $2, 'completed', NOW(), $3, $4, $5)",
    )
    .bind(&export_request_id)
    .bind(&auth.pubkey)
    .bind(&download_token)
    .bind(expires_at)
    .bind(path.to_string_lossy().to_string())
    .execute(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    Ok(Json(ExportRequestResponse {
        export_request_id,
        status: "completed".to_string(),
        download_token: Some(download_token),
        download_expires_at: Some(expires_at.timestamp()),
    }))
}

pub async fn get_export_request(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(export_request_id): Path<String>,
) -> ApiResult<Json<ExportRequestResponse>> {
    let auth = require_auth(&state, &headers).await?;
    let row = sqlx::query(
        "SELECT status, download_token, download_expires_at FROM cn_user.personal_data_export_requests          WHERE export_request_id = $1 AND requester_pubkey = $2",
    )
    .bind(&export_request_id)
    .bind(&auth.pubkey)
    .fetch_optional(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    let Some(row) = row else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "export not found",
        ));
    };

    let status: String = row.try_get("status")?;
    let token: Option<String> = row.try_get("download_token")?;
    let expires_at: Option<chrono::DateTime<chrono::Utc>> = row.try_get("download_expires_at")?;

    Ok(Json(ExportRequestResponse {
        export_request_id,
        status,
        download_token: token,
        download_expires_at: expires_at.map(|value| value.timestamp()),
    }))
}

pub async fn download_export(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(export_request_id): Path<String>,
    Query(query): Query<DownloadQuery>,
) -> ApiResult<Response> {
    let auth = require_auth(&state, &headers).await?;
    let row = sqlx::query(
        "SELECT download_token, download_expires_at, file_path FROM cn_user.personal_data_export_requests          WHERE export_request_id = $1 AND requester_pubkey = $2",
    )
    .bind(&export_request_id)
    .bind(&auth.pubkey)
    .fetch_optional(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    let Some(row) = row else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "export not found",
        ));
    };

    let token: Option<String> = row.try_get("download_token")?;
    if token.as_deref() != Some(query.token.as_str()) {
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "INVALID_TOKEN",
            "invalid download token",
        ));
    }

    let expires_at: Option<chrono::DateTime<chrono::Utc>> = row.try_get("download_expires_at")?;
    if let Some(expires_at) = expires_at {
        if chrono::Utc::now() > expires_at {
            return Err(ApiError::new(
                StatusCode::GONE,
                "EXPIRED",
                "download expired",
            ));
        }
    }

    let file_path: Option<String> = row.try_get("file_path")?;
    let Some(file_path) = file_path else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "file missing",
        ));
    };
    let data = fs::read(&file_path).map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "EXPORT_ERROR",
            err.to_string(),
        )
    })?;

    let mut response = axum::response::Response::new(axum::body::Body::from(data));
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        "application/json".parse().unwrap(),
    );
    response.headers_mut().insert(
        axum::http::header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{export_request_id}.json\"")
            .parse()
            .unwrap(),
    );
    Ok(response)
}

pub async fn create_deletion_request(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> ApiResult<Json<DeletionRequestResponse>> {
    let auth = require_auth(&state, &headers).await?;
    require_consents(&state, &auth).await?;

    let deletion_request_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO cn_user.personal_data_deletion_requests          (deletion_request_id, requester_pubkey, status)          VALUES ($1, $2, 'queued')",
    )
    .bind(&deletion_request_id)
    .bind(&auth.pubkey)
    .execute(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    sqlx::query(
        "UPDATE cn_user.subscriber_accounts SET status = 'deleting', updated_at = NOW() WHERE subscriber_pubkey = $1",
    )
    .bind(&auth.pubkey)
    .execute(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    perform_deletion(&state, &auth.pubkey, &deletion_request_id).await?;

    Ok(Json(DeletionRequestResponse {
        deletion_request_id,
        status: "completed".to_string(),
    }))
}

pub async fn get_deletion_request(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(deletion_request_id): Path<String>,
) -> ApiResult<Json<DeletionRequestResponse>> {
    let auth = require_auth(&state, &headers).await?;
    let row = sqlx::query(
        "SELECT status FROM cn_user.personal_data_deletion_requests          WHERE deletion_request_id = $1 AND requester_pubkey = $2",
    )
    .bind(&deletion_request_id)
    .bind(&auth.pubkey)
    .fetch_optional(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    let Some(row) = row else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "deletion not found",
        ));
    };
    let status: String = row.try_get("status")?;
    Ok(Json(DeletionRequestResponse {
        deletion_request_id,
        status,
    }))
}

fn build_export_path(base: &FsPath, export_request_id: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    path.push(format!("{export_request_id}.json"));
    path
}

async fn build_export_payload(state: &AppState, pubkey: &str) -> ApiResult<serde_json::Value> {
    let consents = sqlx::query(
        "SELECT policy_id, accepted_at FROM cn_user.policy_consents WHERE accepter_pubkey = $1",
    )
    .bind(pubkey)
    .fetch_all(&state.pool)
    .await
    .map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?
    .into_iter()
    .filter_map(|row| {
        let accepted_at: chrono::DateTime<chrono::Utc> = row.try_get("accepted_at").ok()?;
        Some(json!({
            "policy_id": row.try_get::<String, _>("policy_id").ok()?,
            "accepted_at": accepted_at.timestamp()
        }))
    })
    .collect::<Vec<_>>();

    let subscriptions = sqlx::query(
        "SELECT topic_id, status, started_at, ended_at FROM cn_user.topic_subscriptions WHERE subscriber_pubkey = $1",
    )
    .bind(pubkey)
    .fetch_all(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?
    .into_iter()
    .filter_map(|row| {
        let started_at: chrono::DateTime<chrono::Utc> = row.try_get("started_at").ok()?;
        let ended_at: Option<chrono::DateTime<chrono::Utc>> = row.try_get("ended_at").ok();
        Some(json!({
            "topic_id": row.try_get::<String, _>("topic_id").ok()?,
            "status": row.try_get::<String, _>("status").ok()?,
            "started_at": started_at.timestamp(),
            "ended_at": ended_at.map(|value| value.timestamp())
        }))
    })
    .collect::<Vec<_>>();

    let usage_events = sqlx::query(
        "SELECT metric, day, units, outcome, created_at FROM cn_user.usage_events WHERE subscriber_pubkey = $1",
    )
    .bind(pubkey)
    .fetch_all(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?
    .into_iter()
    .filter_map(|row| {
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at").ok()?;
        Some(json!({
            "metric": row.try_get::<String, _>("metric").ok()?,
            "day": row.try_get::<chrono::NaiveDate, _>("day").ok()?.to_string(),
            "units": row.try_get::<i64, _>("units").ok()?,
            "outcome": row.try_get::<String, _>("outcome").ok()?,
            "created_at": created_at.timestamp()
        }))
    })
    .collect::<Vec<_>>();

    let reports = sqlx::query(
        "SELECT target, reason, created_at FROM cn_user.reports WHERE reporter_pubkey = $1",
    )
    .bind(pubkey)
    .fetch_all(&state.pool)
    .await
    .map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?
    .into_iter()
    .filter_map(|row| {
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at").ok()?;
        Some(json!({
            "target": row.try_get::<String, _>("target").ok()?,
            "reason": row.try_get::<String, _>("reason").ok()?,
            "created_at": created_at.timestamp()
        }))
    })
    .collect::<Vec<_>>();

    let memberships = sqlx::query(
        "SELECT topic_id, scope, status, joined_at FROM cn_user.topic_memberships WHERE pubkey = $1",
    )
    .bind(pubkey)
    .fetch_all(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?
    .into_iter()
    .filter_map(|row| {
        let joined_at: chrono::DateTime<chrono::Utc> = row.try_get("joined_at").ok()?;
        Some(json!({
            "topic_id": row.try_get::<String, _>("topic_id").ok()?,
            "scope": row.try_get::<String, _>("scope").ok()?,
            "status": row.try_get::<String, _>("status").ok()?,
            "joined_at": joined_at.timestamp()
        }))
    })
    .collect::<Vec<_>>();

    let events = sqlx::query(
        "SELECT raw_json FROM cn_relay.events WHERE pubkey = $1 AND is_deleted = FALSE LIMIT 1000",
    )
    .bind(pubkey)
    .fetch_all(&state.pool)
    .await
    .map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?
    .into_iter()
    .filter_map(|row| row.try_get::<serde_json::Value, _>("raw_json").ok())
    .collect::<Vec<_>>();

    Ok(json!({
        "pubkey": pubkey,
        "generated_at": chrono::Utc::now().timestamp(),
        "consents": consents,
        "subscriptions": subscriptions,
        "usage_events": usage_events,
        "reports": reports,
        "memberships": memberships,
        "events": events
    }))
}

async fn cleanup_expired_exports(state: &AppState) -> ApiResult<()> {
    let rows = sqlx::query(
        "SELECT export_request_id, file_path FROM cn_user.personal_data_export_requests          WHERE download_expires_at IS NOT NULL AND download_expires_at < NOW()",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    for row in rows {
        let export_request_id: String = row.try_get("export_request_id")?;
        let file_path: Option<String> = row.try_get("file_path")?;
        if let Some(file_path) = file_path {
            let _ = fs::remove_file(&file_path);
        }
        sqlx::query(
            "UPDATE cn_user.personal_data_export_requests              SET download_token = NULL, download_expires_at = NULL, file_path = NULL              WHERE export_request_id = $1",
        )
        .bind(&export_request_id)
        .execute(&state.pool)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    }

    Ok(())
}

async fn load_subject_event_ids(
    tx: &mut Transaction<'_, Postgres>,
    pubkey: &str,
) -> ApiResult<Vec<String>> {
    let rows = sqlx::query(
        "SELECT event_id FROM cn_relay.events WHERE pubkey = $1 AND is_deleted = FALSE",
    )
    .bind(pubkey)
    .fetch_all(&mut **tx)
    .await
    .map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    Ok(rows
        .into_iter()
        .filter_map(|row| row.try_get::<String, _>("event_id").ok())
        .collect())
}

async fn load_subject_topics(
    tx: &mut Transaction<'_, Postgres>,
    pubkey: &str,
) -> ApiResult<Vec<String>> {
    let rows = sqlx::query(
        "SELECT DISTINCT t.topic_id          FROM cn_relay.events e          JOIN cn_relay.event_topics t            ON t.event_id = e.event_id          WHERE e.pubkey = $1            AND e.is_deleted = FALSE",
    )
    .bind(pubkey)
    .fetch_all(&mut **tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let mut topics = BTreeSet::new();
    for row in rows {
        if let Ok(topic_id) = row.try_get::<String, _>("topic_id") {
            if !topic_id.trim().is_empty() {
                topics.insert(topic_id);
            }
        }
    }
    Ok(topics.into_iter().collect())
}

async fn load_recompute_subjects(
    tx: &mut Transaction<'_, Postgres>,
    pubkey: &str,
) -> ApiResult<Vec<String>> {
    let rows = sqlx::query(
        "SELECT DISTINCT subject_pubkey          FROM (              SELECT subject_pubkey                FROM cn_trust.report_events               WHERE reporter_pubkey = $1                 AND subject_pubkey <> $1              UNION              SELECT CASE                        WHEN actor_pubkey = $1 THEN target_pubkey                        ELSE actor_pubkey                     END AS subject_pubkey                FROM cn_trust.interactions               WHERE actor_pubkey = $1 OR target_pubkey = $1          ) derived",
    )
    .bind(pubkey)
    .fetch_all(&mut **tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let mut subjects = BTreeSet::new();
    for row in rows {
        if let Ok(subject_pubkey) = row.try_get::<String, _>("subject_pubkey") {
            if subject_pubkey != pubkey && is_hex_64(&subject_pubkey) {
                subjects.insert(subject_pubkey);
            }
        }
    }
    Ok(subjects.into_iter().collect())
}

async fn cleanup_moderation_derived_data(
    tx: &mut Transaction<'_, Postgres>,
    pubkey: &str,
    event_ids: &[String],
) -> ApiResult<()> {
    let event_id_list = event_ids.to_vec();
    let event_targets: Vec<String> = event_id_list
        .iter()
        .map(|event_id| format!("event:{event_id}"))
        .collect();
    let subject_target = format!("pubkey:{pubkey}");

    sqlx::query(
        "DELETE FROM cn_moderation.labels          WHERE target = $1             OR issuer_pubkey = $2             OR source_event_id = ANY($3)             OR target = ANY($4)",
    )
    .bind(subject_target)
    .bind(pubkey)
    .bind(&event_id_list)
    .bind(&event_targets)
    .execute(&mut **tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    sqlx::query("DELETE FROM cn_moderation.jobs WHERE event_id = ANY($1)")
        .bind(&event_id_list)
        .execute(&mut **tx)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    Ok(())
}

async fn cleanup_trust_derived_data(
    tx: &mut Transaction<'_, Postgres>,
    pubkey: &str,
    affected_subjects: &[String],
) -> ApiResult<()> {
    let affected_subject_list = affected_subjects.to_vec();
    let affected_subject_targets: Vec<String> = affected_subject_list
        .iter()
        .map(|subject_pubkey| format!("pubkey:{subject_pubkey}"))
        .collect();
    let subject_target = format!("pubkey:{pubkey}");

    sqlx::query(
        "DELETE FROM cn_trust.report_events WHERE subject_pubkey = $1 OR reporter_pubkey = $1",
    )
    .bind(pubkey)
    .execute(&mut **tx)
    .await
    .map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    sqlx::query("DELETE FROM cn_trust.interactions WHERE actor_pubkey = $1 OR target_pubkey = $1")
        .bind(pubkey)
        .execute(&mut **tx)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    sqlx::query(
        "DELETE FROM cn_trust.report_scores WHERE subject_pubkey = $1 OR subject_pubkey = ANY($2)",
    )
    .bind(pubkey)
    .bind(&affected_subject_list)
    .execute(&mut **tx)
    .await
    .map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    sqlx::query(
        "DELETE FROM cn_trust.communication_scores WHERE subject_pubkey = $1 OR subject_pubkey = ANY($2)",
    )
    .bind(pubkey)
    .bind(&affected_subject_list)
    .execute(&mut **tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    sqlx::query(
        "DELETE FROM cn_trust.attestations          WHERE subject = $1             OR subject = ANY($2)             OR issuer_pubkey = $3",
    )
    .bind(&subject_target)
    .bind(&affected_subject_targets)
    .bind(pubkey)
    .execute(&mut **tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    Ok(())
}

async fn cleanup_relay_records(tx: &mut Transaction<'_, Postgres>, pubkey: &str) -> ApiResult<()> {
    sqlx::query(
        "INSERT INTO cn_relay.events_outbox          (op, event_id, topic_id, kind, created_at, ingested_at, effective_key, reason)          SELECT              'delete',              e.event_id,              t.topic_id,              e.kind,              e.created_at,              NOW(),              NULL,              'dsar'          FROM cn_relay.events e          JOIN cn_relay.event_topics t            ON t.event_id = e.event_id          WHERE e.pubkey = $1            AND e.is_deleted = FALSE",
    )
    .bind(pubkey)
    .execute(&mut **tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    sqlx::query(
        "UPDATE cn_relay.events          SET is_deleted = TRUE,              deleted_at = NOW(),              is_current = FALSE          WHERE pubkey = $1 AND is_deleted = FALSE",
    )
    .bind(pubkey)
    .execute(&mut **tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    sqlx::query("DELETE FROM cn_relay.replaceable_current WHERE pubkey = $1")
        .bind(pubkey)
        .execute(&mut **tx)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    sqlx::query("DELETE FROM cn_relay.addressable_current WHERE pubkey = $1")
        .bind(pubkey)
        .execute(&mut **tx)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    Ok(())
}

async fn remove_subject_from_age_graph(
    tx: &mut Transaction<'_, Postgres>,
    pubkey: &str,
) -> ApiResult<()> {
    if !is_hex_64(pubkey) {
        return Ok(());
    }

    let age_installed = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age')",
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;
    if !age_installed {
        return Ok(());
    }

    let graph_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM ag_catalog.ag_graph WHERE name = $1)",
    )
    .bind(TRUST_GRAPH_NAME)
    .fetch_one(&mut **tx)
    .await
    .map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;
    if !graph_exists {
        return Ok(());
    }

    sqlx::query("LOAD 'age'")
        .execute(&mut **tx)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;
    sqlx::query(r#"SET search_path = ag_catalog, "$user", public"#)
        .execute(&mut **tx)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    let query = format!("MATCH (u:User {{pubkey: '{pubkey}'}}) DETACH DELETE u");
    let statement = format!(
        "SELECT * FROM cypher('{TRUST_GRAPH_NAME}', $cypher${query}$cypher$) AS (v agtype)"
    );
    sqlx::query(&statement)
        .fetch_all(&mut **tx)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    Ok(())
}

async fn enqueue_reindex_jobs(
    tx: &mut Transaction<'_, Postgres>,
    topic_ids: &[String],
    requested_by: &str,
) -> ApiResult<()> {
    for topic_id in topic_ids {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (              SELECT 1                FROM cn_index.reindex_jobs               WHERE topic_id = $1                 AND status IN ('pending', 'running')          )",
        )
        .bind(topic_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

        if exists {
            continue;
        }

        sqlx::query(
            "INSERT INTO cn_index.reindex_jobs          (job_id, topic_id, status, requested_by)          VALUES ($1, $2, 'pending', $3)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(topic_id)
        .bind(requested_by)
        .execute(&mut **tx)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
    }
    Ok(())
}

async fn enqueue_trust_jobs(
    tx: &mut Transaction<'_, Postgres>,
    subject_pubkeys: &[String],
    requested_by: &str,
) -> ApiResult<()> {
    for subject_pubkey in subject_pubkeys {
        enqueue_trust_job(tx, TRUST_JOB_REPORT_BASED, subject_pubkey, requested_by).await?;
        enqueue_trust_job(tx, TRUST_JOB_COMMUNICATION, subject_pubkey, requested_by).await?;
    }
    Ok(())
}

async fn enqueue_trust_job(
    tx: &mut Transaction<'_, Postgres>,
    job_type: &str,
    subject_pubkey: &str,
    requested_by: &str,
) -> ApiResult<()> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (              SELECT 1                FROM cn_trust.jobs               WHERE job_type = $1                 AND subject_pubkey = $2                 AND status IN ('pending', 'running')          )",
    )
    .bind(job_type)
    .bind(subject_pubkey)
    .fetch_one(&mut **tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    if exists {
        return Ok(());
    }

    sqlx::query(
        "INSERT INTO cn_trust.jobs          (job_id, job_type, subject_pubkey, status, requested_by)          VALUES ($1, $2, $3, 'pending', $4)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(job_type)
    .bind(subject_pubkey)
    .bind(requested_by)
    .execute(&mut **tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    Ok(())
}

async fn perform_deletion(
    state: &AppState,
    pubkey: &str,
    deletion_request_id: &str,
) -> ApiResult<()> {
    let anon = anonymize_pubkey(&state.hmac_secret, pubkey);
    let mut tx = state.pool.begin().await.map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    let event_ids = load_subject_event_ids(&mut tx, pubkey).await?;
    let topic_ids = load_subject_topics(&mut tx, pubkey).await?;
    let recompute_subjects = load_recompute_subjects(&mut tx, pubkey).await?;
    let requested_by = format!("dsar:{deletion_request_id}");

    sqlx::query(
        "UPDATE cn_user.policy_consents          SET accepter_pubkey = $1, accepter_hmac = $1, ip = NULL, user_agent = NULL          WHERE accepter_pubkey = $2",
    )
    .bind(&anon)
    .bind(pubkey)
    .execute(&mut *tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    sqlx::query(
        "UPDATE cn_user.usage_events SET subscriber_pubkey = $1 WHERE subscriber_pubkey = $2",
    )
    .bind(&anon)
    .bind(pubkey)
    .execute(&mut *tx)
    .await
    .map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    sqlx::query("UPDATE cn_user.reports SET reporter_pubkey = $1 WHERE reporter_pubkey = $2")
        .bind(&anon)
        .bind(pubkey)
        .execute(&mut *tx)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    sqlx::query("DELETE FROM cn_user.usage_counters_daily WHERE subscriber_pubkey = $1")
        .bind(pubkey)
        .execute(&mut *tx)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    sqlx::query("DELETE FROM cn_user.topic_subscriptions WHERE subscriber_pubkey = $1")
        .bind(pubkey)
        .execute(&mut *tx)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    sqlx::query("DELETE FROM cn_user.topic_subscription_requests WHERE requester_pubkey = $1")
        .bind(pubkey)
        .execute(&mut *tx)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    sqlx::query("DELETE FROM cn_user.topic_memberships WHERE pubkey = $1")
        .bind(pubkey)
        .execute(&mut *tx)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    sqlx::query("DELETE FROM cn_user.key_envelopes WHERE recipient_pubkey = $1")
        .bind(pubkey)
        .execute(&mut *tx)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    sqlx::query("DELETE FROM cn_user.subscriptions WHERE subscriber_pubkey = $1")
        .bind(pubkey)
        .execute(&mut *tx)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;

    cleanup_moderation_derived_data(&mut tx, pubkey, &event_ids).await?;
    cleanup_trust_derived_data(&mut tx, pubkey, &recompute_subjects).await?;
    remove_subject_from_age_graph(&mut tx, pubkey).await?;
    cleanup_relay_records(&mut tx, pubkey).await?;
    enqueue_reindex_jobs(&mut tx, &topic_ids, &requested_by).await?;
    enqueue_trust_jobs(&mut tx, &recompute_subjects, &requested_by).await?;

    sqlx::query(
        "UPDATE cn_user.subscriber_accounts SET status = 'deleted', updated_at = NOW() WHERE subscriber_pubkey = $1",
    )
    .bind(pubkey)
    .execute(&mut *tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    sqlx::query(
        "UPDATE cn_user.personal_data_deletion_requests          SET status = 'completed', completed_at = NOW(), error_message = NULL          WHERE deletion_request_id = $1",
    )
    .bind(deletion_request_id)
    .execute(&mut *tx)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    tx.commit().await.map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    Ok(())
}

fn anonymize_pubkey(secret: &[u8], pubkey: &str) -> String {
    let mut key = [0u8; 32];
    if secret.len() >= 32 {
        key.copy_from_slice(&secret[..32]);
    } else {
        key[..secret.len()].copy_from_slice(secret);
    }
    let hash = blake3::keyed_hash(&key, pubkey.as_bytes());
    format!("hmac:{}", hash.to_hex())
}

fn is_hex_64(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cn_core::service_config;
    use nostr_sdk::prelude::Keys;
    use serde_json::json;
    use sqlx::postgres::PgPoolOptions;
    use sqlx::{Pool, Postgres, Row};
    use std::path::PathBuf;
    use std::sync::Arc;
    use tokio::sync::OnceCell;
    use uuid::Uuid;

    static MIGRATIONS: OnceCell<()> = OnceCell::const_new();

    fn database_url() -> String {
        std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://cn:cn_password@localhost:5432/cn".to_string())
    }

    async fn ensure_migrated(pool: &Pool<Postgres>) {
        MIGRATIONS
            .get_or_init(|| async {
                cn_core::migrations::run(pool)
                    .await
                    .expect("run migrations");
            })
            .await;
    }

    async fn test_state() -> AppState {
        let pool = PgPoolOptions::new()
            .connect(&database_url())
            .await
            .expect("connect database");
        ensure_migrated(&pool).await;

        let jwt_config = cn_core::auth::JwtConfig {
            issuer: "http://localhost".to_string(),
            audience: crate::TOKEN_AUDIENCE.to_string(),
            secret: "test-secret".to_string(),
            ttl_seconds: 3600,
        };
        let user_config = service_config::static_handle(json!({
            "rate_limit": { "enabled": false }
        }));
        let bootstrap_config = service_config::static_handle(json!({
            "auth": { "mode": "off" }
        }));

        AppState {
            pool,
            jwt_config,
            public_base_url: "http://localhost".to_string(),
            user_config,
            bootstrap_config,
            rate_limiter: Arc::new(cn_core::rate_limit::RateLimiter::new()),
            node_keys: Keys::generate(),
            export_dir: PathBuf::from("tmp/test_exports"),
            hmac_secret: b"test-secret".to_vec(),
            bootstrap_hints: Arc::new(crate::BootstrapHintStore::default()),
        }
    }

    async fn insert_relay_event(
        pool: &Pool<Postgres>,
        event_id: &str,
        pubkey: &str,
        kind: i32,
        created_at: i64,
        topics: &[String],
        replaceable_key: Option<&str>,
        addressable_key: Option<&str>,
    ) {
        let raw_json = json!({
            "id": event_id,
            "pubkey": pubkey,
            "kind": kind,
            "created_at": created_at,
            "tags": [["t", topics.first().cloned().unwrap_or_default()]],
            "content": "content",
            "sig": "sig"
        });
        sqlx::query(
            "INSERT INTO cn_relay.events              (event_id, pubkey, kind, created_at, tags, content, sig, raw_json, is_deleted, is_ephemeral, is_current, replaceable_key, addressable_key)              VALUES ($1, $2, $3, $4, $5, $6, $7, $8, FALSE, FALSE, TRUE, $9, $10)",
        )
        .bind(event_id)
        .bind(pubkey)
        .bind(kind)
        .bind(created_at)
        .bind(json!([]))
        .bind("content")
        .bind("sig")
        .bind(raw_json)
        .bind(replaceable_key)
        .bind(addressable_key)
        .execute(pool)
        .await
        .expect("insert relay event");

        for topic_id in topics {
            sqlx::query(
                "INSERT INTO cn_relay.event_topics (event_id, topic_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(event_id)
            .bind(topic_id)
            .execute(pool)
            .await
            .expect("insert event topic");
        }
    }

    async fn seed_age_graph_edge(
        pool: &Pool<Postgres>,
        actor_pubkey: &str,
        target_pubkey: &str,
    ) -> bool {
        let mut tx = pool.begin().await.expect("begin");
        let age_installed = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age')",
        )
        .fetch_one(&mut *tx)
        .await
        .expect("check age extension");
        if !age_installed {
            tx.rollback().await.ok();
            return false;
        }

        if sqlx::query("LOAD 'age'").execute(&mut *tx).await.is_err() {
            tx.rollback().await.ok();
            return false;
        }
        if sqlx::query(r#"SET search_path = ag_catalog, "$user", public"#)
            .execute(&mut *tx)
            .await
            .is_err()
        {
            tx.rollback().await.ok();
            return false;
        }

        let graph_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM ag_catalog.ag_graph WHERE name = $1)",
        )
        .bind(TRUST_GRAPH_NAME)
        .fetch_one(&mut *tx)
        .await
        .expect("check graph");
        if !graph_exists
            && sqlx::query("SELECT ag_catalog.create_graph($1)")
                .bind(TRUST_GRAPH_NAME)
                .execute(&mut *tx)
                .await
                .is_err()
        {
            tx.rollback().await.ok();
            return false;
        }

        let event_id = Uuid::new_v4().to_string();
        let query = format!(
            "MERGE (a:User {{pubkey: '{actor_pubkey}'}}) MERGE (b:User {{pubkey: '{target_pubkey}'}}) MERGE (a)-[:REPORTED {{event_id: '{event_id}', kind: 39005, created_at: 1}}]->(b)"
        );
        let statement = format!(
            "SELECT * FROM cypher('{TRUST_GRAPH_NAME}', $cypher${query}$cypher$) AS (v agtype)"
        );
        if sqlx::query(&statement).fetch_all(&mut *tx).await.is_err() {
            tx.rollback().await.ok();
            return false;
        }

        if tx.commit().await.is_err() {
            return false;
        }
        true
    }

    async fn age_vertex_count(pool: &Pool<Postgres>, pubkey: &str) -> Option<i64> {
        let mut tx = pool.begin().await.ok()?;
        let age_installed = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age')",
        )
        .fetch_one(&mut *tx)
        .await
        .ok()?;
        if !age_installed {
            tx.rollback().await.ok();
            return None;
        }
        let graph_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM ag_catalog.ag_graph WHERE name = $1)",
        )
        .bind(TRUST_GRAPH_NAME)
        .fetch_one(&mut *tx)
        .await
        .ok()?;
        if !graph_exists {
            tx.rollback().await.ok();
            return None;
        }

        sqlx::query("LOAD 'age'").execute(&mut *tx).await.ok()?;
        sqlx::query(r#"SET search_path = ag_catalog, "$user", public"#)
            .execute(&mut *tx)
            .await
            .ok()?;
        let cypher =
            format!("MATCH (u:User {{pubkey: '{pubkey}'}}) RETURN count(u) AS count_value");
        let statement = format!(
            "SELECT count_value::text AS count_value FROM cypher('{TRUST_GRAPH_NAME}', $cypher${cypher}$cypher$) AS (count_value agtype)"
        );
        let row = sqlx::query(&statement).fetch_one(&mut *tx).await.ok()?;
        let raw: String = row.try_get("count_value").ok()?;
        tx.rollback().await.ok();
        raw.trim_matches('"').parse::<i64>().ok()
    }

    #[tokio::test]
    async fn perform_deletion_removes_derived_data_and_enqueues_jobs() {
        let state = test_state().await;
        let pubkey = Keys::generate().public_key().to_hex();
        let other_pubkey = Keys::generate().public_key().to_hex();
        let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;
        let day = chrono::Utc::now().date_naive();
        let deletion_request_id = Uuid::new_v4().to_string();
        let requested_by = format!("dsar:{deletion_request_id}");
        let topic_a = format!("kukuri:{}", Uuid::new_v4().simple());
        let topic_b = format!("kukuri:{}", Uuid::new_v4().simple());
        let event_id_a = Uuid::new_v4().to_string();
        let event_id_b = Uuid::new_v4().to_string();
        let replaceable_key = format!("{pubkey}:10002");
        let addressable_key = format!("30023:{pubkey}:profile");
        let consent_id = Uuid::new_v4().to_string();
        let report_id = Uuid::new_v4().to_string();
        let usage_request_id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO cn_user.subscriber_accounts (subscriber_pubkey, status) VALUES ($1, 'active')",
        )
        .bind(&pubkey)
        .execute(&state.pool)
        .await
        .expect("insert subscriber");
        sqlx::query(
            "INSERT INTO cn_user.personal_data_deletion_requests (deletion_request_id, requester_pubkey, status) VALUES ($1, $2, 'queued')",
        )
        .bind(&deletion_request_id)
        .bind(&pubkey)
        .execute(&state.pool)
        .await
        .expect("insert deletion request");
        sqlx::query(
            "INSERT INTO cn_user.policy_consents (consent_id, policy_id, accepter_pubkey, ip, user_agent) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&consent_id)
        .bind(format!("privacy-{}", Uuid::new_v4()))
        .bind(&pubkey)
        .bind("127.0.0.1")
        .bind("agent")
        .execute(&state.pool)
        .await
        .expect("insert consent");
        sqlx::query(
            "INSERT INTO cn_user.usage_events (subscriber_pubkey, metric, day, request_id, units, outcome) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&pubkey)
        .bind("index.search_requests")
        .bind(day)
        .bind(&usage_request_id)
        .bind(1_i64)
        .bind("allowed")
        .execute(&state.pool)
        .await
        .expect("insert usage event");
        sqlx::query(
            "INSERT INTO cn_user.reports (report_id, reporter_pubkey, target, reason) VALUES ($1, $2, $3, $4)",
        )
        .bind(&report_id)
        .bind(&pubkey)
        .bind("event:target")
        .bind("spam")
        .execute(&state.pool)
        .await
        .expect("insert report");
        sqlx::query(
            "INSERT INTO cn_user.usage_counters_daily (subscriber_pubkey, metric, day, count) VALUES ($1, $2, $3, $4)",
        )
        .bind(&pubkey)
        .bind("index.search_requests")
        .bind(day)
        .bind(3_i64)
        .execute(&state.pool)
        .await
        .expect("insert usage counter");
        sqlx::query(
            "INSERT INTO cn_user.topic_subscriptions (topic_id, subscriber_pubkey, status) VALUES ($1, $2, 'active')",
        )
        .bind(&topic_a)
        .bind(&pubkey)
        .execute(&state.pool)
        .await
        .expect("insert topic subscription");
        sqlx::query(
            "INSERT INTO cn_user.topic_subscription_requests (request_id, requester_pubkey, topic_id, requested_services, status) VALUES ($1, $2, $3, $4, 'pending')",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&pubkey)
        .bind(&topic_a)
        .bind(json!(["index"]))
        .execute(&state.pool)
        .await
        .expect("insert topic request");
        sqlx::query(
            "INSERT INTO cn_user.topic_memberships (topic_id, scope, pubkey, status) VALUES ($1, 'public', $2, 'active')",
        )
        .bind(&topic_a)
        .bind(&pubkey)
        .execute(&state.pool)
        .await
        .expect("insert membership");
        sqlx::query(
            "INSERT INTO cn_user.key_envelopes (topic_id, scope, epoch, recipient_pubkey, key_envelope_event_json) VALUES ($1, 'public', 1, $2, $3)",
        )
        .bind(&topic_a)
        .bind(&pubkey)
        .bind(json!({ "id": Uuid::new_v4().to_string() }))
        .execute(&state.pool)
        .await
        .expect("insert key envelope");
        sqlx::query(
            "INSERT INTO cn_user.subscriptions (subscription_id, subscriber_pubkey, plan_id, status) VALUES ($1, $2, $3, 'active')",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&pubkey)
        .bind("default")
        .execute(&state.pool)
        .await
        .expect("insert subscription");

        insert_relay_event(
            &state.pool,
            &event_id_a,
            &pubkey,
            10002,
            now,
            &[topic_a.clone(), topic_b.clone()],
            Some(&replaceable_key),
            None,
        )
        .await;
        insert_relay_event(
            &state.pool,
            &event_id_b,
            &pubkey,
            30023,
            now,
            &[topic_b.clone()],
            None,
            Some(&addressable_key),
        )
        .await;
        sqlx::query(
            "INSERT INTO cn_relay.replaceable_current (replaceable_key, event_id, pubkey, kind, created_at) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&replaceable_key)
        .bind(&event_id_a)
        .bind(&pubkey)
        .bind(10002_i32)
        .bind(now)
        .execute(&state.pool)
        .await
        .expect("insert replaceable current");
        sqlx::query(
            "INSERT INTO cn_relay.addressable_current (addressable_key, event_id, pubkey, kind, d_tag, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&addressable_key)
        .bind(&event_id_b)
        .bind(&pubkey)
        .bind(30023_i32)
        .bind("profile")
        .bind(now)
        .execute(&state.pool)
        .await
        .expect("insert addressable current");

        let exp = now + 3600;
        sqlx::query(
            "INSERT INTO cn_moderation.labels              (label_id, source_event_id, target, topic_id, label, confidence, policy_url, policy_ref, exp, issuer_pubkey, source, label_event_json)              VALUES ($1, $2, $3, $4, 'spam', 0.9, $5, $6, $7, $8, 'rule', $9)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&event_id_a)
        .bind(format!("event:{event_id_a}"))
        .bind(&topic_a)
        .bind("https://example.com/policy")
        .bind("moderation-v1")
        .bind(exp)
        .bind(&pubkey)
        .bind(json!({ "id": Uuid::new_v4().to_string() }))
        .execute(&state.pool)
        .await
        .expect("insert moderation label event target");
        sqlx::query(
            "INSERT INTO cn_moderation.labels              (label_id, source_event_id, target, topic_id, label, confidence, policy_url, policy_ref, exp, issuer_pubkey, source, label_event_json)              VALUES ($1, NULL, $2, $3, 'nsfw', 0.8, $4, $5, $6, $7, 'admin', $8)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(format!("pubkey:{pubkey}"))
        .bind(&topic_b)
        .bind("https://example.com/policy")
        .bind("moderation-v1")
        .bind(exp)
        .bind("issuer")
        .bind(json!({ "id": Uuid::new_v4().to_string() }))
        .execute(&state.pool)
        .await
        .expect("insert moderation label pubkey target");
        sqlx::query(
            "INSERT INTO cn_moderation.jobs (job_id, event_id, topic_id, source, status) VALUES ($1, $2, $3, 'outbox', 'pending')",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&event_id_a)
        .bind(&topic_a)
        .execute(&state.pool)
        .await
        .expect("insert moderation job");

        sqlx::query(
            "INSERT INTO cn_trust.report_events              (event_id, subject_pubkey, reporter_pubkey, target, reason, source_kind, topic_id, created_at)              VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&other_pubkey)
        .bind(&pubkey)
        .bind(format!("pubkey:{other_pubkey}"))
        .bind("spam")
        .bind(39005_i32)
        .bind(&topic_a)
        .bind(now)
        .execute(&state.pool)
        .await
        .expect("insert trust report event");
        sqlx::query(
            "INSERT INTO cn_trust.interactions              (event_id, actor_pubkey, target_pubkey, weight, topic_id, created_at)              VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&pubkey)
        .bind(&other_pubkey)
        .bind(1.0_f64)
        .bind(&topic_b)
        .bind(now)
        .execute(&state.pool)
        .await
        .expect("insert trust interaction");
        sqlx::query(
            "INSERT INTO cn_trust.report_scores              (subject_pubkey, score, report_count, label_count, window_start, window_end)              VALUES ($1, 0.8, 2, 1, $2, $3)",
        )
        .bind(&pubkey)
        .bind(now - 600)
        .bind(now)
        .execute(&state.pool)
        .await
        .expect("insert trust report score self");
        sqlx::query(
            "INSERT INTO cn_trust.report_scores              (subject_pubkey, score, report_count, label_count, window_start, window_end)              VALUES ($1, 0.6, 1, 0, $2, $3)",
        )
        .bind(&other_pubkey)
        .bind(now - 600)
        .bind(now)
        .execute(&state.pool)
        .await
        .expect("insert trust report score other");
        sqlx::query(
            "INSERT INTO cn_trust.communication_scores              (subject_pubkey, score, interaction_count, peer_count, window_start, window_end)              VALUES ($1, 0.7, 3, 2, $2, $3)",
        )
        .bind(&pubkey)
        .bind(now - 600)
        .bind(now)
        .execute(&state.pool)
        .await
        .expect("insert trust communication score self");
        sqlx::query(
            "INSERT INTO cn_trust.communication_scores              (subject_pubkey, score, interaction_count, peer_count, window_start, window_end)              VALUES ($1, 0.4, 2, 1, $2, $3)",
        )
        .bind(&other_pubkey)
        .bind(now - 600)
        .bind(now)
        .execute(&state.pool)
        .await
        .expect("insert trust communication score other");
        sqlx::query(
            "INSERT INTO cn_trust.attestations              (attestation_id, subject, claim, score, exp, issuer_pubkey, event_json)              VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(format!("pubkey:{pubkey}"))
        .bind("moderation.risk")
        .bind(0.7_f64)
        .bind(exp)
        .bind("issuer")
        .bind(json!({ "id": Uuid::new_v4().to_string() }))
        .execute(&state.pool)
        .await
        .expect("insert trust attestation self");
        sqlx::query(
            "INSERT INTO cn_trust.attestations              (attestation_id, subject, claim, score, exp, issuer_pubkey, event_json)              VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(format!("pubkey:{other_pubkey}"))
        .bind("communication.density")
        .bind(0.5_f64)
        .bind(exp)
        .bind("issuer")
        .bind(json!({ "id": Uuid::new_v4().to_string() }))
        .execute(&state.pool)
        .await
        .expect("insert trust attestation other");

        let age_seeded = seed_age_graph_edge(&state.pool, &pubkey, &other_pubkey).await;

        perform_deletion(&state, &pubkey, &deletion_request_id)
            .await
            .expect("perform deletion");

        let consent_row = sqlx::query(
            "SELECT accepter_pubkey, accepter_hmac, ip, user_agent FROM cn_user.policy_consents WHERE consent_id = $1",
        )
        .bind(&consent_id)
        .fetch_one(&state.pool)
        .await
        .expect("fetch consent");
        let anonymized_pubkey: String = consent_row
            .try_get("accepter_pubkey")
            .expect("accepter_pubkey");
        let accepter_hmac: String = consent_row.try_get("accepter_hmac").expect("accepter_hmac");
        let ip: Option<String> = consent_row.try_get("ip").expect("ip");
        let user_agent: Option<String> = consent_row.try_get("user_agent").expect("user_agent");
        assert!(anonymized_pubkey.starts_with("hmac:"));
        assert_eq!(anonymized_pubkey, accepter_hmac);
        assert!(ip.is_none());
        assert!(user_agent.is_none());

        let usage_subscriber: String = sqlx::query_scalar(
            "SELECT subscriber_pubkey FROM cn_user.usage_events WHERE request_id = $1",
        )
        .bind(&usage_request_id)
        .fetch_one(&state.pool)
        .await
        .expect("fetch usage subscriber");
        assert_eq!(usage_subscriber, anonymized_pubkey);
        let reporter_pubkey: String =
            sqlx::query_scalar("SELECT reporter_pubkey FROM cn_user.reports WHERE report_id = $1")
                .bind(&report_id)
                .fetch_one(&state.pool)
                .await
                .expect("fetch reporter");
        assert_eq!(reporter_pubkey, anonymized_pubkey);

        let usage_counter_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM cn_user.usage_counters_daily WHERE subscriber_pubkey = $1",
        )
        .bind(&pubkey)
        .fetch_one(&state.pool)
        .await
        .expect("count usage counters");
        assert_eq!(usage_counter_count, 0);

        let relay_deleted_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM cn_relay.events WHERE pubkey = $1 AND is_deleted = TRUE AND is_current = FALSE",
        )
        .bind(&pubkey)
        .fetch_one(&state.pool)
        .await
        .expect("count deleted relay events");
        assert_eq!(relay_deleted_count, 2);

        let outbox_delete_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM cn_relay.events_outbox WHERE op = 'delete' AND reason = 'dsar' AND event_id = ANY($1)",
        )
        .bind(vec![event_id_a.clone(), event_id_b.clone()])
        .fetch_one(&state.pool)
        .await
        .expect("count outbox delete");
        assert_eq!(outbox_delete_count, 3);

        let replaceable_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM cn_relay.replaceable_current WHERE pubkey = $1",
        )
        .bind(&pubkey)
        .fetch_one(&state.pool)
        .await
        .expect("count replaceable");
        assert_eq!(replaceable_count, 0);
        let addressable_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM cn_relay.addressable_current WHERE pubkey = $1",
        )
        .bind(&pubkey)
        .fetch_one(&state.pool)
        .await
        .expect("count addressable");
        assert_eq!(addressable_count, 0);

        let moderation_label_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM cn_moderation.labels WHERE target = $1 OR target = $2 OR target = $3 OR source_event_id = ANY($4)",
        )
        .bind(format!("pubkey:{pubkey}"))
        .bind(format!("event:{event_id_a}"))
        .bind(format!("event:{event_id_b}"))
        .bind(vec![event_id_a.clone(), event_id_b.clone()])
        .fetch_one(&state.pool)
        .await
        .expect("count moderation labels");
        assert_eq!(moderation_label_count, 0);
        let moderation_job_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM cn_moderation.jobs WHERE event_id = ANY($1)")
                .bind(vec![event_id_a.clone(), event_id_b.clone()])
                .fetch_one(&state.pool)
                .await
                .expect("count moderation jobs");
        assert_eq!(moderation_job_count, 0);

        let trust_report_event_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM cn_trust.report_events WHERE reporter_pubkey = $1 OR subject_pubkey = $1",
        )
        .bind(&pubkey)
        .fetch_one(&state.pool)
        .await
        .expect("count trust report events");
        assert_eq!(trust_report_event_count, 0);
        let trust_interaction_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM cn_trust.interactions WHERE actor_pubkey = $1 OR target_pubkey = $1",
        )
        .bind(&pubkey)
        .fetch_one(&state.pool)
        .await
        .expect("count trust interactions");
        assert_eq!(trust_interaction_count, 0);
        let trust_scores_self: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM cn_trust.report_scores WHERE subject_pubkey = $1",
        )
        .bind(&pubkey)
        .fetch_one(&state.pool)
        .await
        .expect("count trust report score self");
        assert_eq!(trust_scores_self, 0);
        let trust_communication_self: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM cn_trust.communication_scores WHERE subject_pubkey = $1",
        )
        .bind(&pubkey)
        .fetch_one(&state.pool)
        .await
        .expect("count trust communication score self");
        assert_eq!(trust_communication_self, 0);

        let trust_job_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM cn_trust.jobs WHERE requested_by = $1 AND subject_pubkey = $2 AND job_type IN ($3, $4) AND status = 'pending'",
        )
        .bind(&requested_by)
        .bind(&other_pubkey)
        .bind(TRUST_JOB_REPORT_BASED)
        .bind(TRUST_JOB_COMMUNICATION)
        .fetch_one(&state.pool)
        .await
        .expect("count trust jobs");
        assert_eq!(trust_job_count, 2);

        let reindex_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM cn_index.reindex_jobs WHERE requested_by = $1 AND topic_id = ANY($2) AND status = 'pending'",
        )
        .bind(&requested_by)
        .bind(vec![topic_a.clone(), topic_b.clone()])
        .fetch_one(&state.pool)
        .await
        .expect("count reindex jobs");
        assert_eq!(reindex_count, 2);

        let account_status: String = sqlx::query_scalar(
            "SELECT status FROM cn_user.subscriber_accounts WHERE subscriber_pubkey = $1",
        )
        .bind(&pubkey)
        .fetch_one(&state.pool)
        .await
        .expect("account status");
        assert_eq!(account_status, "deleted");
        let deletion_status: String = sqlx::query_scalar(
            "SELECT status FROM cn_user.personal_data_deletion_requests WHERE deletion_request_id = $1",
        )
        .bind(&deletion_request_id)
        .fetch_one(&state.pool)
        .await
        .expect("deletion status");
        assert_eq!(deletion_status, "completed");

        if age_seeded {
            assert_eq!(age_vertex_count(&state.pool, &pubkey).await, Some(0));
        }
    }
}
