use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Environment {
    pub id: Uuid,
    pub project_id: Uuid,
    pub title: String,        // e.g., "dev", "staging", "production"
    pub domain_name: String,  // base URL for this environment
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateEnvironment {
    pub title: String,
    pub domain_name: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEnvironment {
    pub title: Option<String>,
    pub domain_name: Option<String>,
}
