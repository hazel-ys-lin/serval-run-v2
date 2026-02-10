use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    // Database
    pub database_url: String,
    pub mongodb_url: String,
    pub mongodb_database: String,
    pub redis_url: String,

    // JWT
    pub jwt_secret: String,
    pub jwt_expiration_hours: i64,

    // Server
    pub host: String,
    pub port: u16,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok(); // Load .env file if exists

        Ok(Self {
            // Database
            database_url: env::var("DATABASE_URL")
                .map_err(|_| ConfigError::Missing("DATABASE_URL"))?,
            mongodb_url: env::var("MONGODB_URL")
                .map_err(|_| ConfigError::Missing("MONGODB_URL"))?,
            mongodb_database: env::var("MONGODB_DATABASE")
                .unwrap_or_else(|_| "serval_run".to_string()),
            redis_url: env::var("REDIS_URL").map_err(|_| ConfigError::Missing("REDIS_URL"))?,

            // JWT
            jwt_secret: env::var("JWT_SECRET").map_err(|_| ConfigError::Missing("JWT_SECRET"))?,
            jwt_expiration_hours: env::var("JWT_EXPIRATION_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()
                .map_err(|_| ConfigError::Invalid("JWT_EXPIRATION_HOURS"))?,

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
