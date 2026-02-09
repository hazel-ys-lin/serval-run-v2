use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use uuid::Uuid;

use crate::entity::project::{Column as ProjectColumn, Entity as ProjectEntity};
use crate::entity::report::Entity as ReportEntity;
use crate::entity::response::{self, ActiveModel, Column, Entity as ResponseEntity};
use crate::error::{AppError, AppResult};
use crate::models::{CreateResponse, Response};
use crate::repositories::Repository;

/// Response repository for database operations
pub struct ResponseRepository;

#[async_trait]
impl Repository<Response> for ResponseRepository {
    async fn find_by_id(db: &DatabaseConnection, id: Uuid) -> AppResult<Response> {
        let model = ResponseEntity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Response".to_string()))?;

        Ok(model.into())
    }

    async fn delete(db: &DatabaseConnection, id: Uuid) -> AppResult<()> {
        let result = ResponseEntity::delete_by_id(id).exec(db).await?;

        if result.rows_affected == 0 {
            return Err(AppError::NotFound("Response".to_string()));
        }

        Ok(())
    }

    async fn list(db: &DatabaseConnection, limit: u64, offset: u64) -> AppResult<Vec<Response>> {
        let models = ResponseEntity::find()
            .order_by_desc(Column::RequestTime)
            .paginate(db, limit)
            .fetch_page(offset / limit)
            .await?;

        Ok(models.into_iter().map(|m| m.into()).collect())
    }

    async fn count(db: &DatabaseConnection) -> AppResult<u64> {
        let count = ResponseEntity::find().count(db).await?;
        Ok(count)
    }
}

impl ResponseRepository {
    /// Create a new response
    pub async fn create(
        db: &DatabaseConnection,
        report_id: Uuid,
        input: &CreateResponse,
    ) -> AppResult<Response> {
        let model = ActiveModel {
            id: Set(Uuid::new_v4()),
            report_id: Set(report_id),
            api_id: Set(input.api_id),
            scenario_id: Set(input.scenario_id),
            example_index: Set(input.example_index),
            response_data: Set(input.response_data.clone()),
            response_status: Set(input.response_status),
            pass: Set(input.pass),
            error_message: Set(input.error_message.clone()),
            request_time: Set(time::OffsetDateTime::now_utc()),
            request_duration_ms: Set(input.request_duration_ms),
        };

        let result = model.insert(db).await?;
        Ok(result.into())
    }

    /// Create multiple responses in batch
    pub async fn create_batch(
        db: &DatabaseConnection,
        report_id: Uuid,
        inputs: &[CreateResponse],
    ) -> AppResult<Vec<Response>> {
        let mut results = Vec::with_capacity(inputs.len());

        for input in inputs {
            let response = Self::create(db, report_id, input).await?;
            results.push(response);
        }

        Ok(results)
    }

    /// Find response by ID with ownership verification
    pub async fn find_by_id_and_user(
        db: &DatabaseConnection,
        id: Uuid,
        user_id: Uuid,
    ) -> AppResult<Response> {
        let model = ResponseEntity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Response".to_string()))?;

        // Verify ownership through report -> project chain
        Self::verify_response_ownership(db, model.report_id, user_id).await?;

        Ok(model.into())
    }

    /// List responses for a specific report (with ownership check)
    pub async fn list_by_report(
        db: &DatabaseConnection,
        report_id: Uuid,
        user_id: Uuid,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<Response>> {
        // Verify ownership
        Self::verify_response_ownership(db, report_id, user_id).await?;

        let models = ResponseEntity::find()
            .filter(Column::ReportId.eq(report_id))
            .order_by_asc(Column::RequestTime)
            .paginate(db, limit)
            .fetch_page(offset / limit)
            .await?;

        Ok(models.into_iter().map(|m| m.into()).collect())
    }

    /// Count responses for a specific report
    pub async fn count_by_report(
        db: &DatabaseConnection,
        report_id: Uuid,
        user_id: Uuid,
    ) -> AppResult<u64> {
        // Verify ownership
        Self::verify_response_ownership(db, report_id, user_id).await?;

        let count = ResponseEntity::find()
            .filter(Column::ReportId.eq(report_id))
            .count(db)
            .await?;

        Ok(count)
    }

    /// Get summary statistics for a report
    pub async fn get_report_stats(
        db: &DatabaseConnection,
        report_id: Uuid,
        user_id: Uuid,
    ) -> AppResult<(i64, i64, i64)> {
        // Verify ownership
        Self::verify_response_ownership(db, report_id, user_id).await?;

        let all_responses = ResponseEntity::find()
            .filter(Column::ReportId.eq(report_id))
            .all(db)
            .await?;

        let total = all_responses.len() as i64;
        let passed = all_responses.iter().filter(|r| r.pass).count() as i64;
        let failed = total - passed;

        Ok((total, passed, failed))
    }

    /// Delete all responses for a report
    pub async fn delete_by_report(
        db: &DatabaseConnection,
        report_id: Uuid,
        user_id: Uuid,
    ) -> AppResult<u64> {
        // Verify ownership
        Self::verify_response_ownership(db, report_id, user_id).await?;

        let result = ResponseEntity::delete_many()
            .filter(Column::ReportId.eq(report_id))
            .exec(db)
            .await?;

        Ok(result.rows_affected)
    }

    /// Verify ownership through report -> project -> user chain
    async fn verify_response_ownership(
        db: &DatabaseConnection,
        report_id: Uuid,
        user_id: Uuid,
    ) -> AppResult<()> {
        // Get the report
        let report = ReportEntity::find_by_id(report_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Report".to_string()))?;

        // Verify project ownership
        ProjectEntity::find_by_id(report.project_id)
            .filter(ProjectColumn::UserId.eq(user_id))
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Project".to_string()))?;

        Ok(())
    }
}

// Conversion from SeaORM model to our domain model
impl From<response::Model> for Response {
    fn from(m: response::Model) -> Self {
        Self {
            id: m.id,
            report_id: m.report_id,
            api_id: m.api_id,
            scenario_id: m.scenario_id,
            example_index: m.example_index,
            response_data: m.response_data,
            response_status: m.response_status,
            pass: m.pass,
            error_message: m.error_message,
            request_time: m.request_time,
            request_duration_ms: m.request_duration_ms,
        }
    }
}
