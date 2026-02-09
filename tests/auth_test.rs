mod common;

use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

use common::{Factory, TestApp};

#[tokio::test]
async fn test_register_success() {
    let app = TestApp::new().await;
    let unique_id = Uuid::new_v4();

    let response = app
        .server
        .post("/api/auth/register")
        .json(&json!({
            "email": format!("test-{}@example.com", unique_id),
            "password": "password123",
            "name": "Test User"
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert!(body["token"].as_str().is_some());
    assert!(body["user"]["id"].as_str().is_some());
    assert_eq!(
        body["user"]["email"].as_str().unwrap(),
        format!("test-{}@example.com", unique_id)
    );
}

#[tokio::test]
async fn test_register_with_job_title() {
    let app = TestApp::new().await;
    let unique_id = Uuid::new_v4();

    let response = app
        .server
        .post("/api/auth/register")
        .json(&json!({
            "email": format!("test-{}@example.com", unique_id),
            "password": "password123",
            "name": "Test User",
            "job_title": "Software Engineer"
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["user"]["job_title"].as_str().unwrap(), "Software Engineer");
}

#[tokio::test]
async fn test_register_duplicate_email() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    // Create a user first
    let auth = factory.create_user().await;

    // Try to register with the same email
    let response = app
        .server
        .post("/api/auth/register")
        .json(&json!({
            "email": auth.email,
            "password": "password123",
            "name": "Another User"
        }))
        .await;

    response.assert_status(StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_register_empty_email() {
    let app = TestApp::new().await;

    let response = app
        .server
        .post("/api/auth/register")
        .json(&json!({
            "email": "",
            "password": "password123",
            "name": "Test User"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_register_short_password() {
    let app = TestApp::new().await;
    let unique_id = Uuid::new_v4();

    let response = app
        .server
        .post("/api/auth/register")
        .json(&json!({
            "email": format!("test-{}@example.com", unique_id),
            "password": "short",
            "name": "Test User"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    let body: serde_json::Value = response.json();
    assert!(body["details"]
        .as_str()
        .unwrap()
        .contains("at least 8 characters"));
}

#[tokio::test]
async fn test_register_empty_name() {
    let app = TestApp::new().await;
    let unique_id = Uuid::new_v4();

    let response = app
        .server
        .post("/api/auth/register")
        .json(&json!({
            "email": format!("test-{}@example.com", unique_id),
            "password": "password123",
            "name": ""
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_login_success() {
    let app = TestApp::new().await;
    let unique_id = Uuid::new_v4();
    let email = format!("test-{}@example.com", unique_id);
    let password = "password123";

    // Register first
    app.server
        .post("/api/auth/register")
        .json(&json!({
            "email": &email,
            "password": password,
            "name": "Test User"
        }))
        .await;

    // Then login
    let response = app
        .server
        .post("/api/auth/login")
        .json(&json!({
            "email": &email,
            "password": password
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert!(body["token"].as_str().is_some());
    assert_eq!(body["user"]["email"].as_str().unwrap(), email);
}

#[tokio::test]
async fn test_login_invalid_email() {
    let app = TestApp::new().await;

    let response = app
        .server
        .post("/api/auth/login")
        .json(&json!({
            "email": "nonexistent@example.com",
            "password": "password123"
        }))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_login_invalid_password() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    let auth = factory.create_user().await;

    let response = app
        .server
        .post("/api/auth/login")
        .json(&json!({
            "email": auth.email,
            "password": "wrongpassword"
        }))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_me_authenticated() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    let auth = factory.create_user().await;

    let response = app
        .server
        .get("/api/auth/me")
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["email"].as_str().unwrap(), auth.email);
}

#[tokio::test]
async fn test_me_no_token() {
    let app = TestApp::new().await;

    let response = app.server.get("/api/auth/me").await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_me_invalid_token() {
    let app = TestApp::new().await;

    let response = app
        .server
        .get("/api/auth/me")
        .add_header("Authorization", "Bearer invalid-token")
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_update_me_success() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    let auth = factory.create_user().await;

    let response = app
        .server
        .put("/api/users/me")
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "name": "Updated Name",
            "job_title": "Senior Engineer"
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["name"].as_str().unwrap(), "Updated Name");
    assert_eq!(body["job_title"].as_str().unwrap(), "Senior Engineer");
}

#[tokio::test]
async fn test_update_me_partial() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    let auth = factory.create_user().await;

    // Update only name
    let response = app
        .server
        .put("/api/users/me")
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "name": "New Name"
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["name"].as_str().unwrap(), "New Name");
}

#[tokio::test]
async fn test_update_me_unauthorized() {
    let app = TestApp::new().await;

    let response = app
        .server
        .put("/api/users/me")
        .json(&json!({
            "name": "Updated Name"
        }))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}
