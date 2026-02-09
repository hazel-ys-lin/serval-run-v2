use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::middlewares::AuthUser;
use crate::queue::{TestJob, TestJobConfig, TestJobType};
use crate::repositories::{ApiRepository, EnvironmentRepository, ScenarioRepository};
use crate::services::{TestConfig, TestResult, TestRunner};
use crate::state::AppState;

// ============ Request/Response DTOs ============

/// Request to run tests
#[derive(Debug, Deserialize, ToSchema)]
pub struct RunTestRequest {
    /// Environment ID to run tests against
    pub environment_id: Uuid,
    /// Optional auth token for authenticated APIs
    pub auth_token: Option<String>,
    /// Optional custom headers (key-value pairs)
    pub custom_headers: Option<std::collections::HashMap<String, String>>,
    /// Timeout in seconds (default: 30)
    pub timeout_seconds: Option<u64>,
    /// Run asynchronously via job queue (default: false for backward compatibility)
    #[serde(default)]
    pub async_execution: bool,
}

/// Single test result response
#[derive(Debug, Serialize, ToSchema)]
pub struct TestResultResponse {
    pub scenario_id: Uuid,
    pub api_id: Uuid,
    pub example_index: i32,
    pub pass: bool,
    pub error_message: Option<String>,
    pub response_status: i16,
    pub response_data: Option<serde_json::Value>,
    pub request_duration_ms: i64,
    #[schema(value_type = String)]
    pub request_time: time::OffsetDateTime,
}

impl From<TestResult> for TestResultResponse {
    fn from(r: TestResult) -> Self {
        Self {
            scenario_id: r.scenario_id,
            api_id: r.api_id,
            example_index: r.example_index,
            pass: r.pass,
            error_message: r.error_message,
            response_status: r.response_status,
            response_data: r.response_data,
            request_duration_ms: r.request_duration_ms,
            request_time: r.request_time,
        }
    }
}

/// Test run summary (sync execution)
#[derive(Debug, Serialize, ToSchema)]
pub struct TestRunResponse {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub pass_rate: f64,
    pub total_duration_ms: i64,
    pub results: Vec<TestResultResponse>,
}

/// Response for async test execution
#[derive(Debug, Serialize, ToSchema)]
pub struct AsyncTestResponse {
    /// Job ID for tracking
    pub job_id: Uuid,
    /// Job status
    pub status: String,
    /// Message
    pub message: String,
}

/// Union response type for test execution
#[derive(Debug, Serialize, ToSchema)]
#[serde(untagged)]
pub enum TestExecutionResponse {
    Sync(TestRunResponse),
    Async(AsyncTestResponse),
}

// ============ Handlers ============

