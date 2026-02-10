use axum::{
    extract::{Path, Query, State},
    Json,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppResult;
use crate::handlers::PaginationParams;
use crate::middlewares::AuthUser;
use crate::models::{CreateReport, Report};
use crate::repositories::{ReportRepository, ResponseRepository};
use crate::state::AppState;

// ============ Request/Response DTOs ============

/// Request to create a new report
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateReportRequest {
    pub environment_id: Uuid,
    pub collection_id: Option<Uuid>,
    /// 1 = collection level, 2 = project level
    pub report_level: i16,
    pub report_type: Option<String>,
}

/// Report response DTO
#[derive(Debug, Serialize, ToSchema)]
pub struct ReportResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub environment_id: Uuid,
    pub collection_id: Option<Uuid>,
    pub report_level: i16,
    pub report_type: Option<String>,
    pub finished: bool,
    pub calculated: bool,
    #[schema(value_type = Option<f64>)]
    pub pass_rate: Option<Decimal>,
    pub response_count: i32,
    #[schema(value_type = String)]
    pub created_at: time::OffsetDateTime,
    #[schema(value_type = Option<String>)]
    pub finished_at: Option<time::OffsetDateTime>,
}

impl From<Report> for ReportResponse {
    fn from(r: Report) -> Self {
        Self {
            id: r.id,
            project_id: r.project_id,
            environment_id: r.environment_id,
            collection_id: r.collection_id,
            report_level: r.report_level,
            report_type: r.report_type,
            finished: r.finished,
            calculated: r.calculated,
            pass_rate: r.pass_rate,
            response_count: r.response_count,
            created_at: r.created_at,
            finished_at: r.finished_at,
        }
    }
}

/// Report list response
#[derive(Debug, Serialize, ToSchema)]
pub struct ReportListResponse {
    pub data: Vec<ReportResponse>,
    pub total: u64,
    pub limit: u64,
    pub offset: u64,
}

/// Report detail with responses
#[derive(Debug, Serialize, ToSchema)]
pub struct ReportDetailResponse {
    #[serde(flatten)]
    pub report: ReportResponse,
    pub responses: Vec<ResponseSummary>,
}

/// Response summary for report detail
#[derive(Debug, Serialize, ToSchema)]
pub struct ResponseSummary {
    pub id: Uuid,
    pub api_id: Uuid,
    pub scenario_id: Uuid,
    pub example_index: i32,
    pub response_status: i16,
    pub pass: bool,
    pub error_message: Option<String>,
    pub request_duration_ms: Option<i32>,
}

// ============ Handlers ============

/// Create a new report
#[utoipa::path(
    post,
    path = "/api/projects/{project_id}/reports",
    params(
        ("project_id" = Uuid, Path, description = "Project ID")
    ),
    request_body = CreateReportRequest,
    responses(
        (status = 201, description = "Report created successfully", body = ReportResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Project not found"),
        (status = 400, description = "Validation error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Reports"
)]
pub async fn create_report(
    user: AuthUser,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Json(payload): Json<CreateReportRequest>,
) -> AppResult<Json<ReportResponse>> {
    let create_report = CreateReport {
        environment_id: payload.environment_id,
        collection_id: payload.collection_id,
        report_level: payload.report_level,
        report_type: payload.report_type,
    };

    let report = ReportRepository::create(&state.db, project_id, user.id, &create_report).await?;

    Ok(Json(report.into()))
}

/// List reports for a project
#[utoipa::path(
    get,
    path = "/api/projects/{project_id}/reports",
    params(
        ("project_id" = Uuid, Path, description = "Project ID"),
        PaginationParams
    ),
    responses(
        (status = 200, description = "List of reports", body = ReportListResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Project not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Reports"
)]
pub async fn list_reports(
    user: AuthUser,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<ReportListResponse>> {
    let limit = params.limit.unwrap_or(20).clamp(1, 100) as u64;
    let offset = params.offset.unwrap_or(0) as u64;

    let reports = ReportRepository::list_by_project(&state.db, project_id, user.id, limit, offset).await?;
    let total = ReportRepository::count_by_project(&state.db, project_id, user.id).await?;

    let data = reports.into_iter().map(|r| r.into()).collect();

    Ok(Json(ReportListResponse {
        data,
        total,
        limit,
        offset,
    }))
}

/// Get a report by ID
#[utoipa::path(
    get,
    path = "/api/reports/{id}",
    params(
        ("id" = Uuid, Path, description = "Report ID")
    ),
    responses(
        (status = 200, description = "Report found", body = ReportResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Report not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Reports"
)]
pub async fn get_report(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ReportResponse>> {
    let report = ReportRepository::find_by_id_and_user(&state.db, id, user.id).await?;

    Ok(Json(report.into()))
}

/// Get report detail with responses
#[utoipa::path(
    get,
    path = "/api/reports/{id}/detail",
    params(
        ("id" = Uuid, Path, description = "Report ID")
    ),
    responses(
        (status = 200, description = "Report detail with responses", body = ReportDetailResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Report not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Reports"
)]
pub async fn get_report_detail(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ReportDetailResponse>> {
    let report = ReportRepository::find_by_id_and_user(&state.db, id, user.id).await?;
    let responses = ResponseRepository::list_by_report(&state.db, id, user.id, 1000, 0).await?;

    let response_summaries: Vec<ResponseSummary> = responses
        .into_iter()
        .map(|r| ResponseSummary {
            id: r.id,
            api_id: r.api_id,
            scenario_id: r.scenario_id,
            example_index: r.example_index,
            response_status: r.response_status,
            pass: r.pass,
            error_message: r.error_message,
            request_duration_ms: r.request_duration_ms,
        })
        .collect();

    Ok(Json(ReportDetailResponse {
        report: report.into(),
        responses: response_summaries,
    }))
}

/// Delete a report
#[utoipa::path(
    delete,
    path = "/api/reports/{id}",
    params(
        ("id" = Uuid, Path, description = "Report ID")
    ),
    responses(
        (status = 204, description = "Report deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Report not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Reports"
)]
pub async fn delete_report(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<()> {
    // Delete all responses first
    ResponseRepository::delete_by_report(&state.db, id, user.id).await?;
    // Then delete the report
    ReportRepository::delete_by_user(&state.db, id, user.id).await?;

    Ok(())
}
