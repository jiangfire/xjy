use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "notifications")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32,
    #[sea_orm(column_type = "String(StringLen::N(50))")]
    pub kind: String,
    pub actor_id: i32,
    #[sea_orm(column_type = "String(StringLen::N(20))")]
    pub target_type: String,
    pub target_id: i32,
    #[sea_orm(column_type = "Text")]
    pub message: String,
    pub is_read: bool,
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::ActorId",
        to = "super::user::Column::Id"
    )]
    Actor,
}

impl ActiveModelBehavior for ActiveModel {}
