use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, RelationTrait};
use uuid::Uuid;

use crate::entity::collection::Entity as CollectionEntity;
use crate::entity::project::{Column as ProjectColumn, Entity as ProjectEntity};
use crate::error::{AppError, AppResult};

/// Shared ownership verification helpers.
///
/// These functions verify that a given resource ultimately belongs to the
/// specified user by traversing the entity hierarchy with efficient queries.
pub struct OwnershipVerifier;

impl OwnershipVerifier {
    /// Verify that a project belongs to the user. (1 query)
    pub async fn verify_project(
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

    /// Verify that a collection belongs to the user via its project. (1 JOIN query)
    pub async fn verify_collection(
        db: &DatabaseConnection,
        collection_id: Uuid,
        user_id: Uuid,
    ) -> AppResult<()> {
        use sea_orm::QuerySelect;

        CollectionEntity::find_by_id(collection_id)
            .inner_join(ProjectEntity)
            .filter(ProjectColumn::UserId.eq(user_id))
            .select_only()
            .column(crate::entity::collection::Column::Id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Collection".to_string()))?;

        Ok(())
    }

    /// Verify that an API belongs to the user via collection → project. (1 JOIN query)
    pub async fn verify_api(db: &DatabaseConnection, api_id: Uuid, user_id: Uuid) -> AppResult<()> {
        use crate::entity::api::Entity as ApiEntity;
        use sea_orm::QuerySelect;

        ApiEntity::find_by_id(api_id)
            .inner_join(CollectionEntity)
            .join(
                sea_orm::JoinType::InnerJoin,
                crate::entity::collection::Relation::Project.def(),
            )
            .filter(ProjectColumn::UserId.eq(user_id))
            .select_only()
            .column(crate::entity::api::Column::Id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Api".to_string()))?;

        Ok(())
    }

    /// Verify that a report belongs to the user via its project. (1 JOIN query)
    pub async fn verify_report(
        db: &DatabaseConnection,
        report_id: Uuid,
        user_id: Uuid,
    ) -> AppResult<()> {
        use crate::entity::report::Entity as ReportEntity;
        use sea_orm::QuerySelect;

        ReportEntity::find_by_id(report_id)
            .inner_join(ProjectEntity)
            .filter(ProjectColumn::UserId.eq(user_id))
            .select_only()
            .column(crate::entity::report::Column::Id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Report".to_string()))?;

        Ok(())
    }
}
