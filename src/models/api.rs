use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Api {
    pub id: Uuid,
    pub collection_id: Uuid,
    pub name: String,
    pub http_method: String,  // GET, POST, PUT, DELETE, PATCH, etc.
    pub endpoint: String,
    pub severity: i16,
    pub description: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateApi {
    pub name: String,
    pub http_method: String,
    pub endpoint: String,
    pub severity: Option<i16>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateApi {
    pub name: Option<String>,
    pub http_method: Option<String>,
    pub endpoint: Option<String>,
    pub severity: Option<i16>,
    pub description: Option<String>,
}
