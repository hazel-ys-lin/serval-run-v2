use std::env;

/// Which feature surface to bring up at startup.
///
/// - `Full`: the v0.1.0 shape — Postgres + MongoDB + Redis. The job
///   queue runs on Redis (`RedisQueue`). Required for team / shared
///   deployments and the docker-compose stack.
/// - `Lite`: aims to drop the docker dependency entirely. SQL backend
///   becomes optional (typically SQLite), Mongo / Redis are skipped,
///   the queue runs in-process (`InMemoryQueue`). Wiring lands across
///   the rest of Phase 0 / PR-B; selecting Lite today returns a clear
///   "not yet implemented" error at startup.
///
/// Selected via the `SERVAL_MODE` env var; default is `Full` so existing
/// deployments are unaffected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppMode {
    #[default]
    Full,
    Lite,
}

impl AppMode {
    /// Read `SERVAL_MODE` from env. Unknown values fall through to `Full`
    /// so a typo doesn't silently flip a production server into lite.
    pub fn from_env() -> Self {
        match env::var("SERVAL_MODE").ok().as_deref() {
            Some("lite") => AppMode::Lite,
            _ => AppMode::Full,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    /// Runtime mode.
    pub mode: AppMode,

    // Database
    pub database_url: String,
    pub mongodb_url: String,
    pub mongodb_database: String,
    pub redis_url: String,

    // JWT
    pub jwt_secret: String,
    pub jwt_expiration_hours: i64,
    pub refresh_token_expiration_days: i64,

    // Server
    pub host: String,
    pub port: u16,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok(); // Load .env file if exists

        let mode = AppMode::from_env();

        // In Lite mode, MONGODB_URL / REDIS_URL aren't reached (Mongo and
        // Redis aren't connected). Tolerate them being absent so a user
        // who has uninstalled their docker stack can still parse Config;
        // AppState then decides whether the chosen mode is implemented.
        let mongodb_url = match mode {
            AppMode::Full => {
                env::var("MONGODB_URL").map_err(|_| ConfigError::Missing("MONGODB_URL"))?
            }
            AppMode::Lite => env::var("MONGODB_URL").unwrap_or_default(),
        };
        let redis_url = match mode {
            AppMode::Full => {
                env::var("REDIS_URL").map_err(|_| ConfigError::Missing("REDIS_URL"))?
            }
            AppMode::Lite => env::var("REDIS_URL").unwrap_or_default(),
        };

        Ok(Self {
            mode,

            // Database
            database_url: env::var("DATABASE_URL")
                .map_err(|_| ConfigError::Missing("DATABASE_URL"))?,
            mongodb_url,
            mongodb_database: env::var("MONGODB_DATABASE")
                .unwrap_or_else(|_| "serval_run".to_string()),
            redis_url,

            // JWT
            jwt_secret: env::var("JWT_SECRET").map_err(|_| ConfigError::Missing("JWT_SECRET"))?,
            jwt_expiration_hours: env::var("JWT_EXPIRATION_HOURS")
                .unwrap_or_else(|_| "1".to_string())
                .parse()
                .map_err(|_| ConfigError::Invalid("JWT_EXPIRATION_HOURS"))?,
            refresh_token_expiration_days: env::var("REFRESH_TOKEN_EXPIRATION_DAYS")
                .unwrap_or_else(|_| "7".to_string())
                .parse()
                .map_err(|_| ConfigError::Invalid("REFRESH_TOKEN_EXPIRATION_DAYS"))?,

            // Server
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .map_err(|_| ConfigError::Invalid("PORT"))?,
        })
    }

    /// Get server address as "host:port"
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing environment variable: {0}")]
    Missing(&'static str),

    #[error("Invalid environment variable: {0}")]
    Invalid(&'static str),
}
