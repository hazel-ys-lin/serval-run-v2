mod common;

use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

use common::{Factory, TestApp};

#[tokio::test]
async fn test_create_report() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let env = factory.create_environment(project.id, auth.user_id).await;

    let response = app
        .server
        .post(&format!("/api/projects/{}/reports", project.id))
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "environment_id": env.id,
            "report_level": 2
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["project_id"].as_str().unwrap(), project.id.to_string());
    assert_eq!(body["environment_id"].as_str().unwrap(), env.id.to_string());
    assert_eq!(body["report_level"].as_i64().unwrap(), 2);
    assert!(!body["finished"].as_bool().unwrap());
}

#[tokio::test]
async fn test_create_report_with_collection() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let collection = factory.create_collection(project.id, auth.user_id).await;
    let env = factory.create_environment(project.id, auth.user_id).await;

    let response = app
        .server
        .post(&format!("/api/projects/{}/reports", project.id))
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "environment_id": env.id,
            "collection_id": collection.id,
            "report_level": 1,
            "report_type": "collection_test"
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["collection_id"].as_str().unwrap(), collection.id.to_string());
    assert_eq!(body["report_level"].as_i64().unwrap(), 1);
    assert_eq!(body["report_type"].as_str().unwrap(), "collection_test");
}

#[tokio::test]
async fn test_create_report_invalid_project() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;

    let fake_project_id = Uuid::new_v4();
    let fake_env_id = Uuid::new_v4();
    let response = app
        .server
        .post(&format!("/api/projects/{}/reports", fake_project_id))
        .add_header("Authorization", auth.auth_header())
        .json(&json!({
            "environment_id": fake_env_id,
            "report_level": 2
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_create_report_other_user_project() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    // User 1 creates a project
    let auth1 = factory.create_user().await;
    let project = factory.create_project(auth1.user_id).await;
    let env = factory.create_environment(project.id, auth1.user_id).await;

    // User 2 tries to create a report in User 1's project
    let auth2 = factory.create_user().await;
    let response = app
        .server
        .post(&format!("/api/projects/{}/reports", project.id))
        .add_header("Authorization", auth2.auth_header())
        .json(&json!({
            "environment_id": env.id,
            "report_level": 2
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_reports_empty() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;

    let response = app
        .server
        .get(&format!("/api/projects/{}/reports", project.id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
    assert_eq!(body["total"].as_i64().unwrap(), 0);
}

#[tokio::test]
async fn test_list_reports() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let env = factory.create_environment(project.id, auth.user_id).await;

    // Create reports
    factory.create_report(project.id, env.id, auth.user_id).await;
    factory.create_report(project.id, env.id, auth.user_id).await;
    factory.create_report(project.id, env.id, auth.user_id).await;

    let response = app
        .server
        .get(&format!("/api/projects/{}/reports", project.id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["data"].as_array().unwrap().len(), 3);
    assert_eq!(body["total"].as_i64().unwrap(), 3);
}

#[tokio::test]
async fn test_list_reports_other_user_project() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    let auth1 = factory.create_user().await;
    let project = factory.create_project(auth1.user_id).await;

    let auth2 = factory.create_user().await;
    let response = app
        .server
        .get(&format!("/api/projects/{}/reports", project.id))
        .add_header("Authorization", auth2.auth_header())
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_report() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let env = factory.create_environment(project.id, auth.user_id).await;
    let report = factory.create_report(project.id, env.id, auth.user_id).await;

    let response = app
        .server
        .get(&format!("/api/reports/{}", report.id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["id"].as_str().unwrap(), report.id.to_string());
    assert_eq!(body["project_id"].as_str().unwrap(), project.id.to_string());
}

#[tokio::test]
async fn test_get_report_not_found() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;

    let fake_id = Uuid::new_v4();
    let response = app
        .server
        .get(&format!("/api/reports/{}", fake_id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_report_other_user() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    let auth1 = factory.create_user().await;
    let project = factory.create_project(auth1.user_id).await;
    let env = factory.create_environment(project.id, auth1.user_id).await;
    let report = factory.create_report(project.id, env.id, auth1.user_id).await;

    let auth2 = factory.create_user().await;
    let response = app
        .server
        .get(&format!("/api/reports/{}", report.id))
        .add_header("Authorization", auth2.auth_header())
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_report_detail() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let env = factory.create_environment(project.id, auth.user_id).await;
    let report = factory.create_report(project.id, env.id, auth.user_id).await;

    let response = app
        .server
        .get(&format!("/api/reports/{}/detail", report.id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["id"].as_str().unwrap(), report.id.to_string());
    assert!(body["responses"].as_array().is_some());
}

#[tokio::test]
async fn test_get_report_detail_other_user() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    let auth1 = factory.create_user().await;
    let project = factory.create_project(auth1.user_id).await;
    let env = factory.create_environment(project.id, auth1.user_id).await;
    let report = factory.create_report(project.id, env.id, auth1.user_id).await;

    let auth2 = factory.create_user().await;
    let response = app
        .server
        .get(&format!("/api/reports/{}/detail", report.id))
        .add_header("Authorization", auth2.auth_header())
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_report() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let env = factory.create_environment(project.id, auth.user_id).await;
    let report = factory.create_report(project.id, env.id, auth.user_id).await;

    let response = app
        .server
        .delete(&format!("/api/reports/{}", report.id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    // Verify deleted
    let get_response = app
        .server
        .get(&format!("/api/reports/{}", report.id))
        .add_header("Authorization", auth.auth_header())
        .await;

    get_response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_report_other_user() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    let auth1 = factory.create_user().await;
    let project = factory.create_project(auth1.user_id).await;
    let env = factory.create_environment(project.id, auth1.user_id).await;
    let report = factory.create_report(project.id, env.id, auth1.user_id).await;

    let auth2 = factory.create_user().await;
    let response = app
        .server
        .delete(&format!("/api/reports/{}", report.id))
        .add_header("Authorization", auth2.auth_header())
        .await;

    response.assert_status(StatusCode::NOT_FOUND);

    // Verify NOT deleted
    let get_response = app
        .server
        .get(&format!("/api/reports/{}", report.id))
        .add_header("Authorization", auth1.auth_header())
        .await;

    get_response.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn test_report_unauthorized() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;

    let response = app
        .server
        .get(&format!("/api/projects/{}/reports", project.id))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}
