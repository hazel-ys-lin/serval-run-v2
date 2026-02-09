use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use uuid::Uuid;

use crate::entity::api::{self, ActiveModel, Column, Entity as ApiEntity};
use crate::entity::collection::Entity as CollectionEntity;
use crate::entity::project::{Column as ProjectColumn, Entity as ProjectEntity};
use crate::error::{AppError, AppResult};
use crate::models::{Api, CreateApi, UpdateApi};
use crate::repositories::Repository;

/// API repository for database operations
pub struct ApiRepository;

#[async_trait]
impl Repository<Api> for ApiRepository {
    async fn find_by_id(db: &DatabaseConnection, id: Uuid) -> AppResult<Api> {
        let model = ApiEntity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Api".to_string()))?;

        Ok(model.into())
    }

    async fn delete(db: &DatabaseConnection, id: Uuid) -> AppResult<()> {
        let result = ApiEntity::delete_by_id(id).exec(db).await?;

        if result.rows_affected == 0 {
            return Err(AppError::NotFound("Api".to_string()));
        }

        Ok(())
    }

    async fn list(db: &DatabaseConnection, limit: u64, offset: u64) -> AppResult<Vec<Api>> {
        let models = ApiEntity::find()
            .order_by_desc(Column::CreatedAt)
            .paginate(db, limit)
            .fetch_page(offset / limit)
            .await?;

        Ok(models.into_iter().map(|m| m.into()).collect())
    }

    async fn count(db: &DatabaseConnection) -> AppResult<u64> {
        let count = ApiEntity::find().count(db).await?;
        Ok(count)
    }
}

impl ApiRepository {
    /// Create a new API
    pub async fn create(
        db: &DatabaseConnection,
        collection_id: Uuid,
        user_id: Uuid,
        input: &CreateApi,
    ) -> AppResult<Api> {
        // Verify collection ownership (which verifies project -> user)
        Self::verify_collection_ownership(db, collection_id, user_id).await?;

        let model = ActiveModel {
            id: Set(Uuid::new_v4()),
            collection_id: Set(collection_id),
            name: Set(input.name.clone()),
            http_method: Set(input.http_method.clone()),
            endpoint: Set(input.endpoint.clone()),
            severity: Set(input.severity.unwrap_or(1)),
            description: Set(input.description.clone()),
            created_at: Set(time::OffsetDateTime::now_utc()),
            updated_at: Set(time::OffsetDateTime::now_utc()),
        };

        let result = model.insert(db).await?;
        Ok(result.into())
    }

    /// Find API by ID with ownership verification
    pub async fn find_by_id_and_user(
        db: &DatabaseConnection,
        id: Uuid,
        user_id: Uuid,
    ) -> AppResult<Api> {
        let model = ApiEntity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Api".to_string()))?;

        // Verify collection ownership
        Self::verify_collection_ownership(db, model.collection_id, user_id).await?;

        Ok(model.into())
    }

    /// List APIs for a specific collection (with ownership check)
    pub async fn list_by_collection(
        db: &DatabaseConnection,
        collection_id: Uuid,
        user_id: Uuid,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<Api>> {
        // Verify collection ownership
        Self::verify_collection_ownership(db, collection_id, user_id).await?;

        let models = ApiEntity::find()
            .filter(Column::CollectionId.eq(collection_id))
            .order_by_desc(Column::CreatedAt)
            .paginate(db, limit)
            .fetch_page(offset / limit)
            .await?;

        Ok(models.into_iter().map(|m| m.into()).collect())
    }

    /// Count APIs for a specific collection
    pub async fn count_by_collection(
        db: &DatabaseConnection,
        collection_id: Uuid,
        user_id: Uuid,
    ) -> AppResult<u64> {
        // Verify collection ownership
        Self::verify_collection_ownership(db, collection_id, user_id).await?;

        let count = ApiEntity::find()
            .filter(Column::CollectionId.eq(collection_id))
            .count(db)
            .await?;

        Ok(count)
    }

    /// Update API (with ownership check)
    pub async fn update(
        db: &DatabaseConnection,
        id: Uuid,
        user_id: Uuid,
        input: &UpdateApi,
    ) -> AppResult<Api> {
        let model = ApiEntity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Api".to_string()))?;

        // Verify collection ownership
        Self::verify_collection_ownership(db, model.collection_id, user_id).await?;

        let mut active: ActiveModel = model.into();

        if let Some(name) = &input.name {
            active.name = Set(name.clone());
        }
        if let Some(http_method) = &input.http_method {
            active.http_method = Set(http_method.clone());
        }
        if let Some(endpoint) = &input.endpoint {
            active.endpoint = Set(endpoint.clone());
        }
        if let Some(severity) = input.severity {
            active.severity = Set(severity);
        }
        if let Some(description) = &input.description {
            active.description = Set(Some(description.clone()));
        }
        active.updated_at = Set(time::OffsetDateTime::now_utc());

        let result = active.update(db).await?;
        Ok(result.into())
    }

    /// Delete API (with ownership check)
    pub async fn delete_by_user(db: &DatabaseConnection, id: Uuid, user_id: Uuid) -> AppResult<()> {
        let model = ApiEntity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Api".to_string()))?;

        // Verify collection ownership
        Self::verify_collection_ownership(db, model.collection_id, user_id).await?;

        let active: ActiveModel = model.into();
        active.delete(db).await?;

        Ok(())
    }

    /// Verify that the user owns the collection (through project ownership)
    async fn verify_collection_ownership(
        db: &DatabaseConnection,
        collection_id: Uuid,
        user_id: Uuid,
    ) -> AppResult<()> {
        // First find the collection to get its project_id
        let collection = CollectionEntity::find_by_id(collection_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Collection".to_string()))?;

        // Then verify the project is owned by the user
        ProjectEntity::find_by_id(collection.project_id)
            .filter(ProjectColumn::UserId.eq(user_id))
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Project".to_string()))?;

        Ok(())
    }
}

// Conversion from SeaORM model to our domain model
impl From<api::Model> for Api {
    fn from(m: api::Model) -> Self {
        Self {
            id: m.id,
            collection_id: m.collection_id,
            name: m.name,
            http_method: m.http_method,
            endpoint: m.endpoint,
            severity: m.severity,
            description: m.description,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}
