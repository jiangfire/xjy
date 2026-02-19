use crate::{
    error::{AppError, AppResult},
    models::{post, user, Comment, Forum, Post, User, UserModel},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder,
};

pub struct AdminService {
    db: DatabaseConnection,
}

impl AdminService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn get_stats(&self) -> AppResult<AdminStats> {
        let total_users = User::find().count(&self.db).await?;
        let total_posts = Post::find().count(&self.db).await?;
        let total_comments = Comment::find().count(&self.db).await?;
        let total_forums = Forum::find().count(&self.db).await?;

        let today = chrono::Utc::now().naive_utc().date();
        let today_start = today.and_hms_opt(0, 0, 0).unwrap();

        let users_today = User::find()
            .filter(user::Column::CreatedAt.gte(today_start))
            .count(&self.db)
            .await?;

        let posts_today = Post::find()
            .filter(post::Column::CreatedAt.gte(today_start))
            .count(&self.db)
            .await?;

        Ok(AdminStats {
            total_users,
            total_posts,
            total_comments,
            total_forums,
            users_today,
            posts_today,
        })
    }

    pub async fn list_users(&self, page: u64, per_page: u64) -> AppResult<(Vec<UserModel>, u64)> {
        let paginator = User::find()
            .order_by_desc(user::Column::CreatedAt)
            .paginate(&self.db, per_page);

        let total = paginator.num_items().await?;
        let users = paginator.fetch_page(page.saturating_sub(1)).await?;
        Ok((users, total))
    }

    pub async fn update_user_role(&self, user_id: i32, role: &str) -> AppResult<UserModel> {
        let valid_roles = ["user", "admin", "moderator", "banned"];
        if !valid_roles.contains(&role) {
            return Err(AppError::Validation(format!(
                "Invalid role. Must be one of: {}",
                valid_roles.join(", ")
            )));
        }

        let existing = User::find_by_id(user_id)
            .one(&self.db)
            .await?
            .ok_or(AppError::NotFound)?;

        let mut active: user::ActiveModel = existing.into();
        active.role = sea_orm::ActiveValue::Set(role.to_string());
        let updated = active.update(&self.db).await?;
        Ok(updated)
    }

    pub async fn admin_delete_post(&self, post_id: i32) -> AppResult<()> {
        Post::find_by_id(post_id)
            .one(&self.db)
            .await?
            .ok_or(AppError::NotFound)?;

        Post::delete_by_id(post_id).exec(&self.db).await?;

        let points = crate::services::points::PointsService::new(self.db.clone());
        let _ = points.rollback_by_ref("post", post_id).await;
        Ok(())
    }

    pub async fn admin_delete_comment(&self, comment_id: i32) -> AppResult<()> {
        Comment::find_by_id(comment_id)
            .one(&self.db)
            .await?
            .ok_or(AppError::NotFound)?;

        Comment::delete_by_id(comment_id).exec(&self.db).await?;

        let points = crate::services::points::PointsService::new(self.db.clone());
        let _ = points.rollback_by_ref("comment", comment_id).await;
        Ok(())
    }
}

pub struct AdminStats {
    pub total_users: u64,
    pub total_posts: u64,
    pub total_comments: u64,
    pub total_forums: u64,
    pub users_today: u64,
    pub posts_today: u64,
}
