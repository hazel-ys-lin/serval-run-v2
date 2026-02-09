use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use uuid::Uuid;

use crate::entity::project::{self, ActiveModel, Column, Entity as ProjectEntity};
use crate::error::{AppError, AppResult};
use crate::models::{CreateProject, Project, UpdateProject};
use crate::repositories::Repository;

/// Project repository for database operations
pub struct ProjectRepository;

#[async_trait]
impl Repository<Project> for ProjectRepository {
    async fn find_by_id(db: &DatabaseConnection, id: Uuid) -> AppResult<Project> {
        let model = ProjectEntity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Project".to_string()))?;

        Ok(model.into())
    }

    async fn delete(db: &DatabaseConnection, id: Uuid) -> AppResult<()> {
        let result = ProjectEntity::delete_by_id(id).exec(db).await?;

        if result.rows_affected == 0 {
            return Err(AppError::NotFound("Project".to_string()));
        }

        Ok(())
    }

    async fn list(db: &DatabaseConnection, limit: u64, offset: u64) -> AppResult<Vec<Project>> {
        let models = ProjectEntity::find()
            .order_by_desc(Column::CreatedAt)
            .paginate(db, limit)
            .fetch_page(offset / limit)
            .await?;

        Ok(models.into_iter().map(|m| m.into()).collect())
    }

    async fn count(db: &DatabaseConnection) -> AppResult<u64> {
        let count = ProjectEntity::find().count(db).await?;
        Ok(count)
    }
}

impl ProjectRepository {
    /// Create a new project
    pub async fn create(
        db: &DatabaseConnection,
        user_id: Uuid,
        input: &CreateProject,
    ) -> AppResult<Project> {
        let model = ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            name: Set(input.name.clone()),
            description: Set(input.description.clone()),
            created_at: Set(time::OffsetDateTime::now_utc()),
            updated_at: Set(time::OffsetDateTime::now_utc()),
        };

        let result = model.insert(db).await?;
        Ok(result.into())
    }

    /// Find project by ID and verify ownership
    pub async fn find_by_id_and_user(
        db: &DatabaseConnection,
        id: Uuid,
        user_id: Uuid,
    ) -> AppResult<Project> {
        let model = ProjectEntity::find_by_id(id)
            .filter(Column::UserId.eq(user_id))
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Project".to_string()))?;

        Ok(model.into())
    }

    /// List projects for a specific user
    pub async fn list_by_user(
        db: &DatabaseConnection,
        user_id: Uuid,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<Project>> {
        let models = ProjectEntity::find()
            .filter(Column::UserId.eq(user_id))
            .order_by_desc(Column::CreatedAt)
            .paginate(db, limit)
            .fetch_page(offset / limit)
            .await?;

        Ok(models.into_iter().map(|m| m.into()).collect())
    }

    /// Count projects for a specific user
    pub async fn count_by_user(db: &DatabaseConnection, user_id: Uuid) -> AppResult<u64> {
        let count = ProjectEntity::find()
            .filter(Column::UserId.eq(user_id))
            .count(db)
            .await?;

        Ok(count)
    }

    /// Update project (with ownership check)
    pub async fn update(
        db: &DatabaseConnection,
        id: Uuid,
        user_id: Uuid,
        input: &UpdateProject,
    ) -> AppResult<Project> {
        let model = ProjectEntity::find_by_id(id)
            .filter(Column::UserId.eq(user_id))
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Project".to_string()))?;

        let mut active: ActiveModel = model.into();

        if let Some(name) = &input.name {
            active.name = Set(name.clone());
        }
        if let Some(description) = &input.description {
            active.description = Set(Some(description.clone()));
        }
        active.updated_at = Set(time::OffsetDateTime::now_utc());

        let result = active.update(db).await?;
        Ok(result.into())
    }

    /// Delete project (with ownership check)
    pub async fn delete_by_user(db: &DatabaseConnection, id: Uuid, user_id: Uuid) -> AppResult<()> {
        // First verify ownership
        let model = ProjectEntity::find_by_id(id)
            .filter(Column::UserId.eq(user_id))
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Project".to_string()))?;

        // Then delete
        let active: ActiveModel = model.into();
        active.delete(db).await?;

        Ok(())
    }
}

// Conversion from SeaORM model to our domain model
impl From<project::Model> for Project {
    fn from(m: project::Model) -> Self {
        Self {
            id: m.id,
            user_id: m.user_id,
            name: m.name,
            description: m.description,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}
