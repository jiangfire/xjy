use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, ToSchema)]
#[sea_orm(table_name = "reports")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub reporter_id: i32,
    #[sea_orm(column_type = "String(StringLen::N(20))")]
    pub target_type: String,
    pub target_id: i32,
    #[sea_orm(column_type = "String(StringLen::N(50))")]
    pub reason: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,
    #[sea_orm(column_type = "String(StringLen::N(20))")]
    pub status: String,
    pub resolved_by: Option<i32>,
    pub resolved_at: Option<DateTime>,
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::ReporterId",
        to = "super::user::Column::Id"
    )]
    Reporter,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::ResolvedBy",
        to = "super::user::Column::Id"
    )]
    Resolver,
}

impl ActiveModelBehavior for ActiveModel {}
