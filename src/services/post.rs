use crate::{
    error::{AppError, AppResult},
    models::{post, Post, PostModel},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    FromQueryResult, PaginatorTrait, QueryFilter, QueryOrder, Statement,
};

pub struct PostService {
    db: DatabaseConnection,
}

impl PostService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn list_by_forum(
        &self,
        forum_id: i32,
        page: u64,
        per_page: u64,
        sort: &str,
    ) -> AppResult<(Vec<PostModel>, u64)> {
        match sort {
            "top" | "hot" => self.list_by_forum_raw(forum_id, page, per_page, sort).await,
            _ => {
                // "new" (default): use SeaORM paginator
                let paginator = Post::find()
                    .filter(post::Column::ForumId.eq(forum_id))
                    .filter(post::Column::IsHidden.eq(false))
                    .order_by_desc(post::Column::IsPinned)
                    .order_by_desc(post::Column::CreatedAt)
                    .paginate(&self.db, per_page);

                let total = paginator.num_items().await?;
                let posts = paginator.fetch_page(page.saturating_sub(1)).await?;

                Ok((posts, total))
            }
        }
    }

    async fn list_by_forum_raw(
        &self,
        forum_id: i32,
        page: u64,
        per_page: u64,
        sort: &str,
    ) -> AppResult<(Vec<PostModel>, u64)> {
        let offset = page.saturating_sub(1) * per_page;

        let author_weight: f64 = std::env::var("POST_AUTHOR_KARMA_WEIGHT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.2);

        let order_clause = match sort {
            "top" => format!(
                "p.is_pinned DESC, \
                ((p.upvotes - p.downvotes) + (LN(GREATEST(u.karma, 0) + 1) * {author_weight})) DESC, \
                p.created_at DESC"
            ),
            "hot" => format!(
                "p.is_pinned DESC, \
                (((p.upvotes - p.downvotes) + (LN(GREATEST(u.karma, 0) + 1) * {author_weight}))::float / \
                POWER(EXTRACT(EPOCH FROM (NOW() - p.created_at)) / 3600.0 + 2.0, 1.5)) DESC, \
                p.created_at DESC"
            ),
            _ => "p.is_pinned DESC, p.created_at DESC".to_string(),
        };

        let count_sql = "SELECT COUNT(*) as count FROM posts \
            WHERE forum_id = $1 AND is_hidden = FALSE";

        let search_sql = format!(
            "SELECT p.id, p.user_id, p.forum_id, p.title, p.content, p.upvotes, p.downvotes, \
                p.view_count, p.is_pinned, p.is_locked, p.is_hidden, p.created_at, p.updated_at \
                FROM posts p \
                JOIN users u ON u.id = p.user_id \
                WHERE p.forum_id = $1 AND p.is_hidden = FALSE \
                ORDER BY {} \
                LIMIT $2 OFFSET $3",
            order_clause
        );

        let count_result = self
            .db
            .query_one(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                count_sql,
                vec![forum_id.into()],
            ))
            .await?
            .ok_or(AppError::Internal(anyhow::anyhow!("Count query failed")))?;

        let total: i64 = count_result.try_get_by_index(0)?;

        let posts = PostModel::find_by_statement(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            &search_sql,
            vec![
                forum_id.into(),
                (per_page as i64).into(),
                (offset as i64).into(),
            ],
        ))
        .all(&self.db)
        .await?;

        Ok((posts, total as u64))
    }

    pub async fn get_by_id(&self, id: i32) -> AppResult<PostModel> {
        Post::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or(AppError::NotFound)
    }

    pub async fn create(
        &self,
        user_id: i32,
        forum_id: i32,
        title: &str,
        content: &str,
    ) -> AppResult<PostModel> {
        let now = chrono::Utc::now().naive_utc();

        let new_post = post::ActiveModel {
            user_id: sea_orm::ActiveValue::Set(user_id),
            forum_id: sea_orm::ActiveValue::Set(forum_id),
            title: sea_orm::ActiveValue::Set(title.to_string()),
            content: sea_orm::ActiveValue::Set(content.to_string()),
            upvotes: sea_orm::ActiveValue::Set(0),
            downvotes: sea_orm::ActiveValue::Set(0),
            view_count: sea_orm::ActiveValue::Set(0),
            is_pinned: sea_orm::ActiveValue::Set(false),
            is_locked: sea_orm::ActiveValue::Set(false),
            created_at: sea_orm::ActiveValue::Set(now),
            updated_at: sea_orm::ActiveValue::Set(now),
            ..Default::default()
        };

        let post = new_post.insert(&self.db).await?;
        Ok(post)
    }

    pub async fn update(
        &self,
        id: i32,
        user_id: i32,
        title: &str,
        content: &str,
    ) -> AppResult<PostModel> {
        let existing = self.get_by_id(id).await?;
        if existing.user_id != user_id {
            return Err(AppError::Forbidden);
        }

        let now = chrono::Utc::now().naive_utc();

        let mut active: post::ActiveModel = existing.into();
        active.title = sea_orm::ActiveValue::Set(title.to_string());
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

        Post::delete_by_id(id).exec(&self.db).await?;
        Ok(())
    }

    pub async fn increment_view_count(&self, id: i32) -> AppResult<()> {
        self.db
            .execute(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "UPDATE posts SET view_count = view_count + 1 WHERE id = $1",
                [id.into()],
            ))
            .await?;
        Ok(())
    }

    pub async fn toggle_pin(&self, id: i32) -> AppResult<PostModel> {
        let existing = self.get_by_id(id).await?;
        let mut active: post::ActiveModel = existing.clone().into();
        active.is_pinned = sea_orm::ActiveValue::Set(!existing.is_pinned);
        let updated = active.update(&self.db).await?;
        Ok(updated)
    }

    pub async fn toggle_lock(&self, id: i32) -> AppResult<PostModel> {
        let existing = self.get_by_id(id).await?;
        let mut active: post::ActiveModel = existing.clone().into();
        active.is_locked = sea_orm::ActiveValue::Set(!existing.is_locked);
        let updated = active.update(&self.db).await?;
        Ok(updated)
    }

    pub async fn search(
        &self,
        query: &str,
        forum_id: Option<i32>,
        page: u64,
        per_page: u64,
        sort: &str,
    ) -> AppResult<(Vec<PostModel>, u64)> {
        let offset = page.saturating_sub(1) * per_page;

        let author_weight: f64 = std::env::var("POST_AUTHOR_KARMA_WEIGHT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.2);

        let order_clause = match sort {
            "new" => "p.created_at DESC".to_string(),
            "top" => format!(
                "((p.upvotes - p.downvotes) + (LN(GREATEST(u.karma, 0) + 1) * {author_weight})) DESC, p.created_at DESC"
            ),
            _ => format!(
                "(ts_rank(p.search_vector, plainto_tsquery('english', $1)) + (LN(GREATEST(u.karma, 0) + 1) * {author_weight} * 0.05)) DESC"
            ),
        };

        // Build parameterized queries — all values passed via bind params
        let (count_sql, search_sql, values) = if let Some(fid) = forum_id {
            let count = "SELECT COUNT(*) as count FROM posts \
                WHERE search_vector @@ plainto_tsquery('english', $1) \
                AND is_hidden = FALSE AND forum_id = $2";
            let search = format!(
                "SELECT p.id, p.user_id, p.forum_id, p.title, p.content, p.upvotes, p.downvotes, \
                    p.view_count, p.is_pinned, p.is_locked, p.is_hidden, p.created_at, p.updated_at \
                    FROM posts p \
                    JOIN users u ON u.id = p.user_id \
                    WHERE p.search_vector @@ plainto_tsquery('english', $1) \
                    AND p.is_hidden = FALSE AND p.forum_id = $2 \
                    ORDER BY {} \
                    LIMIT $3 OFFSET $4",
                order_clause
            );
            let vals: Vec<sea_orm::Value> = vec![
                query.into(),
                fid.into(),
                (per_page as i64).into(),
                (offset as i64).into(),
            ];
            (count.to_string(), search, vals)
        } else {
            let count = "SELECT COUNT(*) as count FROM posts \
                WHERE search_vector @@ plainto_tsquery('english', $1) \
                AND is_hidden = FALSE";
            let search = format!(
                "SELECT p.id, p.user_id, p.forum_id, p.title, p.content, p.upvotes, p.downvotes, \
                    p.view_count, p.is_pinned, p.is_locked, p.is_hidden, p.created_at, p.updated_at \
                    FROM posts p \
                    JOIN users u ON u.id = p.user_id \
                    WHERE p.search_vector @@ plainto_tsquery('english', $1) \
                    AND p.is_hidden = FALSE \
                    ORDER BY {} \
                    LIMIT $2 OFFSET $3",
                order_clause
            );
            let vals: Vec<sea_orm::Value> = vec![
                query.into(),
                (per_page as i64).into(),
                (offset as i64).into(),
            ];
            (count.to_string(), search, vals)
        };

        // Count total matching rows
        let count_result = self
            .db
            .query_one(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                &count_sql,
                values[..if forum_id.is_some() { 2 } else { 1 }].to_vec(),
            ))
            .await?
            .ok_or(AppError::Internal(anyhow::anyhow!("Count query failed")))?;

        let total: i64 = count_result.try_get_by_index(0)?;

        // Fetch paginated results
        let posts = PostModel::find_by_statement(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            &search_sql,
            values,
        ))
        .all(&self.db)
        .await?;

        Ok((posts, total as u64))
    }
}

