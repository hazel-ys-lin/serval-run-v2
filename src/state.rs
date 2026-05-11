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
use crate::queue::{JobQueue, RedisQueue};

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
    pub mongo_client: MongoClient,
    pub redis: RedisConnectionManager,
    pub config: Config,
    /// Job queue for async test execution
    pub job_queue: Arc<dyn JobQueue>,
}

impl AppState {
    /// Create a new AppState by connecting to all databases
    pub async fn new(config: Config) -> Result<Self, AppStateError> {
        reject_unimplemented_mode(config.mode)?;
        let (pool, db) = connect_db(&config.database_url).await?;

        // Connect to MongoDB
        let mongo_client = MongoClient::with_uri_str(&config.mongodb_url)
            .await
            .map_err(|e| AppStateError::Mongo(e.to_string()))?;

        // Connect to Redis
        let redis = connect_redis(&config.redis_url).await?;

        // Create job queue using Redis
        let job_queue: Arc<dyn JobQueue> = Arc::new(RedisQueue::new(redis.clone()));

        Ok(Self {
            db,
            pool,
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
        reject_unimplemented_mode(config.mode)?;
        let (pool, db) = connect_db(&config.database_url).await?;

        // Connect to MongoDB
        let mongo_client = MongoClient::with_uri_str(&config.mongodb_url)
            .await
            .map_err(|e| AppStateError::Mongo(e.to_string()))?;

        // Connect to Redis
        let redis = connect_redis(&config.redis_url).await?;

        Ok(Self {
            db,
            pool,
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

/// Gate AppState construction by mode while lite mode is still being
/// wired up. Removed once the Lite branch wires Mongo/Redis skipping
/// and the in-memory queue in a follow-up commit.
fn reject_unimplemented_mode(mode: AppMode) -> Result<(), AppStateError> {
    match mode {
        AppMode::Full => Ok(()),
        AppMode::Lite => Err(AppStateError::Mode(
            "lite mode is selected via SERVAL_MODE=lite, but its wiring \
             (skip Mongo/Redis, use InMemoryQueue) is not in this commit \
             yet; planned for the next Phase 0 commit. \
             For now run with SERVAL_MODE=full (or unset)."
                .to_string(),
        )),
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

    let db = Database::connect(opt)
        .await
        .map_err(|e| match backend {
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

    #[error("Mode error: {0}")]
    Mode(String),

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
