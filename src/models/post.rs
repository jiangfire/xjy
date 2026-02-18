use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, ToSchema)]
#[sea_orm(table_name = "posts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32,
    pub forum_id: i32,
    pub title: String,
    #[sea_orm(column_type = "Text")]
    pub content: String,
    pub upvotes: i32,
    pub downvotes: i32,
    pub view_count: i32,
    pub is_pinned: bool,
    pub is_locked: bool,
    pub is_hidden: bool,
    pub created_at: DateTime,
    pub updated_at: DateTime,
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
        belongs_to = "super::forum::Entity",
        from = "Column::ForumId",
        to = "super::forum::Column::Id"
    )]
    Forum,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::forum::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Forum.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
