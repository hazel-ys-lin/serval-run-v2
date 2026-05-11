use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use mongodb::Client as MongoClient;
use redis::aio::{ConnectionManager as RedisConnectionManager, ConnectionManagerConfig};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sqlx::postgres::PgPool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};

use crate::config::{AppMode, Config};
use crate::db::{detect_backend, DbBackend, SqlxPool};
use crate::queue::{InMemoryQueue, JobQueue, RedisQueue};

/// Response timeout for the shared Redis connection manager.
///
/// Must be longer than the worker's BLPOP timeout (currently 5s in
/// `worker::main::main` -> `dequeue(5)`); otherwise the manager cancels the
/// blocking command before Redis itself returns nil, surfacing as
/// `Redis error: timed out` in worker logs.
///
/// Note: `ConnectionManager` is not an ideal home for long-blocking
/// commands like BLPOP. The proper fix is a dedicated non-pooled
/// connection for the worker queue. For now, raising the timeout
/// unblocks the worker without restructuring the queue plumbing.
const REDIS_RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    /// SeaORM database connection (primary for queries; routes by URL scheme).
    pub db: DatabaseConnection,
    /// SQLx pool for migration setup and graceful shutdown.
    ///
    /// Tagged with its backend so the right migration directory is selected
    /// and so shutdown can call the variant's `close()`.
    pub pool: SqlxPool,
    /// MongoDB client, present in Full mode. Lite mode leaves this `None`
    /// and Mongo-writing call sites skip the write (all such writes are
    /// already non-fatal in Full mode, so the behaviour matches).
    pub mongo_client: Option<MongoClient>,
    /// Redis connection manager, present in Full mode. Lite mode leaves
    /// this `None`; the job queue is then served by `InMemoryQueue` and
    /// the health check reports redis as `"not_configured"`.
    pub redis: Option<RedisConnectionManager>,
    pub config: Config,
    /// Job queue for async test execution
    pub job_queue: Arc<dyn JobQueue>,
}

impl AppState {
    /// Create a new AppState by connecting to the services the chosen
    /// mode requires.
    ///
    /// Full mode brings up Postgres (or SQLite, by URL scheme) + MongoDB +
    /// Redis, with a `RedisQueue` for async jobs. Lite mode brings up only
    /// the SQL backend (typically SQLite), skips MongoDB and Redis, and
    /// runs an `InMemoryQueue` in-process.
    pub async fn new(config: Config) -> Result<Self, AppStateError> {
        let (pool, db) = connect_db(&config.database_url).await?;

        let (mongo_client, redis, job_queue) = match config.mode {
            AppMode::Full => {
                let mongo_client = MongoClient::with_uri_str(&config.mongodb_url)
                    .await
                    .map_err(|e| AppStateError::Mongo(e.to_string()))?;
                let redis = connect_redis(&config.redis_url).await?;
                let queue: Arc<dyn JobQueue> = Arc::new(RedisQueue::new(redis.clone()));
                (Some(mongo_client), Some(redis), queue)
            }
            AppMode::Lite => {
                tracing::info!(
                    "Lite mode active: MongoDB and Redis not connected; using InMemoryQueue"
                );
                let queue: Arc<dyn JobQueue> = Arc::new(InMemoryQueue::new());
                (None, None, queue)
            }
        };

        Ok(Self {
            db,
            pool,
            mongo_client,
            redis,
            config,
            job_queue,
        })
    }

    /// Create AppState with a custom queue (for testing).
    ///
    /// Honours `config.mode`: Lite skips Mongo/Redis the same way as
    /// `new()`. The caller still supplies the queue (typically an
    /// `InMemoryQueue`) so tests stay deterministic.
    #[allow(dead_code)]
    pub async fn with_queue(
        config: Config,
        job_queue: Arc<dyn JobQueue>,
    ) -> Result<Self, AppStateError> {
        let (pool, db) = connect_db(&config.database_url).await?;

        let (mongo_client, redis) = match config.mode {
            AppMode::Full => {
                let mongo_client = MongoClient::with_uri_str(&config.mongodb_url)
                    .await
                    .map_err(|e| AppStateError::Mongo(e.to_string()))?;
                let redis = connect_redis(&config.redis_url).await?;
                (Some(mongo_client), Some(redis))
            }
            AppMode::Lite => (None, None),
        };

        Ok(Self {
            db,
            pool,
            mongo_client,
            redis,
            config,
            job_queue,
        })
    }

    /// Get MongoDB database when available. Returns `None` under Lite mode
    /// (Mongo is not connected); callers that previously assumed the
    /// presence of a database now skip the write in that case.
    pub fn mongo_db(&self) -> Option<mongodb::Database> {
        self.mongo_client
            .as_ref()
            .map(|c| c.database(&self.config.mongodb_database))
    }
}

