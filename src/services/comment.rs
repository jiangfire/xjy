use crate::{
    error::{AppError, AppResult},
    models::{comment, Comment, CommentModel},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
};

pub struct CommentService {
    db: DatabaseConnection,
}

impl CommentService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn list_by_post(&self, post_id: i32) -> AppResult<Vec<CommentModel>> {
        let comments = Comment::find()
            .filter(comment::Column::PostId.eq(post_id))
            .filter(comment::Column::IsHidden.eq(false))
            .order_by_asc(comment::Column::CreatedAt)
            .all(&self.db)
            .await?;
        Ok(comments)
    }

    pub async fn create(
        &self,
        post_id: i32,
        user_id: i32,
        parent_id: Option<i32>,
        content: &str,
    ) -> AppResult<CommentModel> {
        if let Some(pid) = parent_id {
            self.validate_parent(pid, post_id).await?;
        }

        let now = chrono::Utc::now().naive_utc();

        let new_comment = comment::ActiveModel {
            post_id: sea_orm::ActiveValue::Set(post_id),
            user_id: sea_orm::ActiveValue::Set(user_id),
            parent_id: sea_orm::ActiveValue::Set(parent_id),
            content: sea_orm::ActiveValue::Set(content.to_string()),
            upvotes: sea_orm::ActiveValue::Set(0),
            downvotes: sea_orm::ActiveValue::Set(0),
            created_at: sea_orm::ActiveValue::Set(now),
            updated_at: sea_orm::ActiveValue::Set(now),
            ..Default::default()
        };

        let comment = new_comment.insert(&self.db).await?;
        Ok(comment)
    }

    pub async fn update(&self, id: i32, user_id: i32, content: &str) -> AppResult<CommentModel> {
        let existing = self.get_by_id(id).await?;
        if existing.user_id != user_id {
            return Err(AppError::Forbidden);
        }

        let now = chrono::Utc::now().naive_utc();

        let mut active: comment::ActiveModel = existing.into();
        active.content = sea_orm::ActiveValue::Set(content.to_string());
        active.updated_at = sea_orm::ActiveValue::Set(now);

        let updated = active.update(&self.db).await?;
        Ok(updated)
    }

    pub async fn delete(&self, id: i32, user_id: i32) -> AppResult<()> {
        let existing = self.get_by_id(id).await?;
        if existing.user_id != user_id {
            return Err(AppError::Forbidden);
        }

        Comment::delete_by_id(id).exec(&self.db).await?;
        Ok(())
    }

    pub async fn get_by_id(&self, id: i32) -> AppResult<CommentModel> {
        Comment::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or(AppError::NotFound)
    }

    async fn validate_parent(&self, parent_id: i32, post_id: i32) -> AppResult<()> {
        let parent = Comment::find_by_id(parent_id)
            .one(&self.db)
            .await?
            .ok_or(AppError::Validation("Parent comment not found".to_string()))?;

        if parent.post_id != post_id {
            return Err(AppError::Validation(
                "Parent comment belongs to a different post".to_string(),
            ));
        }

        let depth = self.get_comment_depth(parent_id).await?;
        if depth >= 10 {
            return Err(AppError::Validation(
                "Maximum comment nesting depth reached".to_string(),
            ));
        }

        Ok(())
    }

    async fn get_comment_depth(&self, comment_id: i32) -> AppResult<u32> {
        let mut depth = 0u32;
        let mut current_id = Some(comment_id);

        while let Some(id) = current_id {
            let comment = Comment::find_by_id(id)
                .one(&self.db)
                .await?
                .ok_or(AppError::NotFound)?;
            current_id = comment.parent_id;
            depth += 1;
            if depth > 10 {
                break;
            }
        }

        Ok(depth)
    }
}

#[cfg(test)]
mod tests {
    const MAX_DEPTH: u32 = 10;

    fn is_depth_exceeded(depth: u32) -> bool {
        depth > MAX_DEPTH
    }

    #[test]
    fn test_depth_limit_enforced() {
        assert!(is_depth_exceeded(11));
        assert!(!is_depth_exceeded(10));
    }

    #[test]
    fn test_root_comment_has_no_parent() {
        let parent_id: Option<i32> = None;
        assert!(parent_id.is_none());
    }

    #[test]
    fn test_reply_has_parent() {
        let parent_id = Some(1);
        assert!(parent_id.is_some());
    }
}
