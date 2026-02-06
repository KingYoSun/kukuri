use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{Postgres, QueryBuilder, Row};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::require_admin;
use crate::{ApiError, ApiResult, AppState};

const JOB_REPORT_BASED: &str = "report_based";
const JOB_COMMUNICATION: &str = "communication_density";

#[derive(Deserialize)]
pub struct TrustJobQuery {
    pub status: Option<String>,
    pub job_type: Option<String>,
    pub subject_pubkey: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Deserialize, ToSchema)]
pub struct TrustJobRequest {
    pub job_type: String,
    pub subject_pubkey: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct TrustJobRow {
    pub job_id: String,
    pub job_type: String,
    pub subject_pubkey: Option<String>,
    pub status: String,
    pub total_targets: Option<i64>,
    pub processed_targets: i64,
    pub requested_by: String,
    pub requested_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub error_message: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct TrustScheduleRow {
    pub job_type: String,
    pub interval_seconds: i64,
    pub next_run_at: i64,
    pub is_enabled: bool,
    pub updated_at: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct TrustScheduleUpdate {
    pub interval_seconds: i64,
    pub is_enabled: bool,
}

pub async fn list_jobs(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Query(query): Query<TrustJobQuery>,
) -> ApiResult<Json<Vec<TrustJobRow>>> {
    require_admin(&state, &jar).await?;

    let mut builder = QueryBuilder::<Postgres>::new(
        "SELECT job_id, job_type, subject_pubkey, status, total_targets, processed_targets, requested_by, requested_at, started_at, completed_at, error_message FROM cn_trust.jobs",
    );
    let mut has_where = false;
    if let Some(status) = query.status.as_ref() {
        builder.push(if has_where { " AND " } else { " WHERE " });
        builder.push("status = ");
        builder.push_bind(status);
        has_where = true;
    }
    if let Some(job_type) = query.job_type.as_ref() {
        builder.push(if has_where { " AND " } else { " WHERE " });
        builder.push("job_type = ");
        builder.push_bind(job_type);
        has_where = true;
    }
    if let Some(subject_pubkey) = query.subject_pubkey.as_ref() {
        builder.push(if has_where { " AND " } else { " WHERE " });
        builder.push("subject_pubkey = ");
        builder.push_bind(subject_pubkey);
    }
    builder.push(" ORDER BY requested_at DESC");
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    builder.push(" LIMIT ");
    builder.push_bind(limit);

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

    let mut jobs = Vec::new();
    for row in rows {
        let requested_at: chrono::DateTime<chrono::Utc> = row.try_get("requested_at")?;
        let started_at: Option<chrono::DateTime<chrono::Utc>> = row.try_get("started_at")?;
        let completed_at: Option<chrono::DateTime<chrono::Utc>> = row.try_get("completed_at")?;
        jobs.push(TrustJobRow {
            job_id: row.try_get("job_id")?,
            job_type: row.try_get("job_type")?,
            subject_pubkey: row.try_get("subject_pubkey")?,
            status: row.try_get("status")?,
            total_targets: row.try_get("total_targets")?,
            processed_targets: row.try_get("processed_targets")?,
            requested_by: row.try_get("requested_by")?,
            requested_at: requested_at.timestamp(),
            started_at: started_at.map(|value| value.timestamp()),
            completed_at: completed_at.map(|value| value.timestamp()),
            error_message: row.try_get("error_message")?,
        });
    }

    Ok(Json(jobs))
}

pub async fn create_job(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Json(payload): Json<TrustJobRequest>,
) -> ApiResult<Json<TrustJobRow>> {
    let admin = require_admin(&state, &jar).await?;
    let job_type = normalize_job_type(payload.job_type.trim())?;
    let subject_pubkey = payload
        .subject_pubkey
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if let Some(pubkey) = subject_pubkey.as_ref() {
        if !is_hex_64(pubkey) {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_SUBJECT",
                "invalid pubkey",
            ));
        }
    }

    let job_id = Uuid::new_v4().to_string();
    let row = sqlx::query(
        "INSERT INTO cn_trust.jobs          (job_id, job_type, subject_pubkey, status, requested_by)          VALUES ($1, $2, $3, 'pending', $4)          RETURNING job_id, job_type, subject_pubkey, status, total_targets, processed_targets, requested_by, requested_at, started_at, completed_at, error_message",
    )
    .bind(&job_id)
    .bind(job_type)
    .bind(&subject_pubkey)
    .bind(&admin.admin_user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let diff = json!({
        "job_id": job_id,
        "job_type": job_type,
        "subject_pubkey": subject_pubkey
    });
    cn_core::admin::log_audit(
        &state.pool,
        &admin.admin_user_id,
        "trust.job.enqueue",
        &format!("trust:job:{job_id}"),
        Some(diff),
        None,
    )
    .await
    .ok();

    Ok(Json(map_job_row(row)?))
}

pub async fn list_schedules(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
) -> ApiResult<Json<Vec<TrustScheduleRow>>> {
    require_admin(&state, &jar).await?;
    let rows = sqlx::query(
        "SELECT job_type, interval_seconds, next_run_at, is_enabled, updated_at FROM cn_trust.job_schedules ORDER BY job_type ASC",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let mut schedules = Vec::new();
    for row in rows {
        let next_run_at: chrono::DateTime<chrono::Utc> = row.try_get("next_run_at")?;
        let updated_at: chrono::DateTime<chrono::Utc> = row.try_get("updated_at")?;
        schedules.push(TrustScheduleRow {
            job_type: row.try_get("job_type")?,
            interval_seconds: row.try_get("interval_seconds")?,
            next_run_at: next_run_at.timestamp(),
            is_enabled: row.try_get("is_enabled")?,
            updated_at: updated_at.timestamp(),
        });
    }
    Ok(Json(schedules))
}

pub async fn update_schedule(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    Path(job_type): Path<String>,
    Json(payload): Json<TrustScheduleUpdate>,
) -> ApiResult<Json<TrustScheduleRow>> {
    let admin = require_admin(&state, &jar).await?;
    let job_type = normalize_job_type(job_type.trim())?;
    if payload.interval_seconds < 60 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_INTERVAL",
            "interval_seconds must be >= 60",
        ));
    }

