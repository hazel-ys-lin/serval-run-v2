use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppResult;
use crate::handlers::PaginationParams;
use crate::middlewares::AuthUser;
use crate::models::{CreateEnvironment, Environment, UpdateEnvironment};
use crate::repositories::EnvironmentRepository;
use crate::state::AppState;

// ============ Request/Response DTOs ============

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateEnvironmentRequest {
    pub title: String,
    pub domain_name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateEnvironmentRequest {
    pub title: Option<String>,
    pub domain_name: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EnvironmentResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub title: String,
    pub domain_name: String,
    #[schema(value_type = String)]
    pub created_at: time::OffsetDateTime,
    #[schema(value_type = String)]
    pub updated_at: time::OffsetDateTime,
}

impl From<Environment> for EnvironmentResponse {
    fn from(e: Environment) -> Self {
        Self {
            id: e.id,
            project_id: e.project_id,
            title: e.title,
            domain_name: e.domain_name,
            created_at: e.created_at,
            updated_at: e.updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EnvironmentListResponse {
    pub data: Vec<EnvironmentResponse>,
    pub total: u64,
    pub limit: u64,
    pub offset: u64,
}

// ============ Handlers ============

/// Create a new environment in a project
#[utoipa::path(
    post,
    path = "/api/projects/{project_id}/environments",
    params(
        ("project_id" = Uuid, Path, description = "Project ID")
    ),
    request_body = CreateEnvironmentRequest,
    responses(
        (status = 201, description = "Environment created successfully", body = EnvironmentResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Project not found"),
        (status = 400, description = "Validation error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Environments"
)]
pub async fn create_environment(
    user: AuthUser,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Json(payload): Json<CreateEnvironmentRequest>,
) -> AppResult<Json<EnvironmentResponse>> {
    let create_env = CreateEnvironment {
        title: payload.title,
        domain_name: payload.domain_name,
    };

    let environment =
        EnvironmentRepository::create(&state.db, project_id, user.id, &create_env).await?;
    Ok(Json(environment.into()))
}

/// List all environments in a project
#[utoipa::path(
    get,
    path = "/api/projects/{project_id}/environments",
    params(
        ("project_id" = Uuid, Path, description = "Project ID"),
        PaginationParams
    ),
    responses(
        (status = 200, description = "List of environments", body = EnvironmentListResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Project not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Environments"
)]
pub async fn list_environments(
    user: AuthUser,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<EnvironmentListResponse>> {
    let limit = params.limit.unwrap_or(20).clamp(1, 100) as u64;
    let offset = params.offset.unwrap_or(0).max(0) as u64;

    let environments =
        EnvironmentRepository::list_by_project(&state.db, project_id, user.id, limit, offset)
            .await?;
    let total = EnvironmentRepository::count_by_project(&state.db, project_id, user.id).await?;

    Ok(Json(EnvironmentListResponse {
        data: environments.into_iter().map(|e| e.into()).collect(),
        total,
        limit,
        offset,
    }))
}

/// Get an environment by ID
#[utoipa::path(
    get,
    path = "/api/environments/{id}",
    params(
        ("id" = Uuid, Path, description = "Environment ID")
    ),
    responses(
        (status = 200, description = "Environment details", body = EnvironmentResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Environment not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Environments"
)]
pub async fn get_environment(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<EnvironmentResponse>> {
    let environment = EnvironmentRepository::find_by_id_and_user(&state.db, id, user.id).await?;
    Ok(Json(environment.into()))
}

/// Update an environment
#[utoipa::path(
    put,
    path = "/api/environments/{id}",
    params(
        ("id" = Uuid, Path, description = "Environment ID")
    ),
    request_body = UpdateEnvironmentRequest,
    responses(
        (status = 200, description = "Environment updated successfully", body = EnvironmentResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Environment not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Environments"
)]
pub async fn update_environment(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateEnvironmentRequest>,
) -> AppResult<Json<EnvironmentResponse>> {
    let update_env = UpdateEnvironment {
        title: payload.title,
        domain_name: payload.domain_name,
    };

    let environment = EnvironmentRepository::update(&state.db, id, user.id, &update_env).await?;
    Ok(Json(environment.into()))
}

/// Delete an environment
#[utoipa::path(
    delete,
    path = "/api/environments/{id}",
    params(
        ("id" = Uuid, Path, description = "Environment ID")
    ),
    responses(
        (status = 204, description = "Environment deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Environment not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Environments"
)]
pub async fn delete_environment(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<()> {
    EnvironmentRepository::delete_by_user(&state.db, id, user.id).await?;
    Ok(())
}
