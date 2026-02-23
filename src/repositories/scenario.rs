use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use uuid::Uuid;

use crate::entity::scenario::{self, ActiveModel, Column, Entity as ScenarioEntity};
use crate::error::{AppError, AppResult};
use crate::models::{CreateScenario, Scenario, UpdateScenario};
use crate::repositories::ownership::OwnershipVerifier;
use crate::repositories::Repository;

/// Scenario repository for database operations
pub struct ScenarioRepository;

#[async_trait]
impl Repository<Scenario> for ScenarioRepository {
    async fn find_by_id(db: &DatabaseConnection, id: Uuid) -> AppResult<Scenario> {
        let model = ScenarioEntity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Scenario".to_string()))?;

        Ok(model.into())
    }

    async fn delete(db: &DatabaseConnection, id: Uuid) -> AppResult<()> {
        let result = ScenarioEntity::delete_by_id(id).exec(db).await?;

        if result.rows_affected == 0 {
            return Err(AppError::NotFound("Scenario".to_string()));
        }

        Ok(())
    }

    async fn list(db: &DatabaseConnection, limit: u64, offset: u64) -> AppResult<Vec<Scenario>> {
        let models = ScenarioEntity::find()
            .order_by_desc(Column::CreatedAt)
            .offset(offset)
            .limit(limit)
            .all(db)
            .await?;

        Ok(models.into_iter().map(|m| m.into()).collect())
    }

    async fn count(db: &DatabaseConnection) -> AppResult<u64> {
        let count = ScenarioEntity::find().count(db).await?;
        Ok(count)
    }
}

impl ScenarioRepository {
    /// Create a new scenario
    pub async fn create(
        db: &DatabaseConnection,
        api_id: Uuid,
        user_id: Uuid,
        input: &CreateScenario,
    ) -> AppResult<Scenario> {
        OwnershipVerifier::verify_api(db, api_id, user_id).await?;

        let steps_json = serde_json::to_value(&input.steps)
            .map_err(|e| AppError::Validation(format!("Invalid steps JSON: {}", e)))?;
        let examples_json = serde_json::to_value(&input.examples)
            .map_err(|e| AppError::Validation(format!("Invalid examples JSON: {}", e)))?;

        let model = ActiveModel {
            id: Set(Uuid::new_v4()),
            api_id: Set(api_id),
            title: Set(input.title.clone()),
            description: Set(input.description.clone()),
            tags: Set(input.tags.clone().unwrap_or_default()),
            steps: Set(steps_json),
            examples: Set(examples_json),
            created_at: Set(time::OffsetDateTime::now_utc()),
            updated_at: Set(time::OffsetDateTime::now_utc()),
        };

        let result = model.insert(db).await?;
        Ok(result.into())
    }

    /// Find scenario by ID with ownership verification
    pub async fn find_by_id_and_user(
        db: &DatabaseConnection,
        id: Uuid,
        user_id: Uuid,
    ) -> AppResult<Scenario> {
        let model = ScenarioEntity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Scenario".to_string()))?;

        OwnershipVerifier::verify_api(db, model.api_id, user_id).await?;

        Ok(model.into())
    }

    /// List scenarios for a specific API (with ownership check)
    pub async fn list_by_api(
        db: &DatabaseConnection,
        api_id: Uuid,
        user_id: Uuid,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<Scenario>> {
        OwnershipVerifier::verify_api(db, api_id, user_id).await?;

        let models = ScenarioEntity::find()
            .filter(Column::ApiId.eq(api_id))
            .order_by_desc(Column::CreatedAt)
            .offset(offset)
            .limit(limit)
            .all(db)
            .await?;

        Ok(models.into_iter().map(|m| m.into()).collect())
    }

    /// Count scenarios for a specific API
    pub async fn count_by_api(
        db: &DatabaseConnection,
        api_id: Uuid,
        user_id: Uuid,
    ) -> AppResult<u64> {
        OwnershipVerifier::verify_api(db, api_id, user_id).await?;

        let count = ScenarioEntity::find()
            .filter(Column::ApiId.eq(api_id))
            .count(db)
            .await?;

        Ok(count)
    }

    /// Update scenario (with ownership check)
    pub async fn update(
        db: &DatabaseConnection,
        id: Uuid,
        user_id: Uuid,
        input: &UpdateScenario,
    ) -> AppResult<Scenario> {
        let model = ScenarioEntity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Scenario".to_string()))?;

        OwnershipVerifier::verify_api(db, model.api_id, user_id).await?;

        let mut active: ActiveModel = model.into();

        if let Some(title) = &input.title {
            active.title = Set(title.clone());
        }
        if let Some(description) = &input.description {
            active.description = Set(Some(description.clone()));
        }
        if let Some(tags) = &input.tags {
            active.tags = Set(tags.clone());
        }
        if let Some(steps) = &input.steps {
            let steps_json = serde_json::to_value(steps)
                .map_err(|e| AppError::Validation(format!("Invalid steps JSON: {}", e)))?;
            active.steps = Set(steps_json);
        }
        if let Some(examples) = &input.examples {
            let examples_json = serde_json::to_value(examples)
                .map_err(|e| AppError::Validation(format!("Invalid examples JSON: {}", e)))?;
            active.examples = Set(examples_json);
        }
        active.updated_at = Set(time::OffsetDateTime::now_utc());

        let result = active.update(db).await?;
        Ok(result.into())
    }

    /// Delete scenario (with ownership check)
    pub async fn delete_by_user(db: &DatabaseConnection, id: Uuid, user_id: Uuid) -> AppResult<()> {
        let model = ScenarioEntity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Scenario".to_string()))?;

        OwnershipVerifier::verify_api(db, model.api_id, user_id).await?;

        let active: ActiveModel = model.into();
        active.delete(db).await?;

        Ok(())
    }
}

// Conversion from SeaORM model to our domain model
impl From<scenario::Model> for Scenario {
    fn from(m: scenario::Model) -> Self {
        Self {
            id: m.id,
            api_id: m.api_id,
            title: m.title,
            description: m.description,
            tags: m.tags,
            steps: m.steps,
            examples: m.examples,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}
