use crate::{
    error::{AppError, AppResult},
    models::{comment, post, report, Comment, Post, Report, ReportModel},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder,
};

pub struct ReportService {
    db: DatabaseConnection,
}

impl ReportService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn create_report(
        &self,
        reporter_id: i32,
        target_type: &str,
        target_id: i32,
        reason: &str,
        description: Option<&str>,
    ) -> AppResult<ReportModel> {
        // Validate target_type
        if target_type != "post" && target_type != "comment" {
            return Err(AppError::Validation(
                "target_type must be 'post' or 'comment'".to_string(),
            ));
        }

        // Validate reason
        let valid_reasons = ["spam", "harassment", "inappropriate", "other"];
        if !valid_reasons.contains(&reason) {
            return Err(AppError::Validation(format!(
                "reason must be one of: {}",
                valid_reasons.join(", ")
            )));
        }

        // Verify target exists
        match target_type {
            "post" => {
                Post::find_by_id(target_id)
                    .one(&self.db)
                    .await?
                    .ok_or(AppError::Validation("Post not found".to_string()))?;
            }
            "comment" => {
                Comment::find_by_id(target_id)
                    .one(&self.db)
                    .await?
                    .ok_or(AppError::Validation("Comment not found".to_string()))?;
            }
            _ => unreachable!(),
        }

        let now = chrono::Utc::now().naive_utc();
        let model = report::ActiveModel {
            reporter_id: sea_orm::ActiveValue::Set(reporter_id),
            target_type: sea_orm::ActiveValue::Set(target_type.to_string()),
            target_id: sea_orm::ActiveValue::Set(target_id),
            reason: sea_orm::ActiveValue::Set(reason.to_string()),
            description: sea_orm::ActiveValue::Set(description.map(|s| s.to_string())),
            status: sea_orm::ActiveValue::Set("pending".to_string()),
            created_at: sea_orm::ActiveValue::Set(now),
            ..Default::default()
        };

        let saved = model.insert(&self.db).await?;
        Ok(saved)
    }

    pub async fn list_reports(
        &self,
        status: Option<&str>,
        page: u64,
        per_page: u64,
    ) -> AppResult<(Vec<ReportModel>, u64)> {
        let mut query = Report::find();

        if let Some(s) = status {
            query = query.filter(report::Column::Status.eq(s));
        }

        let paginator = query
            .order_by_desc(report::Column::CreatedAt)
            .paginate(&self.db, per_page);

        let total = paginator.num_items().await?;
        let reports = paginator.fetch_page(page.saturating_sub(1)).await?;
        Ok((reports, total))
    }

    pub async fn resolve(
        &self,
        report_id: i32,
        admin_id: i32,
        action: &str,
    ) -> AppResult<ReportModel> {
        let valid_actions = ["hide", "delete", "dismiss"];
        if !valid_actions.contains(&action) {
            return Err(AppError::Validation(format!(
                "action must be one of: {}",
                valid_actions.join(", ")
            )));
        }

        let existing = Report::find_by_id(report_id)
            .one(&self.db)
            .await?
            .ok_or(AppError::NotFound)?;

        if existing.status != "pending" {
            return Err(AppError::Validation(
                "Report is already resolved".to_string(),
            ));
        }

        // Apply action on the target
        match action {
            "hide" => {
                self.hide_target(&existing.target_type, existing.target_id)
                    .await?;
            }
            "delete" => {
                self.delete_target(&existing.target_type, existing.target_id)
                    .await?;
            }
            "dismiss" => {}
            _ => unreachable!(),
        }

        let now = chrono::Utc::now().naive_utc();
        let mut active: report::ActiveModel = existing.into();
        active.status = sea_orm::ActiveValue::Set(if action == "dismiss" {
            "dismissed".to_string()
        } else {
            "resolved".to_string()
        });
        active.resolved_by = sea_orm::ActiveValue::Set(Some(admin_id));
        active.resolved_at = sea_orm::ActiveValue::Set(Some(now));

        let updated = active.update(&self.db).await?;
        Ok(updated)
    }

    async fn hide_target(&self, target_type: &str, target_id: i32) -> AppResult<()> {
        match target_type {
            "post" => {
                let existing = Post::find_by_id(target_id)
                    .one(&self.db)
                    .await?
                    .ok_or(AppError::NotFound)?;
                let mut active: post::ActiveModel = existing.into();
                active.is_hidden = sea_orm::ActiveValue::Set(true);
                active.update(&self.db).await?;
            }
            "comment" => {
                let existing = Comment::find_by_id(target_id)
                    .one(&self.db)
                    .await?
                    .ok_or(AppError::NotFound)?;
                let mut active: comment::ActiveModel = existing.into();
                active.is_hidden = sea_orm::ActiveValue::Set(true);
                active.update(&self.db).await?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn delete_target(&self, target_type: &str, target_id: i32) -> AppResult<()> {
        match target_type {
            "post" => {
                Post::delete_by_id(target_id).exec(&self.db).await?;
            }
            "comment" => {
                Comment::delete_by_id(target_id).exec(&self.db).await?;
            }
            _ => {}
        }
        Ok(())
    }
}
