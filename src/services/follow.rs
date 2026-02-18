use crate::{
    error::{AppError, AppResult},
    models::{follow, user, Follow, User, UserModel},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder,
};
use std::collections::HashMap;

pub struct FollowService {
    db: DatabaseConnection,
}

impl FollowService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Toggle follow: if exists -> unfollow, if not -> follow.
    /// Returns true if now following, false if unfollowed.
    pub async fn toggle(&self, follower_id: i32, following_id: i32) -> AppResult<bool> {
        if follower_id == following_id {
            return Err(AppError::Validation("Cannot follow yourself".to_string()));
        }

        // Verify target user exists
        User::find_by_id(following_id)
            .one(&self.db)
            .await?
            .ok_or(AppError::NotFound)?;

        let existing = Follow::find()
            .filter(follow::Column::FollowerId.eq(follower_id))
            .filter(follow::Column::FollowingId.eq(following_id))
            .one(&self.db)
            .await?;

        if let Some(existing) = existing {
            Follow::delete_by_id(existing.id)
                .exec(&self.db)
                .await?;
            Ok(false)
        } else {
            let now = chrono::Utc::now().naive_utc();
            let model = follow::ActiveModel {
                follower_id: sea_orm::ActiveValue::Set(follower_id),
                following_id: sea_orm::ActiveValue::Set(following_id),
                created_at: sea_orm::ActiveValue::Set(now),
                ..Default::default()
            };
            model.insert(&self.db).await?;
            Ok(true)
        }
    }

    /// List users who follow the given user (followers of user_id).
    pub async fn list_followers(
        &self,
        user_id: i32,
        page: u64,
        per_page: u64,
    ) -> AppResult<(Vec<UserModel>, u64)> {
        let paginator = Follow::find()
            .filter(follow::Column::FollowingId.eq(user_id))
            .order_by_desc(follow::Column::CreatedAt)
            .paginate(&self.db, per_page);

        let total = paginator.num_items().await?;
        let follows = paginator.fetch_page(page.saturating_sub(1)).await?;

        let user_ids: Vec<i32> = follows.iter().map(|f| f.follower_id).collect();
        if user_ids.is_empty() {
            return Ok((vec![], total));
        }

        let users = User::find()
            .filter(user::Column::Id.is_in(user_ids.clone()))
            .all(&self.db)
            .await?;

        // Reorder to match follow order
        let user_map: HashMap<i32, UserModel> = users.into_iter().map(|u| (u.id, u)).collect();
        let ordered: Vec<UserModel> = user_ids
            .into_iter()
            .filter_map(|id| user_map.get(&id).cloned())
            .collect();

        Ok((ordered, total))
    }

    /// List users that the given user follows (following of user_id).
    pub async fn list_following(
        &self,
        user_id: i32,
        page: u64,
        per_page: u64,
    ) -> AppResult<(Vec<UserModel>, u64)> {
        let paginator = Follow::find()
            .filter(follow::Column::FollowerId.eq(user_id))
            .order_by_desc(follow::Column::CreatedAt)
            .paginate(&self.db, per_page);

        let total = paginator.num_items().await?;
        let follows = paginator.fetch_page(page.saturating_sub(1)).await?;

        let user_ids: Vec<i32> = follows.iter().map(|f| f.following_id).collect();
        if user_ids.is_empty() {
            return Ok((vec![], total));
        }

        let users = User::find()
            .filter(user::Column::Id.is_in(user_ids.clone()))
            .all(&self.db)
            .await?;

        let user_map: HashMap<i32, UserModel> = users.into_iter().map(|u| (u.id, u)).collect();
        let ordered: Vec<UserModel> = user_ids
            .into_iter()
            .filter_map(|id| user_map.get(&id).cloned())
            .collect();

        Ok((ordered, total))
    }
}
