// Library crate for ServalRun v2
// Exports modules for use by the worker binary and tests

pub mod config;
pub mod entity;
pub mod error;
pub mod handlers;
pub mod middlewares;
pub mod models;
pub mod queue;
pub mod repositories;
pub mod services;
pub mod state;

use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};

use crate::handlers::{
    cancel_job, create_api, create_collection, create_environment, create_from_gherkin,
    create_project, create_report, create_scenario, delete_api, delete_collection,
    delete_environment, delete_project, delete_report, delete_scenario, get_api, get_collection,
    get_environment, get_job_status, get_project, get_queue_stats, get_report, get_report_detail,
    get_scenario, list_apis, list_collections, list_environments, list_jobs, list_projects,
    list_reports, list_scenarios, login, me, parse_gherkin, register, requeue_job, run_api_tests,
    run_collection_tests, run_scenario_test, update_api, update_collection, update_environment,
    update_me, update_project, update_scenario,
};
use crate::middlewares::auth_middleware;
use crate::state::AppState;

/// Build the application router with the given state
pub fn build_router(state: AppState) -> Router {
    // Protected routes (require authentication)
    let protected_routes = Router::new()
        // Auth & User routes
        .route("/api/auth/me", get(me))
        .route("/api/users/me", put(update_me))
        // Project routes
        .route("/api/projects/{id}", get(get_project))
        .route("/api/projects", get(list_projects))
        .route("/api/projects", post(create_project))
        .route("/api/projects/{id}", put(update_project))
        .route("/api/projects/{id}", delete(delete_project))
        // Collection routes (nested under projects)
        .route(
            "/api/projects/{project_id}/collections",
            get(list_collections),
        )
        .route(
            "/api/projects/{project_id}/collections",
            post(create_collection),
        )
        // Collection routes (direct access)
        .route("/api/collections/{id}", get(get_collection))
        .route("/api/collections/{id}", put(update_collection))
        .route("/api/collections/{id}", delete(delete_collection))
        // Environment routes (nested under projects)
        .route(
            "/api/projects/{project_id}/environments",
            get(list_environments),
        )
        .route(
            "/api/projects/{project_id}/environments",
            post(create_environment),
        )
        // Environment routes (direct access)
        .route("/api/environments/{id}", get(get_environment))
        .route("/api/environments/{id}", put(update_environment))
        .route("/api/environments/{id}", delete(delete_environment))
        // API routes (nested under collections)
        .route("/api/collections/{collection_id}/apis", get(list_apis))
        .route("/api/collections/{collection_id}/apis", post(create_api))
        // API routes (direct access)
        .route("/api/apis/{id}", get(get_api))
        .route("/api/apis/{id}", put(update_api))
        .route("/api/apis/{id}", delete(delete_api))
        // Scenario routes (nested under APIs)
        .route("/api/apis/{api_id}/scenarios", get(list_scenarios))
        .route("/api/apis/{api_id}/scenarios", post(create_scenario))
        .route("/api/apis/{api_id}/scenarios/parse", post(parse_gherkin))
        .route(
            "/api/apis/{api_id}/scenarios/from-gherkin",
            post(create_from_gherkin),
        )
        // Scenario routes (direct access)
        .route("/api/scenarios/{id}", get(get_scenario))
        .route("/api/scenarios/{id}", put(update_scenario))
        .route("/api/scenarios/{id}", delete(delete_scenario))
        // Test execution routes
        .route("/api/scenarios/{scenario_id}/run", post(run_scenario_test))
        .route("/api/apis/{api_id}/run", post(run_api_tests))
        .route(
            "/api/collections/{collection_id}/run",
            post(run_collection_tests),
        )
        // Job management routes
        .route("/api/jobs", get(list_jobs))
        .route("/api/jobs/stats", get(get_queue_stats))
        .route("/api/jobs/{job_id}", get(get_job_status))
        .route("/api/jobs/{job_id}", delete(cancel_job))
        .route("/api/jobs/{job_id}/requeue", post(requeue_job))
        // Report routes (nested under projects)
        .route("/api/projects/{project_id}/reports", get(list_reports))
        .route("/api/projects/{project_id}/reports", post(create_report))
        // Report routes (direct access)
        .route("/api/reports/{id}", get(get_report))
        .route("/api/reports/{id}/detail", get(get_report_detail))
        .route("/api/reports/{id}", delete(delete_report))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    Router::new()
        .route("/", get(|| async { "Hello, ServalRun v2!" }))
        // Public auth routes
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        // Protected routes
        .merge(protected_routes)
        .with_state(state)
}
