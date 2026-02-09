use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "responses")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub report_id: Uuid,
    pub api_id: Uuid,
    pub scenario_id: Uuid,
    pub example_index: i32,
    #[sea_orm(column_type = "Json", nullable)]
    pub response_data: Option<Json>,
    pub response_status: i16,
    pub pass: bool,
    pub error_message: Option<String>,
    pub request_time: TimeDateTimeWithTimeZone,
    pub request_duration_ms: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::report::Entity",
        from = "Column::ReportId",
        to = "super::report::Column::Id"
    )]
    Report,
    #[sea_orm(
        belongs_to = "super::api::Entity",
        from = "Column::ApiId",
        to = "super::api::Column::Id"
    )]
    Api,
    #[sea_orm(
        belongs_to = "super::scenario::Entity",
        from = "Column::ScenarioId",
        to = "super::scenario::Column::Id"
    )]
    Scenario,
}

impl Related<super::report::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Report.def()
    }
}

impl Related<super::api::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Api.def()
    }
}

impl Related<super::scenario::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Scenario.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
