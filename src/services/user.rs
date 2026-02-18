use crate::{
    error::{AppError, AppResult},
    models::{user, User, UserModel},
};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

pub struct UserService {
    db: DatabaseConnection,
}

impl UserService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn get_by_username(&self, username: &str) -> AppResult<UserModel> {
        User::find()
            .filter(user::Column::Username.eq(username))
            .one(&self.db)
            .await?
            .ok_or(AppError::NotFound)
    }

    pub async fn update_profile(
        &self,
        user_id: i32,
        bio: Option<String>,
        avatar_url: Option<String>,
    ) -> AppResult<UserModel> {
        let existing = User::find_by_id(user_id)
            .one(&self.db)
            .await?
            .ok_or(AppError::NotFound)?;

        let now = chrono::Utc::now().naive_utc();

        let mut active: user::ActiveModel = existing.into();
        active.bio = sea_orm::ActiveValue::Set(bio);
        active.avatar_url = sea_orm::ActiveValue::Set(avatar_url);
        active.updated_at = sea_orm::ActiveValue::Set(now);

        let updated = active.update(&self.db).await?;
        Ok(updated)
    }

    /// Update only the avatar URL (used by upload handler).
    pub async fn update_avatar_url(&self, user_id: i32, url: &str) -> AppResult<UserModel> {
        let existing = User::find_by_id(user_id)
            .one(&self.db)
            .await?
            .ok_or(AppError::NotFound)?;

        let now = chrono::Utc::now().naive_utc();

        let mut active: user::ActiveModel = existing.into();
        active.avatar_url = sea_orm::ActiveValue::Set(Some(url.to_string()));
        active.updated_at = sea_orm::ActiveValue::Set(now);

        let updated = active.update(&self.db).await?;
        Ok(updated)
    }
}
