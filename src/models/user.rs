use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)] // Never expose password hash
    pub password_hash: String,
    pub name: String,
    pub job_title: Option<String>,
    pub role: i16,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// User creation DTO (without id and timestamps)
#[derive(Debug, Deserialize)]
pub struct CreateUser {
    pub email: String,
    pub password: String,
    pub name: String,
    pub job_title: Option<String>,
}

/// User update DTO
#[derive(Debug, Deserialize)]
pub struct UpdateUser {
    pub name: Option<String>,
    pub job_title: Option<String>,
}

/// Public user response (safe to return via API)
#[derive(Debug, Serialize, ToSchema)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub job_title: Option<String>,
    pub role: i16,
    pub created_at: OffsetDateTime,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            name: user.name,
            job_title: user.job_title,
            role: user.role,
            created_at: user.created_at,
        }
    }
}
