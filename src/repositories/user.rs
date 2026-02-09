use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use uuid::Uuid;

use crate::entity::user::{self, ActiveModel, Column, Entity as UserEntity};
use crate::error::{AppError, AppResult};
use crate::models::{CreateUser, UpdateUser, User};
use crate::repositories::Repository;

/// User repository for database operations
pub struct UserRepository;

// Implement the base Repository trait
#[async_trait]
impl Repository<User> for UserRepository {
    async fn find_by_id(db: &DatabaseConnection, id: Uuid) -> AppResult<User> {
        let model = UserEntity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("User".to_string()))?;

        Ok(model.into())
    }

    async fn delete(db: &DatabaseConnection, id: Uuid) -> AppResult<()> {
        let result = UserEntity::delete_by_id(id).exec(db).await?;

        if result.rows_affected == 0 {
            return Err(AppError::NotFound("User".to_string()));
        }

        Ok(())
    }

    async fn list(db: &DatabaseConnection, limit: u64, offset: u64) -> AppResult<Vec<User>> {
        let models = UserEntity::find()
            .order_by_desc(Column::CreatedAt)
            .paginate(db, limit)
            .fetch_page(offset / limit)
            .await?;

        Ok(models.into_iter().map(|m| m.into()).collect())
    }

    async fn count(db: &DatabaseConnection) -> AppResult<u64> {
        let count = UserEntity::find().count(db).await?;
        Ok(count)
    }
}

// User-specific methods (not in the base trait)
impl UserRepository {
    /// Create a new user
    pub async fn create(
        db: &DatabaseConnection,
        input: &CreateUser,
        password_hash: &str,
    ) -> AppResult<User> {
        let model = ActiveModel {
            id: Set(Uuid::new_v4()),
            email: Set(input.email.clone()),
            password_hash: Set(password_hash.to_string()),
            name: Set(input.name.clone()),
            job_title: Set(input.job_title.clone()),
            role: Set(2), // default role
            created_at: Set(time::OffsetDateTime::now_utc()),
            updated_at: Set(time::OffsetDateTime::now_utc()),
        };

        let result = model.insert(db).await.map_err(|e| {
            if e.to_string().contains("duplicate key") || e.to_string().contains("unique") {
                AppError::Conflict("Email already exists".to_string())
            } else {
                AppError::Database(e.to_string())
            }
        })?;

        Ok(result.into())
    }

    /// Find user by email (for login)
    pub async fn find_by_email(db: &DatabaseConnection, email: &str) -> AppResult<User> {
        let model = UserEntity::find()
            .filter(Column::Email.eq(email))
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("User".to_string()))?;

        Ok(model.into())
    }

    /// Check if email exists
    pub async fn email_exists(db: &DatabaseConnection, email: &str) -> AppResult<bool> {
        let count = UserEntity::find()
            .filter(Column::Email.eq(email))
            .count(db)
            .await?;

        Ok(count > 0)
    }

    /// Update user
    pub async fn update(db: &DatabaseConnection, id: Uuid, input: &UpdateUser) -> AppResult<User> {
        let model = UserEntity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("User".to_string()))?;

        let mut active: ActiveModel = model.into();

        if let Some(name) = &input.name {
            active.name = Set(name.clone());
        }
        if let Some(job_title) = &input.job_title {
            active.job_title = Set(Some(job_title.clone()));
        }
        active.updated_at = Set(time::OffsetDateTime::now_utc());

        let result = active.update(db).await?;
        Ok(result.into())
    }
}

// Conversion from SeaORM model to our domain model
impl From<user::Model> for User {
    fn from(m: user::Model) -> Self {
        Self {
            id: m.id,
            email: m.email,
            password_hash: m.password_hash,
            name: m.name,
            job_title: m.job_title,
            role: m.role,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}
