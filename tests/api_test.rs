mod common;

use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

use common::{Factory, TestApp};

#[tokio::test]
async fn test_create_api() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let collection = factory.create_collection(project.id, auth.user_id).await;

    let response = app
        .server
        .post(&format!("/api/collections/{}/apis", collection.id))
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "name": "Get Users",
            "http_method": "GET",
            "endpoint": "/users"
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["name"].as_str().unwrap(), "Get Users");
    assert_eq!(body["http_method"].as_str().unwrap(), "GET");
    assert_eq!(body["endpoint"].as_str().unwrap(), "/users");
    assert_eq!(
        body["collection_id"].as_str().unwrap(),
        collection.id.to_string()
    );
}

#[tokio::test]
async fn test_create_api_with_all_fields() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let collection = factory.create_collection(project.id, auth.user_id).await;

    let response = app
        .server
        .post(&format!("/api/collections/{}/apis", collection.id))
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "name": "Create User",
            "http_method": "POST",
            "endpoint": "/users",
            "severity": 2,
            "description": "Creates a new user"
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["severity"].as_i64().unwrap(), 2);
    assert_eq!(body["description"].as_str().unwrap(), "Creates a new user");
}

#[tokio::test]
async fn test_create_api_invalid_collection() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;

    let fake_collection_id = Uuid::new_v4();
    let response = app
        .server
        .post(&format!("/api/collections/{}/apis", fake_collection_id))
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "name": "Test API",
            "http_method": "GET",
            "endpoint": "/test"
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_create_api_other_user_collection() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    // User 1 creates a collection
    let auth1 = factory.create_user().await;
    let project = factory.create_project(auth1.user_id).await;
    let collection = factory.create_collection(project.id, auth1.user_id).await;

    // User 2 tries to create an API in User 1's collection
    let auth2 = factory.create_user().await;
    let response = app
        .server
        .post(&format!("/api/collections/{}/apis", collection.id))
        .add_header("Authorization", auth2.auth_header())
        .json(&json!({
            "name": "Sneaky API",
            "http_method": "GET",
            "endpoint": "/sneaky"
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_apis_empty() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let collection = factory.create_collection(project.id, auth.user_id).await;

    let response = app
        .server
        .get(&format!("/api/collections/{}/apis", collection.id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
    assert_eq!(body["total"].as_i64().unwrap(), 0);
}

#[tokio::test]
async fn test_list_apis() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let collection = factory.create_collection(project.id, auth.user_id).await;

    // Create APIs
    factory.create_api(collection.id, auth.user_id).await;
    factory.create_api(collection.id, auth.user_id).await;
    factory.create_api(collection.id, auth.user_id).await;

    let response = app
        .server
        .get(&format!("/api/collections/{}/apis", collection.id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["data"].as_array().unwrap().len(), 3);
    assert_eq!(body["total"].as_i64().unwrap(), 3);
}

#[tokio::test]
async fn test_get_api() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let collection = factory.create_collection(project.id, auth.user_id).await;
    let api = factory.create_api(collection.id, auth.user_id).await;

    let response = app
        .server
        .get(&format!("/api/apis/{}", api.id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["id"].as_str().unwrap(), api.id.to_string());
}

#[tokio::test]
async fn test_get_api_not_found() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;

    let fake_id = Uuid::new_v4();
    let response = app
        .server
        .get(&format!("/api/apis/{}", fake_id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_api_other_user() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    let auth1 = factory.create_user().await;
    let project = factory.create_project(auth1.user_id).await;
    let collection = factory.create_collection(project.id, auth1.user_id).await;
    let api = factory.create_api(collection.id, auth1.user_id).await;

    let auth2 = factory.create_user().await;
    let response = app
        .server
        .get(&format!("/api/apis/{}", api.id))
        .add_header("Authorization", auth2.auth_header())
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_api() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let collection = factory.create_collection(project.id, auth.user_id).await;
    let api = factory.create_api(collection.id, auth.user_id).await;

    let response = app
        .server
        .put(&format!("/api/apis/{}", api.id))
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "name": "Updated API",
            "http_method": "POST",
            "endpoint": "/updated",
            "severity": 3
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["name"].as_str().unwrap(), "Updated API");
    assert_eq!(body["http_method"].as_str().unwrap(), "POST");
    assert_eq!(body["endpoint"].as_str().unwrap(), "/updated");
    assert_eq!(body["severity"].as_i64().unwrap(), 3);
}

#[tokio::test]
async fn test_update_api_partial() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let collection = factory.create_collection(project.id, auth.user_id).await;
    let api = factory.create_api(collection.id, auth.user_id).await;

    // Update only name
    let response = app
        .server
        .put(&format!("/api/apis/{}", api.id))
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "name": "New Name Only"
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["name"].as_str().unwrap(), "New Name Only");
    assert_eq!(body["http_method"].as_str().unwrap(), api.http_method);
}

#[tokio::test]
async fn test_update_api_other_user() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    let auth1 = factory.create_user().await;
    let project = factory.create_project(auth1.user_id).await;
    let collection = factory.create_collection(project.id, auth1.user_id).await;
    let api = factory.create_api(collection.id, auth1.user_id).await;

    let auth2 = factory.create_user().await;
    let response = app
        .server
        .put(&format!("/api/apis/{}", api.id))
        .add_header("Authorization", auth2.auth_header())
        .json(&json!({
            "name": "Hacked!"
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_api() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let collection = factory.create_collection(project.id, auth.user_id).await;
    let api = factory.create_api(collection.id, auth.user_id).await;

    let response = app
        .server
        .delete(&format!("/api/apis/{}", api.id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    // Verify deleted
    let get_response = app
        .server
        .get(&format!("/api/apis/{}", api.id))
        .add_header("Authorization", auth.auth_header())
        .await;

    get_response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_api_other_user() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    let auth1 = factory.create_user().await;
    let project = factory.create_project(auth1.user_id).await;
    let collection = factory.create_collection(project.id, auth1.user_id).await;
    let api = factory.create_api(collection.id, auth1.user_id).await;

    let auth2 = factory.create_user().await;
    let response = app
        .server
        .delete(&format!("/api/apis/{}", api.id))
        .add_header("Authorization", auth2.auth_header())
        .await;

    response.assert_status(StatusCode::NOT_FOUND);

    // Verify NOT deleted
    let get_response = app
        .server
        .get(&format!("/api/apis/{}", api.id))
        .add_header("Authorization", auth1.auth_header())
        .await;

    get_response.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn test_api_unauthorized() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let collection = factory.create_collection(project.id, auth.user_id).await;

    let response = app
        .server
        .get(&format!("/api/collections/{}/apis", collection.id))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}
