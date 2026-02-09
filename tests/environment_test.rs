mod common;

use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

use common::{Factory, TestApp};

#[tokio::test]
async fn test_create_environment() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;

    let response = app
        .server
        .post(&format!("/api/projects/{}/environments", project.id))
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "title": "staging",
            "domain_name": "https://api.staging.example.com"
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["title"].as_str().unwrap(), "staging");
    assert_eq!(body["domain_name"].as_str().unwrap(), "https://api.staging.example.com");
    assert_eq!(body["project_id"].as_str().unwrap(), project.id.to_string());
}

#[tokio::test]
async fn test_create_environment_invalid_project() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;

    let fake_project_id = Uuid::new_v4();
    let response = app
        .server
        .post(&format!("/api/projects/{}/environments", fake_project_id))
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "title": "prod",
            "domain_name": "https://api.example.com"
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_create_environment_other_user_project() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    let auth1 = factory.create_user().await;
    let project = factory.create_project(auth1.user_id).await;

    let auth2 = factory.create_user().await;
    let response = app
        .server
        .post(&format!("/api/projects/{}/environments", project.id))
        .add_header("Authorization", auth2.auth_header())
        .json(&json!({
            "title": "hacked",
            "domain_name": "https://evil.com"
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_environments_empty() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;

    let response = app
        .server
        .get(&format!("/api/projects/{}/environments", project.id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
    assert_eq!(body["total"].as_i64().unwrap(), 0);
}

#[tokio::test]
async fn test_list_environments() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;

    // Create environments
    factory.create_environment(project.id, auth.user_id).await;
    factory.create_environment(project.id, auth.user_id).await;

    let response = app
        .server
        .get(&format!("/api/projects/{}/environments", project.id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["data"].as_array().unwrap().len(), 2);
    assert_eq!(body["total"].as_i64().unwrap(), 2);
}

#[tokio::test]
async fn test_get_environment() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let env = factory.create_environment(project.id, auth.user_id).await;

    let response = app
        .server
        .get(&format!("/api/environments/{}", env.id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["id"].as_str().unwrap(), env.id.to_string());
}

#[tokio::test]
async fn test_get_environment_not_found() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;

    let fake_id = Uuid::new_v4();
    let response = app
        .server
        .get(&format!("/api/environments/{}", fake_id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_environment_other_user() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    let auth1 = factory.create_user().await;
    let project = factory.create_project(auth1.user_id).await;
    let env = factory.create_environment(project.id, auth1.user_id).await;

    let auth2 = factory.create_user().await;
    let response = app
        .server
        .get(&format!("/api/environments/{}", env.id))
        .add_header("Authorization", auth2.auth_header())
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_environment() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let env = factory.create_environment(project.id, auth.user_id).await;

    let response = app
        .server
        .put(&format!("/api/environments/{}", env.id))
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "title": "production",
            "domain_name": "https://api.prod.example.com"
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["title"].as_str().unwrap(), "production");
    assert_eq!(body["domain_name"].as_str().unwrap(), "https://api.prod.example.com");
}

#[tokio::test]
async fn test_update_environment_partial() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let env = factory.create_environment(project.id, auth.user_id).await;

    // Update only title
    let response = app
        .server
        .put(&format!("/api/environments/{}", env.id))
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "title": "dev"
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["title"].as_str().unwrap(), "dev");
    // domain_name should remain unchanged
    assert_eq!(body["domain_name"].as_str().unwrap(), env.domain_name);
}

#[tokio::test]
async fn test_update_environment_other_user() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    let auth1 = factory.create_user().await;
    let project = factory.create_project(auth1.user_id).await;
    let env = factory.create_environment(project.id, auth1.user_id).await;

    let auth2 = factory.create_user().await;
    let response = app
        .server
        .put(&format!("/api/environments/{}", env.id))
        .add_header("Authorization", auth2.auth_header())
        .json(&json!({
            "title": "hacked"
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_environment() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let env = factory.create_environment(project.id, auth.user_id).await;

    let response = app
        .server
        .delete(&format!("/api/environments/{}", env.id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    // Verify deleted
    let get_response = app
        .server
        .get(&format!("/api/environments/{}", env.id))
        .add_header("Authorization", auth.auth_header())
        .await;

    get_response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_environment_other_user() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    let auth1 = factory.create_user().await;
    let project = factory.create_project(auth1.user_id).await;
    let env = factory.create_environment(project.id, auth1.user_id).await;

    let auth2 = factory.create_user().await;
    let response = app
        .server
        .delete(&format!("/api/environments/{}", env.id))
        .add_header("Authorization", auth2.auth_header())
        .await;

    response.assert_status(StatusCode::NOT_FOUND);

    // Verify NOT deleted
    let get_response = app
        .server
        .get(&format!("/api/environments/{}", env.id))
        .add_header("Authorization", auth1.auth_header())
        .await;

    get_response.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn test_environment_unauthorized() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;

    let response = app
        .server
        .get(&format!("/api/projects/{}/environments", project.id))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}
