mod common;

use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

use common::{Factory, TestApp};

#[tokio::test]
async fn test_create_project() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;

    let response = app
        .server
        .post("/api/projects")
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "name": "My New Project"
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["name"].as_str().unwrap(), "My New Project");
    assert!(body["id"].as_str().is_some());
    assert_eq!(body["user_id"].as_str().unwrap(), auth.user_id.to_string());
}

#[tokio::test]
async fn test_create_project_with_description() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;

    let response = app
        .server
        .post("/api/projects")
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "name": "Project with Description",
            "description": "This is a test project"
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(
        body["description"].as_str().unwrap(),
        "This is a test project"
    );
}

#[tokio::test]
async fn test_create_project_unauthorized() {
    let app = TestApp::new().await;

    let response = app
        .server
        .post("/api/projects")
        .json(&json!({
            "name": "My Project"
        }))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_list_projects_empty() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;

    let response = app
        .server
        .get("/api/projects")
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
    assert_eq!(body["total"].as_i64().unwrap(), 0);
}

#[tokio::test]
async fn test_list_projects() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;

    // Create some projects
    factory.create_project(auth.user_id).await;
    factory.create_project(auth.user_id).await;
    factory.create_project(auth.user_id).await;

    let response = app
        .server
        .get("/api/projects")
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["data"].as_array().unwrap().len(), 3);
    assert_eq!(body["total"].as_i64().unwrap(), 3);
}

#[tokio::test]
async fn test_list_projects_pagination() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;

    // Create 5 projects
    for _ in 0..5 {
        factory.create_project(auth.user_id).await;
    }

    // Get first page (limit=2)
    let response = app
        .server
        .get("/api/projects?limit=2&offset=0")
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["data"].as_array().unwrap().len(), 2);
    assert_eq!(body["total"].as_i64().unwrap(), 5);
    assert_eq!(body["limit"].as_i64().unwrap(), 2);
    assert_eq!(body["offset"].as_i64().unwrap(), 0);
}

#[tokio::test]
async fn test_list_projects_only_own() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    // User 1 creates projects
    let auth1 = factory.create_user().await;
    factory.create_project(auth1.user_id).await;
    factory.create_project(auth1.user_id).await;

    // User 2 creates projects
    let auth2 = factory.create_user().await;
    factory.create_project(auth2.user_id).await;

    // User 1 should only see their own projects
    let response = app
        .server
        .get("/api/projects")
        .add_header("Authorization", auth1.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["data"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_get_project_success() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;

    let response = app
        .server
        .get(&format!("/api/projects/{}", project.id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["id"].as_str().unwrap(), project.id.to_string());
    assert_eq!(body["name"].as_str().unwrap(), project.name);
}

#[tokio::test]
async fn test_get_project_not_found() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;

    let fake_id = Uuid::new_v4();
    let response = app
        .server
        .get(&format!("/api/projects/{}", fake_id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_project_other_user() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    // User 1 creates a project
    let auth1 = factory.create_user().await;
    let project = factory.create_project(auth1.user_id).await;

    // User 2 tries to access it
    let auth2 = factory.create_user().await;
    let response = app
        .server
        .get(&format!("/api/projects/{}", project.id))
        .add_header("Authorization", auth2.auth_header())
        .await;

    // Should return 404 (not exposing that project exists)
    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_project() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;

    let response = app
        .server
        .put(&format!("/api/projects/{}", project.id))
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "name": "Updated Project Name",
            "description": "Updated description"
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["name"].as_str().unwrap(), "Updated Project Name");
    assert_eq!(body["description"].as_str().unwrap(), "Updated description");
}

#[tokio::test]
async fn test_update_project_partial() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory
        .create_project_with_name(auth.user_id, "Original Name")
        .await;

    // Update only description
    let response = app
        .server
        .put(&format!("/api/projects/{}", project.id))
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "description": "New description only"
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["name"].as_str().unwrap(), "Original Name");
    assert_eq!(
        body["description"].as_str().unwrap(),
        "New description only"
    );
}

#[tokio::test]
async fn test_update_project_other_user() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    let auth1 = factory.create_user().await;
    let project = factory.create_project(auth1.user_id).await;

    let auth2 = factory.create_user().await;
    let response = app
        .server
        .put(&format!("/api/projects/{}", project.id))
        .add_header("Authorization", auth2.auth_header())
        .json(&json!({
            "name": "Hacked!"
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_project() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;

    let response = app
        .server
        .delete(&format!("/api/projects/{}", project.id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    // Verify it's deleted
    let get_response = app
        .server
        .get(&format!("/api/projects/{}", project.id))
        .add_header("Authorization", auth.auth_header())
        .await;

    get_response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_project_other_user() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    let auth1 = factory.create_user().await;
    let project = factory.create_project(auth1.user_id).await;

    let auth2 = factory.create_user().await;
    let response = app
        .server
        .delete(&format!("/api/projects/{}", project.id))
        .add_header("Authorization", auth2.auth_header())
        .await;

    response.assert_status(StatusCode::NOT_FOUND);

    // Verify it's NOT deleted
    let get_response = app
        .server
        .get(&format!("/api/projects/{}", project.id))
        .add_header("Authorization", auth1.auth_header())
        .await;

    get_response.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn test_delete_project_not_found() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;

    let fake_id = Uuid::new_v4();
    let response = app
        .server
        .delete(&format!("/api/projects/{}", fake_id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}
