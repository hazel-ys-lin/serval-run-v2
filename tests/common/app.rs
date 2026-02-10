use std::sync::Arc;

use axum_test::TestServer;
use serval_run::build_router;
use serval_run::config::Config;
use serval_run::queue::InMemoryQueue;
use serval_run::state::AppState;

/// Test configuration
pub fn test_config() -> Config {
    dotenvy::dotenv().ok();

    Config {
        database_url: std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@localhost:5432/serval_test".to_string()
        }),
        mongodb_url: std::env::var("MONGODB_URL")
            .unwrap_or_else(|_| "mongodb://localhost:27017".to_string()),
        redis_url: std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
        jwt_secret: "test-jwt-secret-that-is-at-least-32-characters-long".to_string(),
        jwt_expiration_hours: 24,
        host: "127.0.0.1".to_string(),
        port: 0,
    }
}

/// Test application wrapper
pub struct TestApp {
    pub server: TestServer,
    pub state: AppState,
}

impl TestApp {
    /// Create a new test application
    pub async fn new() -> Self {
        let config = test_config();

        // Use InMemoryQueue for testing (avoids Redis dependency in tests)
        let queue = Arc::new(InMemoryQueue::new());

        let state = AppState::with_queue(config, queue)
            .await
            .expect("Failed to create test app state");

        let router = build_router(state.clone());
        let server = TestServer::new(router).expect("Failed to create test server");

        Self { server, state }
    }
}
