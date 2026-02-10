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
use crate::models::{CreateScenario, GherkinStep, Scenario, TestExample, UpdateScenario};
use crate::repositories::mongo::MongoRepository;
use crate::repositories::ScenarioRepository;
use crate::services::GherkinService;
use crate::state::AppState;

// ============ Request/Response DTOs ============

#[derive(Debug, Deserialize, ToSchema)]
pub struct GherkinStepRequest {
    pub keyword: String,
    pub keyword_type: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_string: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_table: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TestExampleRequest {
    pub example: serde_json::Value,
    pub expected_response_body: serde_json::Value,
    pub expected_status_code: i16,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateScenarioRequest {
    pub title: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub steps: Vec<GherkinStepRequest>,
    pub examples: Vec<TestExampleRequest>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateScenarioRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub steps: Option<Vec<GherkinStepRequest>>,
    pub examples: Option<Vec<TestExampleRequest>>,
}

/// Request to parse Gherkin and create scenarios
#[derive(Debug, Deserialize, ToSchema)]
pub struct ParseGherkinRequest {
    pub gherkin_code: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GherkinStepResponse {
    pub keyword: String,
    pub keyword_type: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_string: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_table: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TestExampleResponse {
    pub example: serde_json::Value,
    pub expected_response_body: serde_json::Value,
    pub expected_status_code: i16,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ScenarioResponse {
    pub id: Uuid,
    pub api_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub steps: serde_json::Value,
    pub examples: serde_json::Value,
    #[schema(value_type = String)]
    pub created_at: time::OffsetDateTime,
    #[schema(value_type = String)]
    pub updated_at: time::OffsetDateTime,
}

impl From<Scenario> for ScenarioResponse {
    fn from(s: Scenario) -> Self {
        Self {
            id: s.id,
            api_id: s.api_id,
            title: s.title,
            description: s.description,
            tags: s.tags,
            steps: s.steps,
            examples: s.examples,
            created_at: s.created_at,
            updated_at: s.updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ScenarioListResponse {
    pub data: Vec<ScenarioResponse>,
    pub total: u64,
    pub limit: u64,
    pub offset: u64,
}

/// Response for parsed Gherkin scenarios (preview before saving)
#[derive(Debug, Serialize, ToSchema)]
pub struct ParsedScenarioResponse {
    pub title: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub steps: Vec<GherkinStepResponse>,
    pub examples: Vec<ParsedExampleResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ParsedExampleResponse {
    pub data: serde_json::Value,
    pub expected_status_code: Option<i16>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ParseGherkinResponse {
    pub feature_name: String,
    pub feature_description: Option<String>,
    /// Background steps that run before each scenario
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub background_steps: Vec<GherkinStepResponse>,
    pub scenarios: Vec<ParsedScenarioResponse>,
}

/// Response for batch create from Gherkin
#[derive(Debug, Serialize, ToSchema)]
pub struct BatchCreateResponse {
    pub created: Vec<ScenarioResponse>,
    pub count: usize,
}

// ============ Handlers ============

/// Create a new scenario for an API
#[utoipa::path(
    post,
    path = "/api/apis/{api_id}/scenarios",
    params(
        ("api_id" = Uuid, Path, description = "API ID")
    ),
    request_body = CreateScenarioRequest,
    responses(
        (status = 201, description = "Scenario created successfully", body = ScenarioResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "API not found"),
        (status = 400, description = "Validation error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Scenarios"
)]
pub async fn create_scenario(
    user: AuthUser,
    State(state): State<AppState>,
    Path(api_id): Path<Uuid>,
    Json(payload): Json<CreateScenarioRequest>,
) -> AppResult<Json<ScenarioResponse>> {
    let create_scenario = CreateScenario {
        title: payload.title,
        description: payload.description,
        tags: payload.tags,
        steps: payload
            .steps
            .into_iter()
            .map(|s| GherkinStep {
                keyword: s.keyword,
                keyword_type: s.keyword_type,
                text: s.text,
                doc_string: s.doc_string,
                data_table: s.data_table,
            })
            .collect(),
        examples: payload
            .examples
            .into_iter()
            .map(|e| TestExample {
                example: e.example,
                expected_response_body: e.expected_response_body,
                expected_status_code: e.expected_status_code,
            })
            .collect(),
    };

    let scenario = ScenarioRepository::create(&state.db, api_id, user.id, &create_scenario).await?;
    Ok(Json(scenario.into()))
}

/// List all scenarios for an API
#[utoipa::path(
    get,
    path = "/api/apis/{api_id}/scenarios",
    params(
        ("api_id" = Uuid, Path, description = "API ID"),
        PaginationParams
    ),
    responses(
        (status = 200, description = "List of scenarios", body = ScenarioListResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "API not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Scenarios"
)]
pub async fn list_scenarios(
    user: AuthUser,
    State(state): State<AppState>,
    Path(api_id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<ScenarioListResponse>> {
    let limit = params.limit.unwrap_or(20).clamp(1, 100) as u64;
    let offset = params.offset.unwrap_or(0).max(0) as u64;

    let scenarios =
        ScenarioRepository::list_by_api(&state.db, api_id, user.id, limit, offset).await?;
    let total = ScenarioRepository::count_by_api(&state.db, api_id, user.id).await?;

    Ok(Json(ScenarioListResponse {
        data: scenarios.into_iter().map(|s| s.into()).collect(),
        total,
        limit,
        offset,
    }))
}

/// Get a scenario by ID
#[utoipa::path(
    get,
    path = "/api/scenarios/{id}",
    params(
        ("id" = Uuid, Path, description = "Scenario ID")
    ),
    responses(
        (status = 200, description = "Scenario details", body = ScenarioResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Scenario not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Scenarios"
)]
pub async fn get_scenario(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ScenarioResponse>> {
    let scenario = ScenarioRepository::find_by_id_and_user(&state.db, id, user.id).await?;
    Ok(Json(scenario.into()))
}

/// Update a scenario
#[utoipa::path(
    put,
    path = "/api/scenarios/{id}",
    params(
        ("id" = Uuid, Path, description = "Scenario ID")
    ),
    request_body = UpdateScenarioRequest,
    responses(
        (status = 200, description = "Scenario updated successfully", body = ScenarioResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Scenario not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Scenarios"
)]
pub async fn update_scenario(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateScenarioRequest>,
) -> AppResult<Json<ScenarioResponse>> {
    let update_scenario = UpdateScenario {
        title: payload.title,
        description: payload.description,
        tags: payload.tags,
        steps: payload.steps.map(|steps| {
            steps
                .into_iter()
                .map(|s| GherkinStep {
                    keyword: s.keyword,
                    keyword_type: s.keyword_type,
                    text: s.text,
                    doc_string: s.doc_string,
                    data_table: s.data_table,
                })
                .collect()
        }),
        examples: payload.examples.map(|examples| {
            examples
                .into_iter()
                .map(|e| TestExample {
                    example: e.example,
                    expected_response_body: e.expected_response_body,
                    expected_status_code: e.expected_status_code,
                })
                .collect()
        }),
    };

    let scenario = ScenarioRepository::update(&state.db, id, user.id, &update_scenario).await?;
    Ok(Json(scenario.into()))
}

/// Delete a scenario
#[utoipa::path(
    delete,
    path = "/api/scenarios/{id}",
    params(
        ("id" = Uuid, Path, description = "Scenario ID")
    ),
    responses(
        (status = 204, description = "Scenario deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Scenario not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Scenarios"
)]
pub async fn delete_scenario(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<()> {
    ScenarioRepository::delete_by_user(&state.db, id, user.id).await?;
    Ok(())
}

/// Parse Gherkin code and preview scenarios (without saving)
#[utoipa::path(
    post,
    path = "/api/apis/{api_id}/scenarios/parse",
    params(
        ("api_id" = Uuid, Path, description = "API ID")
    ),
    request_body = ParseGherkinRequest,
    responses(
        (status = 200, description = "Parsed Gherkin scenarios", body = ParseGherkinResponse),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Invalid Gherkin syntax")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Scenarios"
)]
pub async fn parse_gherkin(
    _user: AuthUser,
    Path(_api_id): Path<Uuid>,
    Json(payload): Json<ParseGherkinRequest>,
) -> AppResult<Json<ParseGherkinResponse>> {
    let parsed = GherkinService::parse(&payload.gherkin_code)?;

    let scenarios = parsed
        .scenarios
        .into_iter()
        .map(|s| ParsedScenarioResponse {
            title: s.title,
            description: s.description,
            tags: s.tags,
            steps: s
                .steps
                .into_iter()
                .map(|step| GherkinStepResponse {
                    keyword: step.keyword,
                    keyword_type: step.keyword_type,
                    text: step.text,
                    doc_string: step.doc_string,
                    data_table: step.data_table,
                })
                .collect(),
            examples: s
                .examples
                .into_iter()
                .map(|e| ParsedExampleResponse {
                    data: e.data,
                    expected_status_code: e.expected_status_code,
                })
                .collect(),
        })
        .collect();

    let background_steps = parsed
        .background_steps
        .into_iter()
        .map(|step| GherkinStepResponse {
            keyword: step.keyword,
            keyword_type: step.keyword_type,
            text: step.text,
            doc_string: step.doc_string,
            data_table: step.data_table,
        })
        .collect();

    Ok(Json(ParseGherkinResponse {
        feature_name: parsed.name,
        feature_description: parsed.description,
        background_steps,
        scenarios,
    }))
}

/// Parse Gherkin code and create scenarios in batch
#[utoipa::path(
    post,
    path = "/api/apis/{api_id}/scenarios/from-gherkin",
    params(
        ("api_id" = Uuid, Path, description = "API ID")
    ),
    request_body = ParseGherkinRequest,
    responses(
        (status = 201, description = "Scenarios created from Gherkin", body = BatchCreateResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "API not found"),
        (status = 400, description = "Invalid Gherkin syntax")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Scenarios"
)]
pub async fn create_from_gherkin(
    user: AuthUser,
    State(state): State<AppState>,
    Path(api_id): Path<Uuid>,
    Json(payload): Json<ParseGherkinRequest>,
) -> AppResult<Json<BatchCreateResponse>> {
    let parsed = GherkinService::parse(&payload.gherkin_code)?;

    // Save raw Gherkin document to MongoDB
    let parsed_json = serde_json::to_value(&parsed).unwrap_or_default();
    if let Err(e) = MongoRepository::save_gherkin_document(
        &state.mongo_db(),
        api_id,
        &payload.gherkin_code,
        &parsed_json,
    )
    .await
    {
        tracing::warn!(error = %e, "Failed to save Gherkin document to MongoDB (non-fatal)");
    }

    let mut created_scenarios = Vec::new();

    for parsed_scenario in parsed.scenarios {
        let create_scenario = CreateScenario {
            title: parsed_scenario.title,
            description: parsed_scenario.description,
            tags: Some(parsed_scenario.tags),
            steps: parsed_scenario
                .steps
                .into_iter()
                .map(|s| GherkinStep {
                    keyword: s.keyword,
                    keyword_type: s.keyword_type,
                    text: s.text,
                    doc_string: s.doc_string,
                    data_table: s.data_table,
                })
                .collect(),
            examples: parsed_scenario
                .examples
                .into_iter()
                .map(|e| TestExample {
                    example: e.data,
                    expected_response_body: serde_json::Value::Null,
                    expected_status_code: e.expected_status_code.unwrap_or(200),
                })
                .collect(),
        };

        let scenario =
            ScenarioRepository::create(&state.db, api_id, user.id, &create_scenario).await?;
        created_scenarios.push(scenario.into());
    }

    let count = created_scenarios.len();
    Ok(Json(BatchCreateResponse {
        created: created_scenarios,
        count,
    }))
}