/// Connect the SQL backend implied by `database_url`, run its migrations,
/// and return both the SQLx pool (for shutdown) and the SeaORM connection
/// (for queries).
///
/// Migrations live under `migrations/<backend>/`. The two backends track
/// their own `_sqlx_migrations` tables independently, so checksum drift
/// in one does not affect the other.
async fn connect_db(database_url: &str) -> Result<(SqlxPool, DatabaseConnection), AppStateError> {
    let backend =
        detect_backend(database_url).map_err(|e| AppStateError::Backend(e.to_string()))?;

    let pool = match backend {
        DbBackend::Postgres => {
            let pg_pool = PgPool::connect(database_url)
                .await
                .map_err(|e| AppStateError::Postgres(e.to_string()))?;

            sqlx::migrate!("./migrations/postgres")
                .run(&pg_pool)
                .await
                .map_err(|e| AppStateError::Migration(e.to_string()))?;

            SqlxPool::Postgres(pg_pool)
        }
        DbBackend::Sqlite => {
            // foreign_keys ON: SQLite defaults to OFF, but our migrations
            // depend on ON DELETE CASCADE / SET NULL to behave like Postgres.
            // create_if_missing: spare lite-mode users a manual `touch`
            // before the first run.
            let options = SqliteConnectOptions::from_str(database_url)
                .map_err(|e| AppStateError::Sqlite(e.to_string()))?
                .foreign_keys(true)
                .create_if_missing(true);

            let sqlite_pool = SqlitePool::connect_with(options)
                .await
                .map_err(|e| AppStateError::Sqlite(e.to_string()))?;

            sqlx::migrate!("./migrations/sqlite")
                .run(&sqlite_pool)
                .await
                .map_err(|e| AppStateError::Migration(e.to_string()))?;

            SqlxPool::Sqlite(sqlite_pool)
        }
    };

    // SeaORM dispatches to the right driver from the URL scheme, so the
    // same ConnectOptions works for both backends. Connection-pool tuning
    // is Postgres-shaped for now; revisit if SQLite contention shows up.
    let mut opt = ConnectOptions::new(database_url);
    opt.max_connections(100)
        .min_connections(5)
        .sqlx_logging(true);

    let db = Database::connect(opt).await.map_err(|e| match backend {
        DbBackend::Postgres => AppStateError::Postgres(e.to_string()),
        DbBackend::Sqlite => AppStateError::Sqlite(e.to_string()),
    })?;

    Ok((pool, db))
}

/// Connect the shared Redis connection manager used by the job queue.
async fn connect_redis(redis_url: &str) -> Result<RedisConnectionManager, AppStateError> {
    let redis_client =
        redis::Client::open(redis_url).map_err(|e| AppStateError::Redis(e.to_string()))?;
    let manager_config =
        ConnectionManagerConfig::new().set_response_timeout(Some(REDIS_RESPONSE_TIMEOUT));
    RedisConnectionManager::new_with_config(redis_client, manager_config)
        .await
        .map_err(|e| AppStateError::Redis(e.to_string()))
}

#[derive(Debug, thiserror::Error)]
pub enum AppStateError {
    #[error("Backend selection error: {0}")]
    Backend(String),

    #[error("PostgreSQL connection error: {0}")]
    Postgres(String),

    #[error("SQLite connection error: {0}")]
    Sqlite(String),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("MongoDB connection error: {0}")]
    Mongo(String),

    #[error("Redis connection error: {0}")]
    Redis(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test: bring up an in-memory SQLite, run the SQLite migration
    /// directory through `connect_db`, and confirm a couple of expected
    /// tables exist. Catches both gross migration regressions (a `.sql`
    /// file fails to parse / execute) and silent dispatch breakage (the
    /// returned pool isn't actually the Sqlite variant).
    ///
    /// Independent of Postgres / Mongo / Redis — runs from `cargo test --lib`
    /// without any docker stack.
    /// Build a minimal Config suitable for unit-level AppState tests.
    /// Everything Lite mode doesn't reach (Mongo, Redis URLs) is set
    /// to empty so this stays a single-process test with no docker.
    fn lite_config(database_url: &str) -> Config {
        Config {
            mode: AppMode::Lite,
            database_url: database_url.to_string(),
            mongodb_url: String::new(),
            mongodb_database: "serval_run".to_string(),
            redis_url: String::new(),
            jwt_secret: "test-jwt-secret-that-is-at-least-32-characters-long".to_string(),
            jwt_expiration_hours: 24,
            refresh_token_expiration_days: 7,
            host: "127.0.0.1".to_string(),
            port: 0,
        }
    }

    /// End-to-end smoke test for lite mode: SQLite in-memory + no Mongo
    /// + no Redis + InMemoryQueue. Catches anything that wires Mongo or
    /// Redis unconditionally and would otherwise hang or panic when run
    /// without docker.
    #[tokio::test]
    async fn lite_mode_stands_up_appstate_without_docker() {
        let config = lite_config("sqlite::memory:");

        let state = AppState::new(config)
            .await
            .expect("AppState::new should succeed in lite mode with sqlite::memory:");

        assert!(state.mongo_client.is_none(), "lite mode should skip Mongo");
        assert!(state.redis.is_none(), "lite mode should skip Redis");
        assert!(
            state.mongo_db().is_none(),
            "mongo_db() must return None when mongo_client is None"
        );

        // Job queue should be functional even without Redis.
        assert_eq!(
            state.job_queue.queue_length().await.expect("queue_length"),
            0,
            "fresh InMemoryQueue should be empty"
        );

        assert_eq!(state.pool.backend(), DbBackend::Sqlite);
    }

    #[tokio::test]
    async fn sqlite_in_memory_runs_migrations() {
        let (pool, _db) = connect_db("sqlite::memory:")
            .await
            .expect("connect_db on sqlite::memory:");

        assert_eq!(
            pool.backend(),
            DbBackend::Sqlite,
            "expected Sqlite pool from sqlite::memory: URL"
        );

        let SqlxPool::Sqlite(sp) = &pool else {
            panic!("expected SqlxPool::Sqlite, got {:?}", pool.backend());
        };

        for table in [
            "users",
            "projects",
            "collections",
            "environments",
            "apis",
            "scenarios",
            "reports",
            "responses",
            "refresh_tokens",
        ] {
            let count: (i64,) = sqlx::query_as(&format!("SELECT count(*) FROM {table}"))
                .fetch_one(sp)
                .await
                .unwrap_or_else(|e| panic!("table {table} not queryable: {e}"));
            assert_eq!(count.0, 0, "table {table} should be empty on fresh schema");
        }
    }
}
