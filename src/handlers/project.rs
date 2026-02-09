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
use crate::models::{CreateProject, Project, UpdateProject};
use crate::repositories::ProjectRepository;
use crate::state::AppState;

// ============ Request/Response DTOs ============

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProjectResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    #[schema(value_type = String)]
    pub created_at: time::OffsetDateTime,
    #[schema(value_type = String)]
    pub updated_at: time::OffsetDateTime,
}

impl From<Project> for ProjectResponse {
    fn from(p: Project) -> Self {
        Self {
            id: p.id,
            user_id: p.user_id,
            name: p.name,
            description: p.description,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProjectListResponse {
    pub data: Vec<ProjectResponse>,
    pub total: u64,
    pub limit: u64,
    pub offset: u64,
}

// ============ Handlers ============

/// Create a new project
#[utoipa::path(
    post,
    path = "/api/projects",
    request_body = CreateProjectRequest,
    responses(
        (status = 201, description = "Project created successfully", body = ProjectResponse),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Validation error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Projects"
)]
pub async fn create_project(
    user: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateProjectRequest>,
) -> AppResult<Json<ProjectResponse>> {
    let create_project = CreateProject {
        name: payload.name,
        description: payload.description,
    };

    let project = ProjectRepository::create(&state.db, user.id, &create_project).await?;
    Ok(Json(project.into()))
}

/// List all projects for the current user
#[utoipa::path(
    get,
    path = "/api/projects",
    params(PaginationParams),
    responses(
        (status = 200, description = "List of projects", body = ProjectListResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Projects"
)]
pub async fn list_projects(
    user: AuthUser,
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<ProjectListResponse>> {
    let limit = params.limit.unwrap_or(20).min(100).max(1) as u64;
    let offset = params.offset.unwrap_or(0).max(0) as u64;

    let projects = ProjectRepository::list_by_user(&state.db, user.id, limit, offset).await?;
    let total = ProjectRepository::count_by_user(&state.db, user.id).await?;

    Ok(Json(ProjectListResponse {
        data: projects.into_iter().map(|p| p.into()).collect(),
        total,
        limit,
        offset,
    }))
}

/// Get a project by ID
#[utoipa::path(
    get,
    path = "/api/projects/{id}",
    params(
        ("id" = Uuid, Path, description = "Project ID")
    ),
    responses(
        (status = 200, description = "Project details", body = ProjectResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Project not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Projects"
)]
pub async fn get_project(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ProjectResponse>> {
    let project = ProjectRepository::find_by_id_and_user(&state.db, id, user.id).await?;
    Ok(Json(project.into()))
}

/// Update a project
#[utoipa::path(
    put,
    path = "/api/projects/{id}",
    params(
        ("id" = Uuid, Path, description = "Project ID")
    ),
    request_body = UpdateProjectRequest,
    responses(
        (status = 200, description = "Project updated successfully", body = ProjectResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Project not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Projects"
)]
pub async fn update_project(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateProjectRequest>,
) -> AppResult<Json<ProjectResponse>> {
    let update_project = UpdateProject {
        name: payload.name,
        description: payload.description,
    };

    let project = ProjectRepository::update(&state.db, id, user.id, &update_project).await?;
    Ok(Json(project.into()))
}

/// Delete a project
#[utoipa::path(
    delete,
    path = "/api/projects/{id}",
    params(
        ("id" = Uuid, Path, description = "Project ID")
    ),
    responses(
        (status = 204, description = "Project deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Project not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Projects"
)]
pub async fn delete_project(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<()> {
    ProjectRepository::delete_by_user(&state.db, id, user.id).await?;
    Ok(())
}
