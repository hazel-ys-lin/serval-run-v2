mod common;

use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

use common::{Factory, TestApp};

#[tokio::test]
async fn test_run_scenario_async() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let h = factory.create_hierarchy().await;
    let api = factory.create_api(h.collection.id, h.auth.user_id).await;
    let scenario = factory.create_scenario(api.id, h.auth.user_id).await;

    let response = app
        .server
        .post(&format!("/api/scenarios/{}/run", scenario.id))
        .add_header("Authorization", h.auth.auth_header())
        .json(&json!({
            "environment_id": h.environment.id,
            "async_execution": true
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert!(body["job_id"].as_str().is_some());
    assert_eq!(body["status"].as_str().unwrap(), "pending");
}

#[tokio::test]
async fn test_run_scenario_not_found() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let h = factory.create_hierarchy().await;

    let fake_scenario_id = Uuid::new_v4();
    let response = app
        .server
        .post(&format!("/api/scenarios/{}/run", fake_scenario_id))
        .add_header("Authorization", h.auth.auth_header())
        .json(&json!({
            "environment_id": h.environment.id,
            "async_execution": true
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_run_scenario_invalid_environment() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let h = factory.create_hierarchy().await;
    let api = factory.create_api(h.collection.id, h.auth.user_id).await;
    let scenario = factory.create_scenario(api.id, h.auth.user_id).await;

    let fake_env_id = Uuid::new_v4();
    let response = app
        .server
        .post(&format!("/api/scenarios/{}/run", scenario.id))
        .add_header("Authorization", h.auth.auth_header())
        .json(&json!({
            "environment_id": fake_env_id,
            "async_execution": true
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_run_scenario_other_user() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    // User 1 creates the hierarchy and scenario
    let h = factory.create_hierarchy().await;
    let api = factory.create_api(h.collection.id, h.auth.user_id).await;
    let scenario = factory.create_scenario(api.id, h.auth.user_id).await;

    // User 2 tries to run it
    let auth2 = factory.create_user().await;
    let response = app
        .server
        .post(&format!("/api/scenarios/{}/run", scenario.id))
        .add_header("Authorization", auth2.auth_header())
        .json(&json!({
            "environment_id": h.environment.id,
            "async_execution": true
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_run_scenario_unauthorized() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let h = factory.create_hierarchy().await;
    let api = factory.create_api(h.collection.id, h.auth.user_id).await;
    let scenario = factory.create_scenario(api.id, h.auth.user_id).await;

    let response = app
        .server
        .post(&format!("/api/scenarios/{}/run", scenario.id))
        .json(&json!({
            "environment_id": h.environment.id,
            "async_execution": true
        }))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_run_api_tests_async() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let h = factory.create_hierarchy().await;
    let api = factory.create_api(h.collection.id, h.auth.user_id).await;
    let _scenario = factory.create_scenario(api.id, h.auth.user_id).await;

    let response = app
        .server
        .post(&format!("/api/apis/{}/run", api.id))
        .add_header("Authorization", h.auth.auth_header())
        .json(&json!({
            "environment_id": h.environment.id,
            "async_execution": true
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert!(body["job_id"].as_str().is_some());
    assert_eq!(body["status"].as_str().unwrap(), "pending");
}

#[tokio::test]
async fn test_run_api_tests_no_scenarios() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let h = factory.create_hierarchy().await;
    let api = factory.create_api(h.collection.id, h.auth.user_id).await;

    // API has no scenarios
    let response = app
        .server
        .post(&format!("/api/apis/{}/run", api.id))
        .add_header("Authorization", h.auth.auth_header())
        .json(&json!({
            "environment_id": h.environment.id,
            "async_execution": false
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_run_api_tests_not_found() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let h = factory.create_hierarchy().await;

    let fake_api_id = Uuid::new_v4();
    let response = app
        .server
        .post(&format!("/api/apis/{}/run", fake_api_id))
        .add_header("Authorization", h.auth.auth_header())
        .json(&json!({
            "environment_id": h.environment.id,
            "async_execution": true
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_run_collection_tests_async() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let h = factory.create_hierarchy().await;
    let api = factory.create_api(h.collection.id, h.auth.user_id).await;
    let _scenario = factory.create_scenario(api.id, h.auth.user_id).await;

    let response = app
        .server
        .post(&format!("/api/collections/{}/run", h.collection.id))
        .add_header("Authorization", h.auth.auth_header())
        .json(&json!({
            "environment_id": h.environment.id,
            "async_execution": true
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert!(body["job_id"].as_str().is_some());
    assert_eq!(body["status"].as_str().unwrap(), "pending");
}

#[tokio::test]
async fn test_run_collection_tests_no_apis() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let h = factory.create_hierarchy().await;

    // Collection has no APIs
    let response = app
        .server
        .post(&format!("/api/collections/{}/run", h.collection.id))
        .add_header("Authorization", h.auth.auth_header())
        .json(&json!({
            "environment_id": h.environment.id,
            "async_execution": false
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_run_collection_tests_not_found() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let h = factory.create_hierarchy().await;

    let fake_collection_id = Uuid::new_v4();
    let response = app
        .server
        .post(&format!("/api/collections/{}/run", fake_collection_id))
        .add_header("Authorization", h.auth.auth_header())
        .json(&json!({
            "environment_id": h.environment.id,
            "async_execution": true
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_run_collection_tests_other_user() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    let h = factory.create_hierarchy().await;
    let api = factory.create_api(h.collection.id, h.auth.user_id).await;
    let _scenario = factory.create_scenario(api.id, h.auth.user_id).await;

    // User 2 tries to run collection tests
    let auth2 = factory.create_user().await;
    let response = app
        .server
        .post(&format!("/api/collections/{}/run", h.collection.id))
        .add_header("Authorization", auth2.auth_header())
        .json(&json!({
            "environment_id": h.environment.id,
            "async_execution": true
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}
