mod common;

use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

use common::{Factory, TestApp};

#[tokio::test]
async fn test_create_scenario() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let collection = factory.create_collection(project.id, auth.user_id).await;
    let api = factory.create_api(collection.id, auth.user_id).await;

    let response = app
        .server
        .post(&format!("/api/apis/{}/scenarios", api.id))
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "title": "Test Login",
            "description": "Test successful login",
            "tags": ["smoke", "auth"],
            "steps": [
                {
                    "keyword": "Given",
                    "keyword_type": "Context",
                    "text": "a valid user exists"
                },
                {
                    "keyword": "When",
                    "keyword_type": "Action",
                    "text": "I send a POST request to /login"
                },
                {
                    "keyword": "Then",
                    "keyword_type": "Outcome",
                    "text": "I should receive a 200 response"
                }
            ],
            "examples": [
                {
                    "example": {"username": "testuser", "password": "pass123"},
                    "expected_response_body": {"success": true},
                    "expected_status_code": 200
                }
            ]
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["title"].as_str().unwrap(), "Test Login");
    assert_eq!(body["api_id"].as_str().unwrap(), api.id.to_string());
}

#[tokio::test]
async fn test_create_scenario_invalid_api() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;

    let fake_api_id = Uuid::new_v4();
    let response = app
        .server
        .post(&format!("/api/apis/{}/scenarios", fake_api_id))
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "title": "Test",
            "steps": [],
            "examples": []
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_create_scenario_other_user_api() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    // User 1 creates an API
    let auth1 = factory.create_user().await;
    let project = factory.create_project(auth1.user_id).await;
    let collection = factory.create_collection(project.id, auth1.user_id).await;
    let api = factory.create_api(collection.id, auth1.user_id).await;

    // User 2 tries to create a scenario
    let auth2 = factory.create_user().await;
    let response = app
        .server
        .post(&format!("/api/apis/{}/scenarios", api.id))
        .add_header("Authorization", auth2.auth_header())
        .json(&json!({
            "title": "Sneaky Scenario",
            "steps": [],
            "examples": []
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_scenarios_empty() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let collection = factory.create_collection(project.id, auth.user_id).await;
    let api = factory.create_api(collection.id, auth.user_id).await;

    let response = app
        .server
        .get(&format!("/api/apis/{}/scenarios", api.id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
    assert_eq!(body["total"].as_i64().unwrap(), 0);
}

#[tokio::test]
async fn test_list_scenarios() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let collection = factory.create_collection(project.id, auth.user_id).await;
    let api = factory.create_api(collection.id, auth.user_id).await;

    // Create scenarios via API
    for i in 1..=3 {
        app.server
            .post(&format!("/api/apis/{}/scenarios", api.id))
            .add_header("Authorization", auth.auth_header())
            .json(&json!({
                "title": format!("Scenario {}", i),
                "steps": [],
                "examples": []
            }))
            .await;
    }

    let response = app
        .server
        .get(&format!("/api/apis/{}/scenarios", api.id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["data"].as_array().unwrap().len(), 3);
    assert_eq!(body["total"].as_i64().unwrap(), 3);
}

#[tokio::test]
async fn test_get_scenario() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let collection = factory.create_collection(project.id, auth.user_id).await;
    let api = factory.create_api(collection.id, auth.user_id).await;

    // Create a scenario
    let create_response = app
        .server
        .post(&format!("/api/apis/{}/scenarios", api.id))
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "title": "Test Scenario",
            "steps": [],
            "examples": []
        }))
        .await;

    let created: serde_json::Value = create_response.json();
    let scenario_id = created["id"].as_str().unwrap();

    let response = app
        .server
        .get(&format!("/api/scenarios/{}", scenario_id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["id"].as_str().unwrap(), scenario_id);
    assert_eq!(body["title"].as_str().unwrap(), "Test Scenario");
}

#[tokio::test]
async fn test_get_scenario_not_found() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;

    let fake_id = Uuid::new_v4();
    let response = app
        .server
        .get(&format!("/api/scenarios/{}", fake_id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_scenario_other_user() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    // User 1 creates a scenario
    let auth1 = factory.create_user().await;
    let project = factory.create_project(auth1.user_id).await;
    let collection = factory.create_collection(project.id, auth1.user_id).await;
    let api = factory.create_api(collection.id, auth1.user_id).await;

    let create_response = app
        .server
        .post(&format!("/api/apis/{}/scenarios", api.id))
        .add_header("Authorization", auth1.auth_header())
        .json(&json!({
            "title": "User 1's Scenario",
            "steps": [],
            "examples": []
        }))
        .await;

    let created: serde_json::Value = create_response.json();
    let scenario_id = created["id"].as_str().unwrap();

    // User 2 tries to access it
    let auth2 = factory.create_user().await;
    let response = app
        .server
        .get(&format!("/api/scenarios/{}", scenario_id))
        .add_header("Authorization", auth2.auth_header())
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_scenario() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let collection = factory.create_collection(project.id, auth.user_id).await;
    let api = factory.create_api(collection.id, auth.user_id).await;

    // Create a scenario
    let create_response = app
        .server
        .post(&format!("/api/apis/{}/scenarios", api.id))
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "title": "Original Title",
            "steps": [],
            "examples": []
        }))
        .await;

    let created: serde_json::Value = create_response.json();
    let scenario_id = created["id"].as_str().unwrap();

    // Update it
    let response = app
        .server
        .put(&format!("/api/scenarios/{}", scenario_id))
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "title": "Updated Title",
            "description": "Updated description"
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["title"].as_str().unwrap(), "Updated Title");
    assert_eq!(body["description"].as_str().unwrap(), "Updated description");
}

