use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "scenarios")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub api_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    #[sea_orm(column_type = "Json")]
    pub steps: Json,
    #[sea_orm(column_type = "Json")]
    pub examples: Json,
    pub created_at: TimeDateTimeWithTimeZone,
    pub updated_at: TimeDateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::api::Entity",
        from = "Column::ApiId",
        to = "super::api::Column::Id"
    )]
    Api,
    #[sea_orm(has_many = "super::response::Entity")]
    Responses,
}

impl Related<super::api::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Api.def()
    }
}

impl Related<super::response::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Responses.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
