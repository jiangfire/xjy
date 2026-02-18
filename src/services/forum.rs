use crate::{
    error::{AppError, AppResult},
    models::{forum, Forum, ForumModel},
    services::cache::CacheService,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
};

const CACHE_KEY_FORUMS_LIST: &str = "forums:list";
const CACHE_TTL_FORUMS: u64 = 300; // 5 minutes

pub struct ForumService {
    db: DatabaseConnection,
    cache: Option<CacheService>,
}

impl ForumService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db, cache: None }
    }

    pub fn with_cache(mut self, cache: CacheService) -> Self {
        self.cache = Some(cache);
        self
    }

    pub async fn list(&self) -> AppResult<Vec<ForumModel>> {
        if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get::<Vec<ForumModel>>(CACHE_KEY_FORUMS_LIST).await {
                return Ok(cached);
            }
        }

        let forums = Forum::find()
            .order_by_asc(forum::Column::SortOrder)
            .all(&self.db)
            .await?;

        if let Some(cache) = &self.cache {
            cache
                .set(CACHE_KEY_FORUMS_LIST, &forums, CACHE_TTL_FORUMS)
                .await;
        }

        Ok(forums)
    }

    #[allow(dead_code)]
    pub async fn get_by_id(&self, id: i32) -> AppResult<ForumModel> {
        Forum::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or(AppError::NotFound)
    }

    pub async fn get_by_slug(&self, slug: &str) -> AppResult<ForumModel> {
        Forum::find()
            .filter(forum::Column::Slug.eq(slug))
            .one(&self.db)
            .await?
            .ok_or(AppError::NotFound)
    }

    pub async fn create(
        &self,
        name: &str,
        description: &str,
        slug: &str,
        sort_order: i32,
        icon_url: Option<String>,
    ) -> AppResult<ForumModel> {
        let now = chrono::Utc::now().naive_utc();

        let new_forum = forum::ActiveModel {
            name: sea_orm::ActiveValue::Set(name.to_string()),
            description: sea_orm::ActiveValue::Set(description.to_string()),
            slug: sea_orm::ActiveValue::Set(slug.to_string()),
            sort_order: sea_orm::ActiveValue::Set(sort_order),
            icon_url: sea_orm::ActiveValue::Set(icon_url),
            created_at: sea_orm::ActiveValue::Set(now),
            updated_at: sea_orm::ActiveValue::Set(now),
            ..Default::default()
        };

        let forum = new_forum.insert(&self.db).await?;
        self.invalidate_list_cache().await;
        Ok(forum)
    }

    pub async fn update(
        &self,
        slug: &str,
        name: &str,
        description: &str,
        sort_order: i32,
        icon_url: Option<String>,
    ) -> AppResult<ForumModel> {
        let existing = self.get_by_slug(slug).await?;
        let now = chrono::Utc::now().naive_utc();

        let mut active: forum::ActiveModel = existing.into();
        active.name = sea_orm::ActiveValue::Set(name.to_string());
        active.description = sea_orm::ActiveValue::Set(description.to_string());
        active.sort_order = sea_orm::ActiveValue::Set(sort_order);
        active.icon_url = sea_orm::ActiveValue::Set(icon_url);
        active.updated_at = sea_orm::ActiveValue::Set(now);

        let updated = active.update(&self.db).await?;
        self.invalidate_list_cache().await;
        Ok(updated)
    }

    pub async fn delete(&self, slug: &str) -> AppResult<()> {
        let existing = self.get_by_slug(slug).await?;
        Forum::delete_by_id(existing.id).exec(&self.db).await?;
        self.invalidate_list_cache().await;
        Ok(())
    }

    async fn invalidate_list_cache(&self) {
        if let Some(cache) = &self.cache {
            cache.invalidate(CACHE_KEY_FORUMS_LIST).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_key_constant() {
        assert_eq!(CACHE_KEY_FORUMS_LIST, "forums:list");
    }

    #[test]
    fn cache_ttl_value() {
        assert_eq!(CACHE_TTL_FORUMS, 300);
    }
}
