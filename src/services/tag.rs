use crate::error::{AppError, AppResult};
use crate::models::{post_tag, tag, PostModel, Tag, TagModel};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    FromQueryResult, ModelTrait, QueryFilter, QueryOrder, Set, Statement,
};

pub struct TagService {
    db: DatabaseConnection,
}

impl TagService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Get or create tags by name. Returns all matching TagModels.
    pub async fn get_or_create_tags(&self, names: Vec<String>) -> AppResult<Vec<TagModel>> {
        let mut result = Vec::new();

        for name in names {
            let name = name.trim().to_lowercase();
            if name.is_empty() || name.len() > 30 {
                continue;
            }

            let slug = name
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '-' })
                .collect::<String>();

            // Try to find existing tag
            let existing = Tag::find()
                .filter(tag::Column::Slug.eq(&slug))
                .one(&self.db)
                .await?;

            if let Some(tag) = existing {
                result.push(tag);
            } else {
                let now = chrono::Utc::now().naive_utc();
                let new_tag = tag::ActiveModel {
                    name: sea_orm::ActiveValue::Set(name),
                    slug: sea_orm::ActiveValue::Set(slug),
                    created_at: sea_orm::ActiveValue::Set(now),
                    ..Default::default()
                };
                let tag = new_tag.insert(&self.db).await?;
                result.push(tag);
            }
        }

        Ok(result)
    }

    /// Replace all tags for a post.
    pub async fn set_post_tags(&self, post_id: i32, tag_ids: Vec<i32>) -> AppResult<()> {
        // Delete existing tags
        self.db
            .execute(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "DELETE FROM post_tags WHERE post_id = $1",
                vec![post_id.into()],
            ))
            .await?;

        // Insert new tags
        for tag_id in tag_ids {
            let pt = post_tag::ActiveModel {
                post_id: sea_orm::ActiveValue::Set(post_id),
                tag_id: sea_orm::ActiveValue::Set(tag_id),
                ..Default::default()
            };
            pt.insert(&self.db).await?;
        }

        Ok(())
    }

    /// Get tags for a single post.
    pub async fn get_post_tags(&self, post_id: i32) -> AppResult<Vec<TagModel>> {
        let tags = TagModel::find_by_statement(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT t.id, t.name, t.slug, t.created_at \
                FROM tags t \
                INNER JOIN post_tags pt ON pt.tag_id = t.id \
                WHERE pt.post_id = $1 \
                ORDER BY t.name",
            vec![post_id.into()],
        ))
        .all(&self.db)
        .await?;

        Ok(tags)
    }

    /// Get tags for multiple posts (batch).
    pub async fn get_tags_for_posts(
        &self,
        post_ids: &[i32],
    ) -> AppResult<std::collections::HashMap<i32, Vec<String>>> {
        use std::collections::HashMap;

        if post_ids.is_empty() {
            return Ok(HashMap::new());
        }

        // Build $1, $2, ... placeholders
        let placeholders: Vec<String> = post_ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", i + 1))
            .collect();
        let sql = format!(
            "SELECT pt.post_id, t.name \
                FROM post_tags pt \
                INNER JOIN tags t ON t.id = pt.tag_id \
                WHERE pt.post_id IN ({}) \
                ORDER BY t.name",
            placeholders.join(", ")
        );

        let values: Vec<sea_orm::Value> = post_ids.iter().map(|&id| id.into()).collect();

        let rows = self
            .db
            .query_all(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                &sql,
                values,
            ))
            .await?;

        let mut map: HashMap<i32, Vec<String>> = HashMap::new();
        for row in rows {
            let pid: i32 = row.try_get_by_index(0)?;
            let name: String = row.try_get_by_index(1)?;
            map.entry(pid).or_default().push(name);
        }

        Ok(map)
    }

    /// List all tags.
    pub async fn list_tags(&self) -> AppResult<Vec<TagModel>> {
        let tags = Tag::find()
            .order_by_asc(tag::Column::Name)
            .all(&self.db)
            .await?;
        Ok(tags)
    }

    /// Get posts by tag slug with pagination.
    pub async fn get_posts_by_tag(
        &self,
        tag_slug: &str,
        page: u64,
        per_page: u64,
    ) -> AppResult<(Vec<PostModel>, u64)> {
        let offset = page.saturating_sub(1) * per_page;

        // Look up tag
        let tag = Tag::find()
            .filter(tag::Column::Slug.eq(tag_slug))
            .one(&self.db)
            .await?
            .ok_or(AppError::NotFound)?;

        // Count
        let count_result = self
            .db
            .query_one(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT COUNT(*) as count FROM posts p \
                    INNER JOIN post_tags pt ON pt.post_id = p.id \
                    WHERE pt.tag_id = $1 AND p.is_hidden = FALSE",
                vec![tag.id.into()],
            ))
            .await?
            .ok_or(AppError::Internal(anyhow::anyhow!("Count query failed")))?;

        let total: i64 = count_result.try_get_by_index(0)?;

        // Fetch
        let posts = PostModel::find_by_statement(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT p.id, p.user_id, p.forum_id, p.title, p.content, p.upvotes, p.downvotes, \
                p.view_count, p.is_pinned, p.is_locked, p.is_hidden, p.created_at, p.updated_at \
                FROM posts p \
                INNER JOIN post_tags pt ON pt.post_id = p.id \
                WHERE pt.tag_id = $1 AND p.is_hidden = FALSE \
                ORDER BY p.created_at DESC \
                LIMIT $2 OFFSET $3",
            vec![
                tag.id.into(),
                (per_page as i64).into(),
                (offset as i64).into(),
            ],
        ))
        .all(&self.db)
        .await?;

        Ok((posts, total as u64))
    }

    pub async fn create_tag(&self, name: &str) -> AppResult<TagModel> {
        let name = name.trim().to_lowercase();
        let slug = name
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>();

        let existing = Tag::find()
            .filter(tag::Column::Slug.eq(&slug))
            .one(&self.db)
            .await?;
        if existing.is_some() {
            return Err(AppError::Conflict("Tag already exists".to_string()));
        }

        let now = chrono::Utc::now().naive_utc();
        let new_tag = tag::ActiveModel {
            name: Set(name),
            slug: Set(slug),
            created_at: Set(now),
            ..Default::default()
        };
        Ok(new_tag.insert(&self.db).await?)
    }

    pub async fn update_tag(&self, id: i32, name: &str) -> AppResult<TagModel> {
        let tag = Tag::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or(AppError::NotFound)?;
        let name = name.trim().to_lowercase();
        let slug = name
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>();

        let mut active: tag::ActiveModel = tag.into();
        active.name = Set(name);
        active.slug = Set(slug);
        Ok(active.update(&self.db).await?)
    }

    pub async fn delete_tag(&self, id: i32) -> AppResult<()> {
        let tag = Tag::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or(AppError::NotFound)?;
        tag.delete(&self.db).await?;
        Ok(())
    }
}