    let row = sqlx::query(
        "INSERT INTO cn_trust.job_schedules          (job_type, interval_seconds, next_run_at, is_enabled)          VALUES ($1, $2, NOW() + ($2 * INTERVAL '1 second'), $3)          ON CONFLICT (job_type) DO UPDATE SET interval_seconds = EXCLUDED.interval_seconds,              is_enabled = EXCLUDED.is_enabled,              next_run_at = LEAST(cn_trust.job_schedules.next_run_at, NOW() + (EXCLUDED.interval_seconds * INTERVAL '1 second')),              updated_at = NOW()          RETURNING job_type, interval_seconds, next_run_at, is_enabled, updated_at",
    )
    .bind(&job_type)
    .bind(payload.interval_seconds)
    .bind(payload.is_enabled)
    .fetch_one(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let diff = json!({
        "job_type": job_type,
        "interval_seconds": payload.interval_seconds,
        "is_enabled": payload.is_enabled
    });
    cn_core::admin::log_audit(
        &state.pool,
        &admin.admin_user_id,
        "trust.schedule.update",
        &format!("trust:schedule:{job_type}"),
        Some(diff),
        None,
    )
    .await
    .ok();

    Ok(Json(map_schedule_row(row)?))
}

fn normalize_job_type(job_type: &str) -> ApiResult<&'static str> {
    match job_type {
        JOB_REPORT_BASED => Ok(JOB_REPORT_BASED),
        JOB_COMMUNICATION => Ok(JOB_COMMUNICATION),
        _ => Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_JOB_TYPE",
            "unknown job_type",
        )),
    }
}

fn map_job_row(row: sqlx::postgres::PgRow) -> Result<TrustJobRow, ApiError> {
    let requested_at: chrono::DateTime<chrono::Utc> = row.try_get("requested_at")?;
    let started_at: Option<chrono::DateTime<chrono::Utc>> = row.try_get("started_at")?;
    let completed_at: Option<chrono::DateTime<chrono::Utc>> = row.try_get("completed_at")?;
    Ok(TrustJobRow {
        job_id: row.try_get("job_id")?,
        job_type: row.try_get("job_type")?,
        subject_pubkey: row.try_get("subject_pubkey")?,
        status: row.try_get("status")?,
        total_targets: row.try_get("total_targets")?,
        processed_targets: row.try_get("processed_targets")?,
        requested_by: row.try_get("requested_by")?,
        requested_at: requested_at.timestamp(),
        started_at: started_at.map(|value| value.timestamp()),
        completed_at: completed_at.map(|value| value.timestamp()),
        error_message: row.try_get("error_message")?,
    })
}

fn map_schedule_row(row: sqlx::postgres::PgRow) -> Result<TrustScheduleRow, ApiError> {
    let next_run_at: chrono::DateTime<chrono::Utc> = row.try_get("next_run_at")?;
    let updated_at: chrono::DateTime<chrono::Utc> = row.try_get("updated_at")?;
    Ok(TrustScheduleRow {
        job_type: row.try_get("job_type")?,
        interval_seconds: row.try_get("interval_seconds")?,
        next_run_at: next_run_at.timestamp(),
        is_enabled: row.try_get("is_enabled")?,
        updated_at: updated_at.timestamp(),
    })
}

fn is_hex_64(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit())
}
