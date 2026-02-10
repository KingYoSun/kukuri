use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use axum_extra::extract::cookie::CookieJar;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{Postgres, QueryBuilder, Row};
use utoipa::ToSchema;

use crate::auth::require_admin;
use crate::{ApiError, ApiResult, AppState};

const STATUS_QUEUED: &str = "queued";
const STATUS_RUNNING: &str = "running";
const STATUS_COMPLETED: &str = "completed";
const STATUS_FAILED: &str = "failed";

#[derive(Debug, Clone, Copy)]
enum DsarJobType {
    Export,
    Deletion,
}

impl DsarJobType {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "export" => Some(Self::Export),
            "deletion" => Some(Self::Deletion),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Export => "export",
            Self::Deletion => "deletion",
        }
    }
}

#[derive(Deserialize)]
pub struct DsarJobQuery {
    pub status: Option<String>,
    pub request_type: Option<String>,
    pub requester_pubkey: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Serialize, ToSchema, Clone)]
pub struct DsarJobRow {
    pub job_id: String,
    pub request_type: String,
    pub requester_pubkey: String,
    pub status: String,
    pub created_at: i64,
    pub completed_at: Option<i64>,
    pub error_message: Option<String>,
}

pub async fn list_jobs(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<DsarJobQuery>,
) -> ApiResult<Json<Vec<DsarJobRow>>> {
    require_admin(&state, &jar).await?;

    let status_filter = parse_status_filter(query.status.as_deref())?;
    let requester_filter = query
        .requester_pubkey
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let request_type = parse_job_type_filter(query.request_type.as_deref())?;
    let limit = query.limit.unwrap_or(200).clamp(1, 1000);

    let jobs = match request_type {
        Some(DsarJobType::Export) => {
            fetch_export_jobs(&state, status_filter.as_deref(), requester_filter, limit).await?
        }
        Some(DsarJobType::Deletion) => {
            fetch_deletion_jobs(&state, status_filter.as_deref(), requester_filter, limit).await?
        }
        None => {
            let mut jobs =
                fetch_export_jobs(&state, status_filter.as_deref(), requester_filter, limit)
                    .await?;
            jobs.extend(
                fetch_deletion_jobs(&state, status_filter.as_deref(), requester_filter, limit)
                    .await?,
            );
            jobs.sort_by(|left, right| {
                right
                    .created_at
                    .cmp(&left.created_at)
                    .then_with(|| left.job_id.cmp(&right.job_id))
            });
            jobs.truncate(limit as usize);
            jobs
        }
    };

    Ok(Json(jobs))
}

pub async fn retry_job(
    State(state): State<AppState>,
    jar: CookieJar,
    Path((job_type_raw, job_id)): Path<(String, String)>,
) -> ApiResult<Json<DsarJobRow>> {
    let admin = require_admin(&state, &jar).await?;
    let job_type = parse_job_type(&job_type_raw)?;
    let current = fetch_job(&state, job_type, &job_id).await?;

    if current.status == STATUS_QUEUED || current.status == STATUS_RUNNING {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "INVALID_STATE",
            "queued/running job cannot be retried",
        ));
    }

    let updated = update_job_status(&state, job_type, &job_id, STATUS_QUEUED, None).await?;
    let target = format!("dsar:{}:{}", job_type.as_str(), job_id);
    cn_core::admin::log_audit(
        &state.pool,
        &admin.admin_user_id,
        "dsar.job.retry",
        &target,
        Some(json!({
            "previous_status": current.status,
            "next_status": updated.status
        })),
        None,
    )
    .await
    .ok();

    Ok(Json(updated))
}

