mod common;

use axum::http::StatusCode;
use uuid::Uuid;

use common::{Factory, TestApp};
use serval_run::queue::{TestJob, TestJobConfig, TestJobType};

#[tokio::test]
async fn test_list_jobs_empty() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;

    let response = app
        .server
        .get("/api/jobs")
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
    assert_eq!(body["total"].as_i64().unwrap(), 0);
}

#[tokio::test]
async fn test_list_jobs() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let env = factory.create_environment(project.id, auth.user_id).await;

    // Create jobs directly via the queue
    let config = TestJobConfig::default();
    for _ in 0..3 {
        let job = TestJob::new(
            TestJobType::Scenario,
            Uuid::new_v4(),
            env.id,
            auth.user_id,
            config.clone(),
        );
        app.state.job_queue.enqueue(job).await.unwrap();
    }

    let response = app
        .server
        .get("/api/jobs")
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["data"].as_array().unwrap().len(), 3);
    assert_eq!(body["total"].as_i64().unwrap(), 3);
}

#[tokio::test]
async fn test_list_jobs_only_own() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    // User 1 creates jobs
    let auth1 = factory.create_user().await;
    let project1 = factory.create_project(auth1.user_id).await;
    let env1 = factory.create_environment(project1.id, auth1.user_id).await;

    let config = TestJobConfig::default();
    for _ in 0..2 {
        let job = TestJob::new(
            TestJobType::Scenario,
            Uuid::new_v4(),
            env1.id,
            auth1.user_id,
            config.clone(),
        );
        app.state.job_queue.enqueue(job).await.unwrap();
    }

    // User 2 creates jobs
    let auth2 = factory.create_user().await;
    let project2 = factory.create_project(auth2.user_id).await;
    let env2 = factory.create_environment(project2.id, auth2.user_id).await;

    let job = TestJob::new(
        TestJobType::Scenario,
        Uuid::new_v4(),
        env2.id,
        auth2.user_id,
        config.clone(),
    );
    app.state.job_queue.enqueue(job).await.unwrap();

    // User 1 should only see their own jobs
    let response = app
        .server
        .get("/api/jobs")
        .add_header("Authorization", auth1.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["data"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_get_job_status() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let env = factory.create_environment(project.id, auth.user_id).await;

    let config = TestJobConfig::default();
    let job = TestJob::new(
        TestJobType::Scenario,
        Uuid::new_v4(),
        env.id,
        auth.user_id,
        config,
    );
    let job_id = app.state.job_queue.enqueue(job).await.unwrap();

    let response = app
        .server
        .get(&format!("/api/jobs/{}", job_id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["job_id"].as_str().unwrap(), job_id.to_string());
    assert_eq!(body["status"].as_str().unwrap(), "pending");
}

#[tokio::test]
async fn test_get_job_not_found() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;

    let fake_id = Uuid::new_v4();
    let response = app
        .server
        .get(&format!("/api/jobs/{}", fake_id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_job_other_user() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    // User 1 creates a job
    let auth1 = factory.create_user().await;
    let project = factory.create_project(auth1.user_id).await;
    let env = factory.create_environment(project.id, auth1.user_id).await;

    let config = TestJobConfig::default();
    let job = TestJob::new(
        TestJobType::Scenario,
        Uuid::new_v4(),
        env.id,
        auth1.user_id,
        config,
    );
    let job_id = app.state.job_queue.enqueue(job).await.unwrap();

    // User 2 tries to access it
    let auth2 = factory.create_user().await;
    let response = app
        .server
        .get(&format!("/api/jobs/{}", job_id))
        .add_header("Authorization", auth2.auth_header())
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_cancel_job() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;
    let project = factory.create_project(auth.user_id).await;
    let env = factory.create_environment(project.id, auth.user_id).await;

    let config = TestJobConfig::default();
    let job = TestJob::new(
        TestJobType::Scenario,
        Uuid::new_v4(),
        env.id,
        auth.user_id,
        config,
    );
    let job_id = app.state.job_queue.enqueue(job).await.unwrap();

    let response = app
        .server
        .delete(&format!("/api/jobs/{}", job_id))
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["status"].as_str().unwrap(), "cancelled");
}

#[tokio::test]
async fn test_cancel_job_other_user() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);

    let auth1 = factory.create_user().await;
    let project = factory.create_project(auth1.user_id).await;
    let env = factory.create_environment(project.id, auth1.user_id).await;

    let config = TestJobConfig::default();
    let job = TestJob::new(
        TestJobType::Scenario,
        Uuid::new_v4(),
        env.id,
        auth1.user_id,
        config,
    );
    let job_id = app.state.job_queue.enqueue(job).await.unwrap();

    let auth2 = factory.create_user().await;
    let response = app
        .server
        .delete(&format!("/api/jobs/{}", job_id))
        .add_header("Authorization", auth2.auth_header())
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_queue_stats() {
    let app = TestApp::new().await;
    let factory = Factory::new(&app.state);
    let auth = factory.create_user().await;

    let response = app
        .server
        .get("/api/jobs/stats")
        .add_header("Authorization", auth.auth_header())
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert!(body["queue_length"].as_i64().is_some());
}

#[tokio::test]
async fn test_job_unauthorized() {
    let app = TestApp::new().await;

    let response = app.server.get("/api/jobs").await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}