#[tokio::test]
async fn test_update_scenario_other_user() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    let auth1 = factory.create_user().await;
    let project = factory.create_project(auth1.user_id).await;
    let collection = factory.create_collection(project.id, auth1.user_id).await;
    let api = factory.create_api(collection.id, auth1.user_id).await;

    let create_response = app
        .server
        .post(&format!("/api/apis/{}/scenarios", api.id))
        .add_header("Authorization", auth1.auth_header())
        .json(&json!({
            "title": "Original",
            "steps": [],
            "examples": []
        }))
        .await;

    let created: serde_json::Value = create_response.json();
    let scenario_id = created["id"].as_str().unwrap();

    let auth2 = factory.create_user().await;
    let response = app
        .server
        .put(&format!("/api/scenarios/{}", scenario_id))
        .add_header("Authorization", auth2.auth_header())
        .json(&json!({
            "title": "Hacked!"
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_scenario() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let collection = factory.create_collection(project.id, auth.user_id).await;
    let api = factory.create_api(collection.id, auth.user_id).await;

    let create_response = app
        .server
        .post(&format!("/api/apis/{}/scenarios", api.id))
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "title": "To Be Deleted",
            "steps": [],
            "examples": []
        }))
        .await;

    let created: serde_json::Value = create_response.json();
    let scenario_id = created["id"].as_str().unwrap();

    let response = app
        .server
        .delete(&format!("/api/scenarios/{}", scenario_id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    // Verify deleted
    let get_response = app
        .server
        .get(&format!("/api/scenarios/{}", scenario_id))
        .add_header("Authorization", auth.auth_header())
        .await;

    get_response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_scenario_other_user() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    let auth1 = factory.create_user().await;
    let project = factory.create_project(auth1.user_id).await;
    let collection = factory.create_collection(project.id, auth1.user_id).await;
    let api = factory.create_api(collection.id, auth1.user_id).await;

    let create_response = app
        .server
        .post(&format!("/api/apis/{}/scenarios", api.id))
        .add_header("Authorization", auth1.auth_header())
        .json(&json!({
            "title": "Protected",
            "steps": [],
            "examples": []
        }))
        .await;

    let created: serde_json::Value = create_response.json();
    let scenario_id = created["id"].as_str().unwrap();

    let auth2 = factory.create_user().await;
    let response = app
        .server
        .delete(&format!("/api/scenarios/{}", scenario_id))
        .add_header("Authorization", auth2.auth_header())
        .await;

    response.assert_status(StatusCode::NOT_FOUND);

    // Verify NOT deleted
    let get_response = app
        .server
        .get(&format!("/api/scenarios/{}", scenario_id))
        .add_header("Authorization", auth1.auth_header())
        .await;

    get_response.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn test_parse_gherkin() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let collection = factory.create_collection(project.id, auth.user_id).await;
    let api = factory.create_api(collection.id, auth.user_id).await;

    let gherkin_code = r#"
Feature: Login
  As a user
  I want to login
  So that I can access my account

  Scenario: Successful login
    Given a valid user exists
    When I send a POST request to /login
    Then I should receive a 200 response
"#;

    let response = app
        .server
        .post(&format!("/api/apis/{}/scenarios/parse", api.id))
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "gherkin_code": gherkin_code
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["feature_name"].as_str().unwrap(), "Login");
    assert_eq!(body["scenarios"].as_array().unwrap().len(), 1);
    assert_eq!(
        body["scenarios"][0]["title"].as_str().unwrap(),
        "Successful login"
    );
}

#[tokio::test]
async fn test_scenario_unauthorized() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let collection = factory.create_collection(project.id, auth.user_id).await;
    let api = factory.create_api(collection.id, auth.user_id).await;

    let response = app
        .server
        .get(&format!("/api/apis/{}/scenarios", api.id))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}
