use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::handlers::PaginationParams;
use crate::middlewares::AuthUser;
use crate::state::AppState;

// ============ Response DTOs ============

/// Job status response
#[derive(Debug, Serialize, ToSchema)]
pub struct JobStatusResponse {
    pub job_id: Uuid,
    pub job_type: String,
    pub target_id: Uuid,
    pub environment_id: Uuid,
    pub status: String,
    pub retry_count: u32,
    pub max_retries: u32,
    #[schema(value_type = String)]
    pub created_at: time::OffsetDateTime,
    #[schema(value_type = Option<String>)]
    pub started_at: Option<time::OffsetDateTime>,
    #[schema(value_type = Option<String>)]
    pub completed_at: Option<time::OffsetDateTime>,
    pub error_message: Option<String>,
    pub report_id: Option<Uuid>,
}

/// Job list response
#[derive(Debug, Serialize, ToSchema)]
pub struct JobListResponse {
    pub data: Vec<JobStatusResponse>,
    pub total: u64,
    pub limit: u64,
}

/// Queue statistics
#[derive(Debug, Serialize, ToSchema)]
pub struct QueueStatsResponse {
    pub queue_length: u64,
}

// ============ Handlers ============

/// Get job status by ID
#[utoipa::path(
    get,
    path = "/api/jobs/{job_id}",
    params(
        ("job_id" = Uuid, Path, description = "Job ID")
    ),
    responses(
        (status = 200, description = "Job status", body = JobStatusResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Job not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Jobs"
)]
pub async fn get_job_status(
    user: AuthUser,
    State(state): State<AppState>,
    Path(job_id): Path<Uuid>,
) -> AppResult<Json<JobStatusResponse>> {
    let job = state
        .job_queue
        .get_job(job_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Job".to_string()))?;

    // Verify ownership
    if job.user_id != user.id {
        return Err(AppError::NotFound("Job".to_string()));
    }

    Ok(Json(JobStatusResponse {
        job_id: job.id,
        job_type: job.job_type.as_str().to_string(),
        target_id: job.target_id,
        environment_id: job.environment_id,
        status: job.status.as_str().to_string(),
        retry_count: job.retry_count,
        max_retries: job.max_retries,
        created_at: job.created_at,
        started_at: job.started_at,
        completed_at: job.completed_at,
        error_message: job.error_message,
        report_id: job.report_id,
    }))
}

/// List jobs for the current user
#[utoipa::path(
    get,
    path = "/api/jobs",
    params(
        PaginationParams
    ),
    responses(
        (status = 200, description = "List of jobs", body = JobListResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Jobs"
)]
pub async fn list_jobs(
    user: AuthUser,
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<JobListResponse>> {
    let limit = params.limit.unwrap_or(20).min(100).max(1) as u64;

    let jobs = state.job_queue.list_jobs_by_user(user.id, limit).await?;

    let data: Vec<JobStatusResponse> = jobs
        .into_iter()
        .map(|job| JobStatusResponse {
            job_id: job.id,
            job_type: job.job_type.as_str().to_string(),
            target_id: job.target_id,
            environment_id: job.environment_id,
            status: job.status.as_str().to_string(),
            retry_count: job.retry_count,
            max_retries: job.max_retries,
            created_at: job.created_at,
            started_at: job.started_at,
            completed_at: job.completed_at,
            error_message: job.error_message,
            report_id: job.report_id,
        })
        .collect();

    let total = data.len() as u64;

    Ok(Json(JobListResponse { data, total, limit }))
}

/// Cancel a job
#[utoipa::path(
    delete,
    path = "/api/jobs/{job_id}",
    params(
        ("job_id" = Uuid, Path, description = "Job ID")
    ),
    responses(
        (status = 200, description = "Job cancelled", body = JobStatusResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Job not found"),
        (status = 400, description = "Cannot cancel completed job")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Jobs"
)]
pub async fn cancel_job(
    user: AuthUser,
    State(state): State<AppState>,
    Path(job_id): Path<Uuid>,
) -> AppResult<Json<JobStatusResponse>> {
    let job = state
        .job_queue
        .get_job(job_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Job".to_string()))?;

    // Verify ownership
    if job.user_id != user.id {
        return Err(AppError::NotFound("Job".to_string()));
    }

    // Cancel the job
    state.job_queue.cancel_job(job_id).await?;

    // Get updated job
    let job = state
        .job_queue
        .get_job(job_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Job".to_string()))?;

    Ok(Json(JobStatusResponse {
        job_id: job.id,
        job_type: job.job_type.as_str().to_string(),
        target_id: job.target_id,
        environment_id: job.environment_id,
        status: job.status.as_str().to_string(),
        retry_count: job.retry_count,
        max_retries: job.max_retries,
        created_at: job.created_at,
        started_at: job.started_at,
        completed_at: job.completed_at,
        error_message: job.error_message,
        report_id: job.report_id,
    }))
}

/// Requeue a failed job
#[utoipa::path(
    post,
    path = "/api/jobs/{job_id}/requeue",
    params(
        ("job_id" = Uuid, Path, description = "Job ID")
    ),
    responses(
        (status = 200, description = "Job requeued", body = JobStatusResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Job not found"),
        (status = 400, description = "Only failed jobs can be requeued")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Jobs"
)]
pub async fn requeue_job(
    user: AuthUser,
    State(state): State<AppState>,
    Path(job_id): Path<Uuid>,
) -> AppResult<Json<JobStatusResponse>> {
    let job = state
        .job_queue
        .get_job(job_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Job".to_string()))?;

    // Verify ownership
    if job.user_id != user.id {
        return Err(AppError::NotFound("Job".to_string()));
    }

    // Requeue the job
    state.job_queue.requeue(job_id).await?;

    // Get updated job
    let job = state
        .job_queue
        .get_job(job_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Job".to_string()))?;

    Ok(Json(JobStatusResponse {
        job_id: job.id,
        job_type: job.job_type.as_str().to_string(),
        target_id: job.target_id,
        environment_id: job.environment_id,
        status: job.status.as_str().to_string(),
        retry_count: job.retry_count,
        max_retries: job.max_retries,
        created_at: job.created_at,
        started_at: job.started_at,
        completed_at: job.completed_at,
        error_message: job.error_message,
        report_id: job.report_id,
    }))
}

/// Get queue statistics
#[utoipa::path(
    get,
    path = "/api/jobs/stats",
    responses(
        (status = 200, description = "Queue statistics", body = QueueStatsResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Jobs"
)]
pub async fn get_queue_stats(
    _user: AuthUser,
    State(state): State<AppState>,
) -> AppResult<Json<QueueStatsResponse>> {
    let queue_length = state.job_queue.queue_length().await?;

    Ok(Json(QueueStatsResponse { queue_length }))
}
