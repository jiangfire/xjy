use crate::{
    error::{AppError, AppResult},
    models::{bookmark, post, Bookmark, Post, PostModel},
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Statement,
};
use std::collections::HashMap;

pub struct BookmarkService {
    db: DatabaseConnection,
}

impl BookmarkService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn add_bookmark(&self, user_id: i32, post_id: i32) -> AppResult<bool> {
        Post::find_by_id(post_id)
            .one(&self.db)
            .await?
            .ok_or(AppError::NotFound)?;

        self.db
            .execute(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "INSERT INTO bookmarks (user_id, post_id, created_at)
                 VALUES ($1, $2, NOW())
                 ON CONFLICT (user_id, post_id) DO NOTHING",
                vec![user_id.into(), post_id.into()],
            ))
            .await?;
        Ok(true)
    }

    pub async fn remove_bookmark(&self, user_id: i32, post_id: i32) -> AppResult<bool> {
        Bookmark::delete_many()
            .filter(bookmark::Column::UserId.eq(user_id))
            .filter(bookmark::Column::PostId.eq(post_id))
            .exec(&self.db)
            .await?;
        Ok(false)
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

        if existing.is_some() {
            self.remove_bookmark(user_id, post_id).await
        } else {
            self.add_bookmark(user_id, post_id).await
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
