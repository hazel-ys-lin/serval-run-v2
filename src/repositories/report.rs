use async_trait::async_trait;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use uuid::Uuid;

use crate::entity::project::{Column as ProjectColumn, Entity as ProjectEntity};
use crate::entity::report::{self, ActiveModel, Column, Entity as ReportEntity};
use crate::error::{AppError, AppResult};
use crate::models::{CreateReport, Report};
use crate::repositories::Repository;

/// Report repository for database operations
pub struct ReportRepository;

#[async_trait]
impl Repository<Report> for ReportRepository {
    async fn find_by_id(db: &DatabaseConnection, id: Uuid) -> AppResult<Report> {
        let model = ReportEntity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Report".to_string()))?;

        Ok(model.into())
    }

    async fn delete(db: &DatabaseConnection, id: Uuid) -> AppResult<()> {
        let result = ReportEntity::delete_by_id(id).exec(db).await?;

        if result.rows_affected == 0 {
            return Err(AppError::NotFound("Report".to_string()));
        }

        Ok(())
    }

    async fn list(db: &DatabaseConnection, limit: u64, offset: u64) -> AppResult<Vec<Report>> {
        let models = ReportEntity::find()
            .order_by_desc(Column::CreatedAt)
            .paginate(db, limit)
            .fetch_page(offset / limit)
            .await?;

        Ok(models.into_iter().map(|m| m.into()).collect())
    }

    async fn count(db: &DatabaseConnection) -> AppResult<u64> {
        let count = ReportEntity::find().count(db).await?;
        Ok(count)
    }
}

impl ReportRepository {
    /// Create a new report
    pub async fn create(
        db: &DatabaseConnection,
        project_id: Uuid,
        user_id: Uuid,
        input: &CreateReport,
    ) -> AppResult<Report> {
        // Verify project ownership
        Self::verify_project_ownership(db, project_id, user_id).await?;

        let model = ActiveModel {
            id: Set(Uuid::new_v4()),
            project_id: Set(project_id),
            environment_id: Set(input.environment_id),
            collection_id: Set(input.collection_id),
            report_level: Set(input.report_level),
            report_type: Set(input.report_type.clone()),
            finished: Set(false),
            calculated: Set(false),
            pass_rate: Set(None),
            response_count: Set(0),
            created_at: Set(time::OffsetDateTime::now_utc()),
            finished_at: Set(None),
        };

        let result = model.insert(db).await?;
        Ok(result.into())
    }

    /// Find report by ID with project ownership verification
    pub async fn find_by_id_and_user(
        db: &DatabaseConnection,
        id: Uuid,
        user_id: Uuid,
    ) -> AppResult<Report> {
        let model = ReportEntity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Report".to_string()))?;

        // Verify project ownership
        Self::verify_project_ownership(db, model.project_id, user_id).await?;

        Ok(model.into())
    }

    /// List reports for a specific project (with ownership check)
    pub async fn list_by_project(
        db: &DatabaseConnection,
        project_id: Uuid,
        user_id: Uuid,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<Report>> {
        // Verify project ownership
        Self::verify_project_ownership(db, project_id, user_id).await?;

        let models = ReportEntity::find()
            .filter(Column::ProjectId.eq(project_id))
            .order_by_desc(Column::CreatedAt)
            .paginate(db, limit)
            .fetch_page(offset / limit)
            .await?;

        Ok(models.into_iter().map(|m| m.into()).collect())
    }

    /// Count reports for a specific project
    pub async fn count_by_project(
        db: &DatabaseConnection,
        project_id: Uuid,
        user_id: Uuid,
    ) -> AppResult<u64> {
        // Verify project ownership
        Self::verify_project_ownership(db, project_id, user_id).await?;

        let count = ReportEntity::find()
            .filter(Column::ProjectId.eq(project_id))
            .count(db)
            .await?;

        Ok(count)
    }

    /// Mark report as finished and calculate pass rate
    pub async fn finish_report(
        db: &DatabaseConnection,
        id: Uuid,
        user_id: Uuid,
        pass_rate: Decimal,
        response_count: i32,
    ) -> AppResult<Report> {
        let model = ReportEntity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Report".to_string()))?;

        // Verify project ownership
        Self::verify_project_ownership(db, model.project_id, user_id).await?;

        let mut active: ActiveModel = model.into();
        active.finished = Set(true);
        active.calculated = Set(true);
        active.pass_rate = Set(Some(pass_rate));
        active.response_count = Set(response_count);
        active.finished_at = Set(Some(time::OffsetDateTime::now_utc()));

        let result = active.update(db).await?;
        Ok(result.into())
    }

    /// Delete report (with ownership check)
    pub async fn delete_by_user(db: &DatabaseConnection, id: Uuid, user_id: Uuid) -> AppResult<()> {
        let model = ReportEntity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Report".to_string()))?;

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
impl From<report::Model> for Report {
    fn from(m: report::Model) -> Self {
        Self {
            id: m.id,
            project_id: m.project_id,
            environment_id: m.environment_id,
            collection_id: m.collection_id,
            report_level: m.report_level,
            report_type: m.report_type,
            finished: m.finished,
            calculated: m.calculated,
            pass_rate: m.pass_rate,
            response_count: m.response_count,
            created_at: m.created_at,
            finished_at: m.finished_at,
        }
    }
}
