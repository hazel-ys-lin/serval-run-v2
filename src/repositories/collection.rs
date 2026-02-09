use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use uuid::Uuid;

use crate::entity::collection::{self, ActiveModel, Column, Entity as CollectionEntity};
use crate::entity::project::{Column as ProjectColumn, Entity as ProjectEntity};
use crate::error::{AppError, AppResult};
use crate::models::{Collection, CreateCollection, UpdateCollection};
use crate::repositories::Repository;

/// Collection repository for database operations
pub struct CollectionRepository;

#[async_trait]
impl Repository<Collection> for CollectionRepository {
    async fn find_by_id(db: &DatabaseConnection, id: Uuid) -> AppResult<Collection> {
        let model = CollectionEntity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Collection".to_string()))?;

        Ok(model.into())
    }

    async fn delete(db: &DatabaseConnection, id: Uuid) -> AppResult<()> {
        let result = CollectionEntity::delete_by_id(id).exec(db).await?;

        if result.rows_affected == 0 {
            return Err(AppError::NotFound("Collection".to_string()));
        }

        Ok(())
    }

    async fn list(db: &DatabaseConnection, limit: u64, offset: u64) -> AppResult<Vec<Collection>> {
        let models = CollectionEntity::find()
            .order_by_desc(Column::CreatedAt)
            .paginate(db, limit)
            .fetch_page(offset / limit)
            .await?;

        Ok(models.into_iter().map(|m| m.into()).collect())
    }

    async fn count(db: &DatabaseConnection) -> AppResult<u64> {
        let count = CollectionEntity::find().count(db).await?;
        Ok(count)
    }
}

impl CollectionRepository {
    /// Create a new collection
    pub async fn create(
        db: &DatabaseConnection,
        project_id: Uuid,
        user_id: Uuid,
        input: &CreateCollection,
    ) -> AppResult<Collection> {
        // Verify project ownership
        Self::verify_project_ownership(db, project_id, user_id).await?;

        let model = ActiveModel {
            id: Set(Uuid::new_v4()),
            project_id: Set(project_id),
            name: Set(input.name.clone()),
            description: Set(input.description.clone()),
            created_at: Set(time::OffsetDateTime::now_utc()),
            updated_at: Set(time::OffsetDateTime::now_utc()),
        };

        let result = model.insert(db).await?;
        Ok(result.into())
    }

    /// Find collection by ID with project ownership verification
    pub async fn find_by_id_and_user(
        db: &DatabaseConnection,
        id: Uuid,
        user_id: Uuid,
    ) -> AppResult<Collection> {
        let model = CollectionEntity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Collection".to_string()))?;

        // Verify project ownership
        Self::verify_project_ownership(db, model.project_id, user_id).await?;

        Ok(model.into())
    }

    /// List collections for a specific project (with ownership check)
    pub async fn list_by_project(
        db: &DatabaseConnection,
        project_id: Uuid,
        user_id: Uuid,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<Collection>> {
        // Verify project ownership
        Self::verify_project_ownership(db, project_id, user_id).await?;

        let models = CollectionEntity::find()
            .filter(Column::ProjectId.eq(project_id))
            .order_by_desc(Column::CreatedAt)
            .paginate(db, limit)
            .fetch_page(offset / limit)
            .await?;

        Ok(models.into_iter().map(|m| m.into()).collect())
    }

    /// Count collections for a specific project
    pub async fn count_by_project(
        db: &DatabaseConnection,
        project_id: Uuid,
        user_id: Uuid,
    ) -> AppResult<u64> {
        // Verify project ownership
        Self::verify_project_ownership(db, project_id, user_id).await?;

        let count = CollectionEntity::find()
            .filter(Column::ProjectId.eq(project_id))
            .count(db)
            .await?;

        Ok(count)
    }

    /// Update collection (with ownership check)
    pub async fn update(
        db: &DatabaseConnection,
        id: Uuid,
        user_id: Uuid,
        input: &UpdateCollection,
    ) -> AppResult<Collection> {
        let model = CollectionEntity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Collection".to_string()))?;

        // Verify project ownership
        Self::verify_project_ownership(db, model.project_id, user_id).await?;

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

    /// Delete collection (with ownership check)
    pub async fn delete_by_user(db: &DatabaseConnection, id: Uuid, user_id: Uuid) -> AppResult<()> {
        let model = CollectionEntity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Collection".to_string()))?;

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
impl From<collection::Model> for Collection {
    fn from(m: collection::Model) -> Self {
        Self {
            id: m.id,
            project_id: m.project_id,
            name: m.name,
            description: m.description,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}