pub async fn cancel_job(
    State(state): State<AppState>,
    jar: CookieJar,
    Path((job_type_raw, job_id)): Path<(String, String)>,
) -> ApiResult<Json<DsarJobRow>> {
    let admin = require_admin(&state, &jar).await?;
    let job_type = parse_job_type(&job_type_raw)?;
    let current = fetch_job(&state, job_type, &job_id).await?;

    if current.status != STATUS_QUEUED && current.status != STATUS_RUNNING {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "INVALID_STATE",
            "only queued/running job can be canceled",
        ));
    }

    let updated = update_job_status(
        &state,
        job_type,
        &job_id,
        STATUS_FAILED,
        Some("canceled by admin"),
    )
    .await?;
    let target = format!("dsar:{}:{}", job_type.as_str(), job_id);
    cn_core::admin::log_audit(
        &state.pool,
        &admin.admin_user_id,
        "dsar.job.cancel",
        &target,
        Some(json!({
            "previous_status": current.status,
            "next_status": updated.status,
            "error_message": updated.error_message
        })),
        None,
    )
    .await
    .ok();

    Ok(Json(updated))
}

fn parse_job_type(raw: &str) -> ApiResult<DsarJobType> {
    DsarJobType::parse(raw).ok_or_else(|| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_REQUEST_TYPE",
            "request_type must be export|deletion",
        )
    })
}

fn parse_job_type_filter(raw: Option<&str>) -> ApiResult<Option<DsarJobType>> {
    let Some(value) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    parse_job_type(value).map(Some)
}

fn parse_status_filter(raw: Option<&str>) -> ApiResult<Option<String>> {
    let Some(value) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if !matches!(
        value,
        STATUS_QUEUED | STATUS_RUNNING | STATUS_COMPLETED | STATUS_FAILED
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_STATUS",
            "status must be queued|running|completed|failed",
        ));
    }
    Ok(Some(value.to_string()))
}

async fn fetch_export_jobs(
    state: &AppState,
    status_filter: Option<&str>,
    requester_filter: Option<&str>,
    limit: i64,
) -> ApiResult<Vec<DsarJobRow>> {
    let mut builder = QueryBuilder::<Postgres>::new(
        "SELECT export_request_id, requester_pubkey, status, created_at, completed_at, error_message \
         FROM cn_user.personal_data_export_requests WHERE 1=1",
    );

    if let Some(status_filter) = status_filter {
        builder.push(" AND status = ");
        builder.push_bind(status_filter);
    }
    if let Some(requester_filter) = requester_filter {
        builder.push(" AND requester_pubkey = ");
        builder.push_bind(requester_filter);
    }
    builder.push(" ORDER BY created_at DESC LIMIT ");
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

    let mut jobs = Vec::with_capacity(rows.len());
    for row in rows {
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
        let completed_at: Option<chrono::DateTime<chrono::Utc>> = row.try_get("completed_at")?;
        jobs.push(DsarJobRow {
            job_id: row.try_get("export_request_id")?,
            request_type: DsarJobType::Export.as_str().to_string(),
            requester_pubkey: row.try_get("requester_pubkey")?,
            status: row.try_get("status")?,
            created_at: created_at.timestamp(),
            completed_at: completed_at.map(|value| value.timestamp()),
            error_message: row.try_get("error_message").ok(),
        });
    }

    Ok(jobs)
}

async fn fetch_deletion_jobs(
    state: &AppState,
    status_filter: Option<&str>,
    requester_filter: Option<&str>,
    limit: i64,
) -> ApiResult<Vec<DsarJobRow>> {
    let mut builder = QueryBuilder::<Postgres>::new(
        "SELECT deletion_request_id, requester_pubkey, status, created_at, completed_at, error_message \
         FROM cn_user.personal_data_deletion_requests WHERE 1=1",
    );

    if let Some(status_filter) = status_filter {
        builder.push(" AND status = ");
        builder.push_bind(status_filter);
    }
    if let Some(requester_filter) = requester_filter {
        builder.push(" AND requester_pubkey = ");
        builder.push_bind(requester_filter);
    }
    builder.push(" ORDER BY created_at DESC LIMIT ");
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

    let mut jobs = Vec::with_capacity(rows.len());
    for row in rows {
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
        let completed_at: Option<chrono::DateTime<chrono::Utc>> = row.try_get("completed_at")?;
        jobs.push(DsarJobRow {
            job_id: row.try_get("deletion_request_id")?,
            request_type: DsarJobType::Deletion.as_str().to_string(),
            requester_pubkey: row.try_get("requester_pubkey")?,
            status: row.try_get("status")?,
            created_at: created_at.timestamp(),
            completed_at: completed_at.map(|value| value.timestamp()),
            error_message: row.try_get("error_message").ok(),
        });
    }

    Ok(jobs)
}

