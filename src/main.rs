use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_swagger_ui::SwaggerUi;

use serval_run::config::Config;
use serval_run::handlers::{
    ApiListResponse, ApiResponse, AsyncTestResponse, AuthResponse, BatchCreateResponse,
    CollectionListResponse, CollectionResponse, CreateApiRequest, CreateCollectionRequest,
    CreateEnvironmentRequest, CreateProjectRequest, CreateReportRequest, CreateScenarioRequest,
    EnvironmentListResponse, EnvironmentResponse, JobListResponse, JobStatusResponse, LoginRequest,
    ParseGherkinRequest, ParseGherkinResponse, ProjectListResponse, ProjectResponse,
    QueueStatsResponse, RegisterRequest, ReportDetailResponse, ReportListResponse, ReportResponse,
    ResponseSummary, RunTestRequest, ScenarioListResponse, ScenarioResponse, TestResultResponse,
    TestRunResponse, UpdateApiRequest, UpdateCollectionRequest, UpdateEnvironmentRequest,
    UpdateProjectRequest, UpdateScenarioRequest, UpdateUserRequest,
};
use serval_run::models::UserResponse;
use serval_run::state::AppState;
use serval_run::{build_router, handlers};

/// Security scheme for Bearer token
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::auth::register,
        handlers::auth::login,
        handlers::auth::me,
        handlers::auth::update_me,
        handlers::project::create_project,
        handlers::project::list_projects,
        handlers::project::get_project,
        handlers::project::update_project,
        handlers::project::delete_project,
        handlers::collection::create_collection,
        handlers::collection::list_collections,
        handlers::collection::get_collection,
        handlers::collection::update_collection,
        handlers::collection::delete_collection,
        handlers::environment::create_environment,
        handlers::environment::list_environments,
        handlers::environment::get_environment,
        handlers::environment::update_environment,
        handlers::environment::delete_environment,
        handlers::api::create_api,
        handlers::api::list_apis,
        handlers::api::get_api,
        handlers::api::update_api,
        handlers::api::delete_api,
        handlers::scenario::create_scenario,
        handlers::scenario::list_scenarios,
        handlers::scenario::get_scenario,
        handlers::scenario::update_scenario,
        handlers::scenario::delete_scenario,
        handlers::scenario::parse_gherkin,
        handlers::scenario::create_from_gherkin,
        handlers::test_run::run_scenario_test,
        handlers::test_run::run_api_tests,
        handlers::test_run::run_collection_tests,
        handlers::job::get_job_status,
        handlers::job::list_jobs,
        handlers::job::cancel_job,
        handlers::job::requeue_job,
        handlers::job::get_queue_stats,
        handlers::report::create_report,
        handlers::report::list_reports,
        handlers::report::get_report,
        handlers::report::get_report_detail,
        handlers::report::delete_report,
    ),
    components(schemas(
        RegisterRequest,
        LoginRequest,
        AuthResponse,
        UserResponse,
        UpdateUserRequest,
        CreateProjectRequest,
        ProjectListResponse,
        ProjectResponse,
        UpdateProjectRequest,
        CreateCollectionRequest,
        CollectionListResponse,
        CollectionResponse,
        UpdateCollectionRequest,
        CreateEnvironmentRequest,
        EnvironmentListResponse,
        EnvironmentResponse,
        UpdateEnvironmentRequest,
        CreateApiRequest,
        ApiListResponse,
        ApiResponse,
        UpdateApiRequest,
        CreateScenarioRequest,
        ScenarioListResponse,
        ScenarioResponse,
        UpdateScenarioRequest,
        ParseGherkinRequest,
        ParseGherkinResponse,
        BatchCreateResponse,
        RunTestRequest,
        TestResultResponse,
        TestRunResponse,
        AsyncTestResponse,
        JobStatusResponse,
        JobListResponse,
        QueueStatsResponse,
        CreateReportRequest,
        ReportResponse,
        ReportListResponse,
        ReportDetailResponse,
        ResponseSummary,
    )),
    modifiers(&SecurityAddon),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Auth", description = "Authentication endpoints"),
        (name = "Users", description = "User management endpoints"),
        (name = "Projects", description = "Project management endpoints"),
        (name = "Collections", description = "Collection management endpoints"),
        (name = "Environments", description = "Environment management endpoints"),
        (name = "APIs", description = "API management endpoints"),
        (name = "Scenarios", description = "Scenario management endpoints with Gherkin support"),
        (name = "Test Execution", description = "Run tests against APIs"),
        (name = "Jobs", description = "Job queue management endpoints"),
        (name = "Reports", description = "Test report management endpoints")
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Load configuration
    let config = Config::from_env().expect("Failed to load configuration");
    let addr = config.server_addr();

    // Initialize application state (connects to all databases)
    tracing::info!("Connecting to databases...");
    let state = AppState::new(config)
        .await
        .expect("Failed to initialize application state");
    tracing::info!("Database connections established");

    // Build the main application router
    let app = build_router(state)
        // Add Swagger UI
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()));

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    tracing::info!("Server started on http://{}", addr);
    tracing::info!("Swagger UI: http://{}/swagger-ui/", addr);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .unwrap();
}
