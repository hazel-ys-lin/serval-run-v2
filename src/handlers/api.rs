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
use crate::models::{Api, CreateApi, UpdateApi};
use crate::repositories::ApiRepository;
use crate::state::AppState;

// ============ Request/Response DTOs ============

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateApiRequest {
    pub name: String,
    pub http_method: String,
    pub endpoint: String,
    pub severity: Option<i16>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateApiRequest {
    pub name: Option<String>,
    pub http_method: Option<String>,
    pub endpoint: Option<String>,
    pub severity: Option<i16>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiResponse {
    pub id: Uuid,
    pub collection_id: Uuid,
    pub name: String,
    pub http_method: String,
    pub endpoint: String,
    pub severity: i16,
    pub description: Option<String>,
    #[schema(value_type = String)]
    pub created_at: time::OffsetDateTime,
    #[schema(value_type = String)]
    pub updated_at: time::OffsetDateTime,
}

impl From<Api> for ApiResponse {
    fn from(a: Api) -> Self {
        Self {
            id: a.id,
            collection_id: a.collection_id,
            name: a.name,
            http_method: a.http_method,
            endpoint: a.endpoint,
            severity: a.severity,
            description: a.description,
            created_at: a.created_at,
            updated_at: a.updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiListResponse {
    pub data: Vec<ApiResponse>,
    pub total: u64,
    pub limit: u64,
    pub offset: u64,
}

// ============ Handlers ============

/// Create a new API in a collection
#[utoipa::path(
    post,
    path = "/api/collections/{collection_id}/apis",
    params(
        ("collection_id" = Uuid, Path, description = "Collection ID")
    ),
    request_body = CreateApiRequest,
    responses(
        (status = 201, description = "API created successfully", body = ApiResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Collection not found"),
        (status = 400, description = "Validation error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "APIs"
)]
pub async fn create_api(
    user: AuthUser,
    State(state): State<AppState>,
    Path(collection_id): Path<Uuid>,
    Json(payload): Json<CreateApiRequest>,
) -> AppResult<Json<ApiResponse>> {
    let create_api = CreateApi {
        name: payload.name,
        http_method: payload.http_method,
        endpoint: payload.endpoint,
        severity: payload.severity,
        description: payload.description,
    };

    let api = ApiRepository::create(&state.db, collection_id, user.id, &create_api).await?;
    Ok(Json(api.into()))
}

/// List all APIs in a collection
#[utoipa::path(
    get,
    path = "/api/collections/{collection_id}/apis",
    params(
        ("collection_id" = Uuid, Path, description = "Collection ID"),
        PaginationParams
    ),
    responses(
        (status = 200, description = "List of APIs", body = ApiListResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Collection not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "APIs"
)]
pub async fn list_apis(
    user: AuthUser,
    State(state): State<AppState>,
    Path(collection_id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<ApiListResponse>> {
    let limit = params.limit.unwrap_or(20).clamp(1, 100) as u64;
    let offset = params.offset.unwrap_or(0).max(0) as u64;

    let apis =
        ApiRepository::list_by_collection(&state.db, collection_id, user.id, limit, offset).await?;
    let total = ApiRepository::count_by_collection(&state.db, collection_id, user.id).await?;

    Ok(Json(ApiListResponse {
        data: apis.into_iter().map(|a| a.into()).collect(),
        total,
        limit,
        offset,
    }))
}

/// Get an API by ID
#[utoipa::path(
    get,
    path = "/api/apis/{id}",
    params(
        ("id" = Uuid, Path, description = "API ID")
    ),
    responses(
        (status = 200, description = "API details", body = ApiResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "API not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "APIs"
)]
pub async fn get_api(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ApiResponse>> {
    let api = ApiRepository::find_by_id_and_user(&state.db, id, user.id).await?;
    Ok(Json(api.into()))
}

/// Update an API
#[utoipa::path(
    put,
    path = "/api/apis/{id}",
    params(
        ("id" = Uuid, Path, description = "API ID")
    ),
    request_body = UpdateApiRequest,
    responses(
        (status = 200, description = "API updated successfully", body = ApiResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "API not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "APIs"
)]
pub async fn update_api(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateApiRequest>,
) -> AppResult<Json<ApiResponse>> {
    let update_api = UpdateApi {
        name: payload.name,
        http_method: payload.http_method,
        endpoint: payload.endpoint,
        severity: payload.severity,
        description: payload.description,
    };

    let api = ApiRepository::update(&state.db, id, user.id, &update_api).await?;
    Ok(Json(api.into()))
}

/// Delete an API
#[utoipa::path(
    delete,
    path = "/api/apis/{id}",
    params(
        ("id" = Uuid, Path, description = "API ID")
    ),
    responses(
        (status = 204, description = "API deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "API not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "APIs"
)]
pub async fn delete_api(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<()> {
    ApiRepository::delete_by_user(&state.db, id, user.id).await?;
    Ok(())
}
