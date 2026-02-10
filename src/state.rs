use std::sync::Arc;

use mongodb::Client as MongoClient;
use redis::aio::ConnectionManager as RedisConnectionManager;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sqlx::postgres::PgPool;

use crate::config::Config;
use crate::queue::{JobQueue, RedisQueue};

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    /// SeaORM database connection (primary for queries)
    pub db: DatabaseConnection,
    /// SQLx pool for migrations only
    pub pg_pool: PgPool,
    pub mongo_client: MongoClient,
    pub redis: RedisConnectionManager,
    pub config: Config,
    /// Job queue for async test execution
    pub job_queue: Arc<dyn JobQueue>,
}

impl AppState {
    /// Create a new AppState by connecting to all databases
    pub async fn new(config: Config) -> Result<Self, AppStateError> {
        // Connect to PostgreSQL with SQLx (for migrations)
        let pg_pool = PgPool::connect(&config.database_url)
            .await
            .map_err(|e| AppStateError::Postgres(e.to_string()))?;

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pg_pool)
            .await
            .map_err(|e| AppStateError::Migration(e.to_string()))?;

        // Connect to PostgreSQL with SeaORM
        let mut opt = ConnectOptions::new(&config.database_url);
        opt.max_connections(100)
            .min_connections(5)
            .sqlx_logging(true);

        let db = Database::connect(opt)
            .await
            .map_err(|e| AppStateError::Postgres(e.to_string()))?;

        // Connect to MongoDB
        let mongo_client = MongoClient::with_uri_str(&config.mongodb_url)
            .await
            .map_err(|e| AppStateError::Mongo(e.to_string()))?;

        // Connect to Redis
        let redis_client = redis::Client::open(config.redis_url.as_str())
            .map_err(|e| AppStateError::Redis(e.to_string()))?;
        let redis = RedisConnectionManager::new(redis_client)
            .await
            .map_err(|e| AppStateError::Redis(e.to_string()))?;

        // Create job queue using Redis
        let job_queue: Arc<dyn JobQueue> = Arc::new(RedisQueue::new(redis.clone()));

        Ok(Self {
            db,
            pg_pool,
            mongo_client,
            redis,
            config,
            job_queue,
        })
    }

    /// Create AppState with a custom queue (for testing)
    #[allow(dead_code)]
    pub async fn with_queue(
        config: Config,
        job_queue: Arc<dyn JobQueue>,
    ) -> Result<Self, AppStateError> {
        // Connect to PostgreSQL with SQLx (for migrations)
        let pg_pool = PgPool::connect(&config.database_url)
            .await
            .map_err(|e| AppStateError::Postgres(e.to_string()))?;

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pg_pool)
            .await
            .map_err(|e| AppStateError::Migration(e.to_string()))?;

        // Connect to PostgreSQL with SeaORM
        let mut opt = ConnectOptions::new(&config.database_url);
        opt.max_connections(100)
            .min_connections(5)
            .sqlx_logging(true);

        let db = Database::connect(opt)
            .await
            .map_err(|e| AppStateError::Postgres(e.to_string()))?;

        // Connect to MongoDB
        let mongo_client = MongoClient::with_uri_str(&config.mongodb_url)
            .await
            .map_err(|e| AppStateError::Mongo(e.to_string()))?;

        // Connect to Redis
        let redis_client = redis::Client::open(config.redis_url.as_str())
            .map_err(|e| AppStateError::Redis(e.to_string()))?;
        let redis = RedisConnectionManager::new(redis_client)
            .await
            .map_err(|e| AppStateError::Redis(e.to_string()))?;

        Ok(Self {
            db,
            pg_pool,
            mongo_client,
            redis,
            config,
            job_queue,
        })
    }

    /// Get MongoDB database (configurable via MONGODB_DATABASE env var)
    pub fn mongo_db(&self) -> mongodb::Database {
        self.mongo_client.database(&self.config.mongodb_database)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppStateError {
    #[error("PostgreSQL connection error: {0}")]
    Postgres(String),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("MongoDB connection error: {0}")]
    Mongo(String),

    #[error("Redis connection error: {0}")]
    Redis(String),
}
