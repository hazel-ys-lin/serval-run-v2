mod common;

use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

use common::{Factory, TestApp};

#[tokio::test]
async fn test_create_collection() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;

    let response = app
        .server
        .post(&format!("/api/projects/{}/collections", project.id))
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "name": "My Collection"
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["name"].as_str().unwrap(), "My Collection");
    assert_eq!(body["project_id"].as_str().unwrap(), project.id.to_string());
}

#[tokio::test]
async fn test_create_collection_with_description() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;

    let response = app
        .server
        .post(&format!("/api/projects/{}/collections", project.id))
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "name": "API Collection",
            "description": "Contains all API tests"
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(
        body["description"].as_str().unwrap(),
        "Contains all API tests"
    );
}

#[tokio::test]
async fn test_create_collection_invalid_project() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;

    let fake_project_id = Uuid::new_v4();
    let response = app
        .server
        .post(&format!("/api/projects/{}/collections", fake_project_id))
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "name": "My Collection"
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_create_collection_other_user_project() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    // User 1 creates a project
    let auth1 = factory.create_user().await;
    let project = factory.create_project(auth1.user_id).await;

    // User 2 tries to create a collection in User 1's project
    let auth2 = factory.create_user().await;
    let response = app
        .server
        .post(&format!("/api/projects/{}/collections", project.id))
        .add_header("Authorization", auth2.auth_header())
        .json(&json!({
            "name": "Sneaky Collection"
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_collections_empty() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;

    let response = app
        .server
        .get(&format!("/api/projects/{}/collections", project.id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
    assert_eq!(body["total"].as_i64().unwrap(), 0);
}

#[tokio::test]
async fn test_list_collections() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;

    // Create collections
    factory.create_collection(project.id, auth.user_id).await;
    factory.create_collection(project.id, auth.user_id).await;

    let response = app
        .server
        .get(&format!("/api/projects/{}/collections", project.id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["data"].as_array().unwrap().len(), 2);
    assert_eq!(body["total"].as_i64().unwrap(), 2);
}

#[tokio::test]
async fn test_get_collection() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let collection = factory.create_collection(project.id, auth.user_id).await;

    let response = app
        .server
        .get(&format!("/api/collections/{}", collection.id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["id"].as_str().unwrap(), collection.id.to_string());
}

#[tokio::test]
async fn test_get_collection_not_found() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;

    let fake_id = Uuid::new_v4();
    let response = app
        .server
        .get(&format!("/api/collections/{}", fake_id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_collection_other_user() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    let auth1 = factory.create_user().await;
    let project = factory.create_project(auth1.user_id).await;
    let collection = factory.create_collection(project.id, auth1.user_id).await;

    let auth2 = factory.create_user().await;
    let response = app
        .server
        .get(&format!("/api/collections/{}", collection.id))
        .add_header("Authorization", auth2.auth_header())
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_collection() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let collection = factory.create_collection(project.id, auth.user_id).await;

    let response = app
        .server
        .put(&format!("/api/collections/{}", collection.id))
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "name": "Updated Collection",
            "description": "New description"
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["name"].as_str().unwrap(), "Updated Collection");
    assert_eq!(body["description"].as_str().unwrap(), "New description");
}

#[tokio::test]
async fn test_update_collection_other_user() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    let auth1 = factory.create_user().await;
    let project = factory.create_project(auth1.user_id).await;
    let collection = factory.create_collection(project.id, auth1.user_id).await;

    let auth2 = factory.create_user().await;
    let response = app
        .server
        .put(&format!("/api/collections/{}", collection.id))
        .add_header("Authorization", auth2.auth_header())
        .json(&json!({
            "name": "Hacked!"
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_collection() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let collection = factory.create_collection(project.id, auth.user_id).await;

    let response = app
        .server
        .delete(&format!("/api/collections/{}", collection.id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    // Verify deleted
    let get_response = app
        .server
        .get(&format!("/api/collections/{}", collection.id))
        .add_header("Authorization", auth.auth_header())
        .await;

    get_response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_collection_other_user() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    let auth1 = factory.create_user().await;
    let project = factory.create_project(auth1.user_id).await;
    let collection = factory.create_collection(project.id, auth1.user_id).await;

    let auth2 = factory.create_user().await;
    let response = app
        .server
        .delete(&format!("/api/collections/{}", collection.id))
        .add_header("Authorization", auth2.auth_header())
        .await;

    response.assert_status(StatusCode::NOT_FOUND);

    // Verify NOT deleted
    let get_response = app
        .server
        .get(&format!("/api/collections/{}", collection.id))
        .add_header("Authorization", auth1.auth_header())
        .await;

    get_response.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn test_collection_unauthorized() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;

    let response = app
        .server
        .get(&format!("/api/projects/{}/collections", project.id))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}