#[cfg(test)]
mod tests {
    fn get_order_clause(sort: &str) -> &str {
        match sort {
            "top" => "is_pinned DESC, (upvotes - downvotes) DESC, created_at DESC",
            "hot" => "is_pinned DESC, (upvotes - downvotes)::float / POWER(EXTRACT(EPOCH FROM (NOW() - created_at)) / 3600.0 + 2.0, 1.5) DESC, created_at DESC",
            _ => "is_pinned DESC, created_at DESC",
        }
    }

    fn calculate_offset(page: u64, per_page: u64) -> u64 {
        page.saturating_sub(1) * per_page
    }

    #[test]
    fn test_sort_top_prioritizes_score() {
        let clause = get_order_clause("top");
        assert!(clause.contains("(upvotes - downvotes)"));
        assert!(clause.starts_with("is_pinned DESC"));
    }

    #[test]
    fn test_sort_hot_uses_time_decay() {
        let clause = get_order_clause("hot");
        assert!(clause.contains("POWER"));
        assert!(clause.contains("EXTRACT(EPOCH"));
    }

    #[test]
    fn test_pagination_first_page() {
        assert_eq!(calculate_offset(1, 20), 0);
    }

    #[test]
    fn test_pagination_second_page() {
        assert_eq!(calculate_offset(2, 20), 20);
    }

    #[test]
    fn test_pagination_zero_page_safe() {
        assert_eq!(calculate_offset(0, 20), 0);
    }
}
