use crate::{
    error::{AppError, AppResult},
    models::{bookmark, post, Bookmark, Post, PostModel},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder,
};
use std::collections::HashMap;

pub struct BookmarkService {
    db: DatabaseConnection,
}

impl BookmarkService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Toggle bookmark: if exists -> delete, if not -> create.
    /// Returns true if bookmarked, false if un-bookmarked.
    pub async fn toggle(&self, user_id: i32, post_id: i32) -> AppResult<bool> {
        // Verify post exists
        Post::find_by_id(post_id)
            .one(&self.db)
            .await?
            .ok_or(AppError::NotFound)?;

        let existing = Bookmark::find()
            .filter(bookmark::Column::UserId.eq(user_id))
            .filter(bookmark::Column::PostId.eq(post_id))
            .one(&self.db)
            .await?;

        if let Some(existing) = existing {
            Bookmark::delete_by_id(existing.id).exec(&self.db).await?;
            Ok(false)
        } else {
            let now = chrono::Utc::now().naive_utc();
            let model = bookmark::ActiveModel {
                user_id: sea_orm::ActiveValue::Set(user_id),
                post_id: sea_orm::ActiveValue::Set(post_id),
                created_at: sea_orm::ActiveValue::Set(now),
                ..Default::default()
            };
            model.insert(&self.db).await?;
            Ok(true)
        }
    }

    /// List user's bookmarked posts with pagination.
    /// Returns posts in bookmark order (most recently bookmarked first).
    pub async fn list_user_bookmarks(
        &self,
        user_id: i32,
        page: u64,
        per_page: u64,
    ) -> AppResult<(Vec<PostModel>, u64)> {
        let paginator = Bookmark::find()
            .filter(bookmark::Column::UserId.eq(user_id))
            .order_by_desc(bookmark::Column::CreatedAt)
            .paginate(&self.db, per_page);

        let total = paginator.num_items().await?;
        let bookmarks = paginator.fetch_page(page.saturating_sub(1)).await?;

        let post_ids: Vec<i32> = bookmarks.iter().map(|b| b.post_id).collect();
        if post_ids.is_empty() {
            return Ok((vec![], total));
        }

        let posts = Post::find()
            .filter(post::Column::Id.is_in(post_ids.clone()))
            .all(&self.db)
            .await?;

        // Reorder posts to match bookmark order
        let post_map: HashMap<i32, PostModel> = posts.into_iter().map(|p| (p.id, p)).collect();
        let ordered: Vec<PostModel> = post_ids
            .into_iter()
            .filter_map(|id| post_map.get(&id).cloned())
            .collect();

        Ok((ordered, total))
    }
}