/// Run tests for a single scenario
#[utoipa::path(
    post,
    path = "/api/scenarios/{scenario_id}/run",
    params(
        ("scenario_id" = Uuid, Path, description = "Scenario ID")
    ),
    request_body = RunTestRequest,
    responses(
        (status = 200, description = "Test execution completed or job queued", body = TestRunResponse),
        (status = 202, description = "Test job queued for async execution", body = AsyncTestResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Scenario or Environment not found"),
        (status = 400, description = "Validation error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Test Execution"
)]
pub async fn run_scenario_test(
    user: AuthUser,
    State(state): State<AppState>,
    Path(scenario_id): Path<Uuid>,
    Json(payload): Json<RunTestRequest>,
) -> AppResult<Json<serde_json::Value>> {
    // Validate scenario ownership
    let scenario =
        ScenarioRepository::find_by_id_and_user(&state.db, scenario_id, user.id).await?;

    // Validate API ownership
    let api = ApiRepository::find_by_id_and_user(&state.db, scenario.api_id, user.id).await?;

    // Validate environment ownership
    let environment =
        EnvironmentRepository::find_by_id_and_user(&state.db, payload.environment_id, user.id)
            .await?;

    if payload.async_execution {
        // Async execution - enqueue job
        let config = TestJobConfig {
            timeout_seconds: payload.timeout_seconds.unwrap_or(30),
            auth_token: payload.auth_token,
            custom_headers: payload.custom_headers.unwrap_or_default(),
        };

        let job = TestJob::new(
            TestJobType::Scenario,
            scenario_id,
            payload.environment_id,
            user.id,
            config,
        );

        let job_id = state.job_queue.enqueue(job).await?;

        let response = AsyncTestResponse {
            job_id,
            status: "pending".to_string(),
            message: "Test job queued successfully".to_string(),
        };

        Ok(Json(serde_json::to_value(response).map_err(|e| AppError::Internal(e.to_string()))?))
    } else {
        // Sync execution - run immediately
        let config = TestConfig {
            timeout: std::time::Duration::from_secs(payload.timeout_seconds.unwrap_or(30)),
            auth_token: payload.auth_token,
            custom_headers: payload.custom_headers.unwrap_or_default(),
        };

        let runner = TestRunner::with_config(config);
        let results = runner.run_scenario(&scenario, &api, &environment).await?;

        let response = build_test_run_response(results);
        Ok(Json(serde_json::to_value(response).map_err(|e| AppError::Internal(e.to_string()))?))
    }
}

/// Run tests for all scenarios under an API
#[utoipa::path(
    post,
    path = "/api/apis/{api_id}/run",
    params(
        ("api_id" = Uuid, Path, description = "API ID")
    ),
    request_body = RunTestRequest,
    responses(
        (status = 200, description = "Test execution completed or job queued", body = TestRunResponse),
        (status = 202, description = "Test job queued for async execution", body = AsyncTestResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "API or Environment not found"),
        (status = 400, description = "Validation error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Test Execution"
)]
pub async fn run_api_tests(
    user: AuthUser,
    State(state): State<AppState>,
    Path(api_id): Path<Uuid>,
    Json(payload): Json<RunTestRequest>,
) -> AppResult<Json<serde_json::Value>> {
    // Validate API ownership
    let api = ApiRepository::find_by_id_and_user(&state.db, api_id, user.id).await?;

    // Validate environment ownership
    let environment =
        EnvironmentRepository::find_by_id_and_user(&state.db, payload.environment_id, user.id)
            .await?;

    // Get all scenarios for this API (for validation in sync mode)
    let scenarios = ScenarioRepository::list_by_api(&state.db, api_id, user.id, 1000, 0).await?;

    if scenarios.is_empty() {
        return Err(AppError::Validation(
            "No scenarios found for this API".to_string(),
        ));
    }

    if payload.async_execution {
        // Async execution - enqueue job
        let config = TestJobConfig {
            timeout_seconds: payload.timeout_seconds.unwrap_or(30),
            auth_token: payload.auth_token,
            custom_headers: payload.custom_headers.unwrap_or_default(),
        };

        let job = TestJob::new(
            TestJobType::Api,
            api_id,
            payload.environment_id,
            user.id,
            config,
        );

        let job_id = state.job_queue.enqueue(job).await?;

        let response = AsyncTestResponse {
            job_id,
            status: "pending".to_string(),
            message: "Test job queued successfully".to_string(),
        };

        Ok(Json(serde_json::to_value(response).map_err(|e| AppError::Internal(e.to_string()))?))
    } else {
        // Sync execution - run immediately
        let config = TestConfig {
            timeout: std::time::Duration::from_secs(payload.timeout_seconds.unwrap_or(30)),
            auth_token: payload.auth_token,
            custom_headers: payload.custom_headers.unwrap_or_default(),
        };

        let runner = TestRunner::with_config(config);
        let mut all_results = Vec::new();

        for scenario in &scenarios {
            let results = runner.run_scenario(scenario, &api, &environment).await?;
            all_results.extend(results);
        }

        let response = build_test_run_response(all_results);
        Ok(Json(serde_json::to_value(response).map_err(|e| AppError::Internal(e.to_string()))?))
    }
}

/// Run tests for all APIs in a collection
#[utoipa::path(
    post,
    path = "/api/collections/{collection_id}/run",
    params(
        ("collection_id" = Uuid, Path, description = "Collection ID")
    ),
    request_body = RunTestRequest,
    responses(
        (status = 200, description = "Test execution completed or job queued", body = TestRunResponse),
        (status = 202, description = "Test job queued for async execution", body = AsyncTestResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Collection or Environment not found"),
        (status = 400, description = "Validation error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Test Execution"
)]
pub async fn run_collection_tests(
    user: AuthUser,
    State(state): State<AppState>,
    Path(collection_id): Path<Uuid>,
    Json(payload): Json<RunTestRequest>,
) -> AppResult<Json<serde_json::Value>> {
    use crate::repositories::CollectionRepository;

    // Verify collection ownership
    let _collection =
        CollectionRepository::find_by_id_and_user(&state.db, collection_id, user.id).await?;

    // Validate environment ownership
    let environment =
        EnvironmentRepository::find_by_id_and_user(&state.db, payload.environment_id, user.id)
            .await?;

    // Get all APIs for this collection
    let apis =
        ApiRepository::list_by_collection(&state.db, collection_id, user.id, 1000, 0).await?;

    if apis.is_empty() {
        return Err(AppError::Validation(
            "No APIs found for this collection".to_string(),
        ));
    }

    if payload.async_execution {
        // Async execution - enqueue job
        let config = TestJobConfig {
            timeout_seconds: payload.timeout_seconds.unwrap_or(30),
            auth_token: payload.auth_token,
            custom_headers: payload.custom_headers.unwrap_or_default(),
        };

        let job = TestJob::new(
            TestJobType::Collection,
            collection_id,
            payload.environment_id,
            user.id,
            config,
        );

        let job_id = state.job_queue.enqueue(job).await?;

        let response = AsyncTestResponse {
            job_id,
            status: "pending".to_string(),
            message: "Test job queued successfully".to_string(),
        };

        Ok(Json(serde_json::to_value(response).map_err(|e| AppError::Internal(e.to_string()))?))
    } else {
        // Sync execution - run immediately
        let config = TestConfig {
            timeout: std::time::Duration::from_secs(payload.timeout_seconds.unwrap_or(30)),
            auth_token: payload.auth_token.clone(),
            custom_headers: payload.custom_headers.clone().unwrap_or_default(),
        };

        let runner = TestRunner::with_config(config);
        let mut all_results = Vec::new();

        for api in &apis {
            let scenarios =
                ScenarioRepository::list_by_api(&state.db, api.id, user.id, 1000, 0).await?;

            for scenario in &scenarios {
                let results = runner.run_scenario(scenario, api, &environment).await?;
                all_results.extend(results);
            }
        }

        if all_results.is_empty() {
            return Err(AppError::Validation(
                "No scenarios found for any API in this collection".to_string(),
            ));
        }

        let response = build_test_run_response(all_results);
        Ok(Json(serde_json::to_value(response).map_err(|e| AppError::Internal(e.to_string()))?))
    }
}

/// Helper to build test run response from results
fn build_test_run_response(results: Vec<TestResult>) -> TestRunResponse {
    let total = results.len();
    let passed = results.iter().filter(|r| r.pass).count();
    let failed = total - passed;
    let pass_rate = if total > 0 {
        (passed as f64 / total as f64) * 100.0
    } else {
        0.0
    };
    let total_duration_ms: i64 = results.iter().map(|r| r.request_duration_ms).sum();

    TestRunResponse {
        total,
        passed,
        failed,
        pass_rate,
        total_duration_ms,
        results: results.into_iter().map(|r| r.into()).collect(),
    }
}