async fn fetch_job(state: &AppState, job_type: DsarJobType, job_id: &str) -> ApiResult<DsarJobRow> {
    let maybe_row = match job_type {
        DsarJobType::Export => sqlx::query(
            "SELECT export_request_id, requester_pubkey, status, created_at, completed_at, error_message \
             FROM cn_user.personal_data_export_requests WHERE export_request_id = $1",
        )
        .bind(job_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?
        .map(|row| (row, DsarJobType::Export)),
        DsarJobType::Deletion => sqlx::query(
            "SELECT deletion_request_id, requester_pubkey, status, created_at, completed_at, error_message \
             FROM cn_user.personal_data_deletion_requests WHERE deletion_request_id = $1",
        )
        .bind(job_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?
        .map(|row| (row, DsarJobType::Deletion)),
    };

    let Some((row, row_type)) = maybe_row else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "job not found",
        ));
    };

    let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
    let completed_at: Option<chrono::DateTime<chrono::Utc>> = row.try_get("completed_at")?;
    let job_id = match row_type {
        DsarJobType::Export => row.try_get("export_request_id")?,
        DsarJobType::Deletion => row.try_get("deletion_request_id")?,
    };

    Ok(DsarJobRow {
        job_id,
        request_type: row_type.as_str().to_string(),
        requester_pubkey: row.try_get("requester_pubkey")?,
        status: row.try_get("status")?,
        created_at: created_at.timestamp(),
        completed_at: completed_at.map(|value| value.timestamp()),
        error_message: row.try_get("error_message").ok(),
    })
}

async fn update_job_status(
    state: &AppState,
    job_type: DsarJobType,
    job_id: &str,
    next_status: &str,
    error_message: Option<&str>,
) -> ApiResult<DsarJobRow> {
    let updated = match job_type {
        DsarJobType::Export => sqlx::query(
            "UPDATE cn_user.personal_data_export_requests \
             SET status = $2, \
                 completed_at = CASE WHEN $2 = 'failed' OR $2 = 'completed' THEN NOW() ELSE NULL END, \
                 error_message = $3 \
             WHERE export_request_id = $1 \
             RETURNING export_request_id, requester_pubkey, status, created_at, completed_at, error_message",
        )
        .bind(job_id)
        .bind(next_status)
        .bind(error_message)
        .fetch_optional(&state.pool)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?
        .map(|row| (row, DsarJobType::Export)),
        DsarJobType::Deletion => sqlx::query(
            "UPDATE cn_user.personal_data_deletion_requests \
             SET status = $2, \
                 completed_at = CASE WHEN $2 = 'failed' OR $2 = 'completed' THEN NOW() ELSE NULL END, \
                 error_message = $3 \
             WHERE deletion_request_id = $1 \
             RETURNING deletion_request_id, requester_pubkey, status, created_at, completed_at, error_message",
        )
        .bind(job_id)
        .bind(next_status)
        .bind(error_message)
        .fetch_optional(&state.pool)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?
        .map(|row| (row, DsarJobType::Deletion)),
    };

    let Some((row, row_type)) = updated else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "job not found",
        ));
    };

    let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
    let completed_at: Option<chrono::DateTime<chrono::Utc>> = row.try_get("completed_at")?;
    let job_id = match row_type {
        DsarJobType::Export => row.try_get("export_request_id")?,
        DsarJobType::Deletion => row.try_get("deletion_request_id")?,
    };

    Ok(DsarJobRow {
        job_id,
        request_type: row_type.as_str().to_string(),
        requester_pubkey: row.try_get("requester_pubkey")?,
        status: row.try_get("status")?,
        created_at: created_at.timestamp(),
        completed_at: completed_at.map(|value| value.timestamp()),
        error_message: row.try_get("error_message").ok(),
    })
}
