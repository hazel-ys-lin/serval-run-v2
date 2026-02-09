use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use uuid::Uuid;

use crate::entity::environment::{self, ActiveModel, Column, Entity as EnvironmentEntity};
use crate::entity::project::{Column as ProjectColumn, Entity as ProjectEntity};
use crate::error::{AppError, AppResult};
use crate::models::{CreateEnvironment, Environment, UpdateEnvironment};
use crate::repositories::Repository;

/// Environment repository for database operations
pub struct EnvironmentRepository;

#[async_trait]
impl Repository<Environment> for EnvironmentRepository {
    async fn find_by_id(db: &DatabaseConnection, id: Uuid) -> AppResult<Environment> {
        let model = EnvironmentEntity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Environment".to_string()))?;

        Ok(model.into())
    }

    async fn delete(db: &DatabaseConnection, id: Uuid) -> AppResult<()> {
        let result = EnvironmentEntity::delete_by_id(id).exec(db).await?;

        if result.rows_affected == 0 {
            return Err(AppError::NotFound("Environment".to_string()));
        }

        Ok(())
    }

    async fn list(db: &DatabaseConnection, limit: u64, offset: u64) -> AppResult<Vec<Environment>> {
        let models = EnvironmentEntity::find()
            .order_by_desc(Column::CreatedAt)
            .paginate(db, limit)
            .fetch_page(offset / limit)
            .await?;

        Ok(models.into_iter().map(|m| m.into()).collect())
    }

    async fn count(db: &DatabaseConnection) -> AppResult<u64> {
        let count = EnvironmentEntity::find().count(db).await?;
        Ok(count)
    }
}

impl EnvironmentRepository {
    /// Create a new environment
    pub async fn create(
        db: &DatabaseConnection,
        project_id: Uuid,
        user_id: Uuid,
        input: &CreateEnvironment,
    ) -> AppResult<Environment> {
        // Verify project ownership
        Self::verify_project_ownership(db, project_id, user_id).await?;

        let model = ActiveModel {
            id: Set(Uuid::new_v4()),
            project_id: Set(project_id),
            title: Set(input.title.clone()),
            domain_name: Set(input.domain_name.clone()),
            created_at: Set(time::OffsetDateTime::now_utc()),
            updated_at: Set(time::OffsetDateTime::now_utc()),
        };

        let result = model.insert(db).await?;
        Ok(result.into())
    }

    /// Find environment by ID with project ownership verification
    pub async fn find_by_id_and_user(
        db: &DatabaseConnection,
        id: Uuid,
        user_id: Uuid,
    ) -> AppResult<Environment> {
        let model = EnvironmentEntity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Environment".to_string()))?;

        // Verify project ownership
        Self::verify_project_ownership(db, model.project_id, user_id).await?;

        Ok(model.into())
    }

    /// List environments for a specific project (with ownership check)
    pub async fn list_by_project(
        db: &DatabaseConnection,
        project_id: Uuid,
        user_id: Uuid,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<Environment>> {
        // Verify project ownership
        Self::verify_project_ownership(db, project_id, user_id).await?;

        let models = EnvironmentEntity::find()
            .filter(Column::ProjectId.eq(project_id))
            .order_by_desc(Column::CreatedAt)
            .paginate(db, limit)
            .fetch_page(offset / limit)
            .await?;

        Ok(models.into_iter().map(|m| m.into()).collect())
    }

    /// Count environments for a specific project
    pub async fn count_by_project(
        db: &DatabaseConnection,
        project_id: Uuid,
        user_id: Uuid,
    ) -> AppResult<u64> {
        // Verify project ownership
        Self::verify_project_ownership(db, project_id, user_id).await?;

        let count = EnvironmentEntity::find()
            .filter(Column::ProjectId.eq(project_id))
            .count(db)
            .await?;

        Ok(count)
    }

    /// Update environment (with ownership check)
    pub async fn update(
        db: &DatabaseConnection,
        id: Uuid,
        user_id: Uuid,
        input: &UpdateEnvironment,
    ) -> AppResult<Environment> {
        let model = EnvironmentEntity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Environment".to_string()))?;

        // Verify project ownership
        Self::verify_project_ownership(db, model.project_id, user_id).await?;

        let mut active: ActiveModel = model.into();

        if let Some(title) = &input.title {
            active.title = Set(title.clone());
        }
        if let Some(domain_name) = &input.domain_name {
            active.domain_name = Set(domain_name.clone());
        }
        active.updated_at = Set(time::OffsetDateTime::now_utc());

        let result = active.update(db).await?;
        Ok(result.into())
    }

    /// Delete environment (with ownership check)
    pub async fn delete_by_user(db: &DatabaseConnection, id: Uuid, user_id: Uuid) -> AppResult<()> {
        let model = EnvironmentEntity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Environment".to_string()))?;

        // Verify project ownership
        Self::verify_project_ownership(db, model.project_id, user_id).await?;

        let active: ActiveModel = model.into();
        active.delete(db).await?;

        Ok(())
    }

    /// Verify that the user owns the project
    async fn verify_project_ownership(
        db: &DatabaseConnection,
        project_id: Uuid,
        user_id: Uuid,
    ) -> AppResult<()> {
        ProjectEntity::find_by_id(project_id)
            .filter(ProjectColumn::UserId.eq(user_id))
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Project".to_string()))?;

        Ok(())
    }
}

// Conversion from SeaORM model to our domain model
impl From<environment::Model> for Environment {
    fn from(m: environment::Model) -> Self {
        Self {
            id: m.id,
            project_id: m.project_id,
            title: m.title,
            domain_name: m.domain_name,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}
