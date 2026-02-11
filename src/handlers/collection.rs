use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppResult;
use crate::handlers::{validate_optional, validate_required, PaginationParams};
use crate::middlewares::AuthUser;
use crate::models::{Collection, CreateCollection, UpdateCollection};
use crate::repositories::CollectionRepository;
use crate::state::AppState;

// ============ Request/Response DTOs ============

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateCollectionRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateCollectionRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CollectionResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    #[schema(value_type = String)]
    pub created_at: time::OffsetDateTime,
    #[schema(value_type = String)]
    pub updated_at: time::OffsetDateTime,
}

impl From<Collection> for CollectionResponse {
    fn from(c: Collection) -> Self {
        Self {
            id: c.id,
            project_id: c.project_id,
            name: c.name,
            description: c.description,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CollectionListResponse {
    pub data: Vec<CollectionResponse>,
    pub total: u64,
    pub limit: u64,
    pub offset: u64,
}

// ============ Handlers ============

/// Create a new collection in a project
#[utoipa::path(
    post,
    path = "/api/projects/{project_id}/collections",
    params(
        ("project_id" = Uuid, Path, description = "Project ID")
    ),
    request_body = CreateCollectionRequest,
    responses(
        (status = 201, description = "Collection created successfully", body = CollectionResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Project not found"),
        (status = 400, description = "Validation error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Collections"
)]
pub async fn create_collection(
    user: AuthUser,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Json(payload): Json<CreateCollectionRequest>,
) -> AppResult<Json<CollectionResponse>> {
    validate_required(&payload.name, "Name", 100)?;
    validate_optional(&payload.description, "Description", 1000)?;

    let create_collection = CreateCollection {
        name: payload.name,
        description: payload.description,
    };

    let collection =
        CollectionRepository::create(&state.db, project_id, user.id, &create_collection).await?;
    Ok(Json(collection.into()))
}

/// List all collections in a project
#[utoipa::path(
    get,
    path = "/api/projects/{project_id}/collections",
    params(
        ("project_id" = Uuid, Path, description = "Project ID"),
        PaginationParams
    ),
    responses(
        (status = 200, description = "List of collections", body = CollectionListResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Project not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Collections"
)]
pub async fn list_collections(
    user: AuthUser,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<CollectionListResponse>> {
    let limit = params.limit.unwrap_or(20).clamp(1, 100) as u64;
    let offset = params.offset.unwrap_or(0).max(0) as u64;

    let collections =
        CollectionRepository::list_by_project(&state.db, project_id, user.id, limit, offset)
            .await?;
    let total = CollectionRepository::count_by_project(&state.db, project_id, user.id).await?;

    Ok(Json(CollectionListResponse {
        data: collections.into_iter().map(|c| c.into()).collect(),
        total,
        limit,
        offset,
    }))
}

/// Get a collection by ID
#[utoipa::path(
    get,
    path = "/api/collections/{id}",
    params(
        ("id" = Uuid, Path, description = "Collection ID")
    ),
    responses(
        (status = 200, description = "Collection details", body = CollectionResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Collection not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Collections"
)]
pub async fn get_collection(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<CollectionResponse>> {
    let collection = CollectionRepository::find_by_id_and_user(&state.db, id, user.id).await?;
    Ok(Json(collection.into()))
}

/// Update a collection
#[utoipa::path(
    put,
    path = "/api/collections/{id}",
    params(
        ("id" = Uuid, Path, description = "Collection ID")
    ),
    request_body = UpdateCollectionRequest,
    responses(
        (status = 200, description = "Collection updated successfully", body = CollectionResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Collection not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Collections"
)]
pub async fn update_collection(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateCollectionRequest>,
) -> AppResult<Json<CollectionResponse>> {
    validate_optional(&payload.name, "Name", 100)?;
    validate_optional(&payload.description, "Description", 1000)?;

    let update_collection = UpdateCollection {
        name: payload.name,
        description: payload.description,
    };

    let collection =
        CollectionRepository::update(&state.db, id, user.id, &update_collection).await?;
    Ok(Json(collection.into()))
}

/// Delete a collection
#[utoipa::path(
    delete,
    path = "/api/collections/{id}",
    params(
        ("id" = Uuid, Path, description = "Collection ID")
    ),
    responses(
        (status = 204, description = "Collection deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Collection not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Collections"
)]
pub async fn delete_collection(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<()> {
    CollectionRepository::delete_by_user(&state.db, id, user.id).await?;
    Ok(())
}
