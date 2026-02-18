use crate::{
    error::AppResult,
    models::{notification, Notification, NotificationModel},
    websocket::hub::NotificationHub,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder,
};

pub struct NotificationService {
    db: DatabaseConnection,
    hub: NotificationHub,
}

impl NotificationService {
    pub fn new(db: DatabaseConnection, hub: NotificationHub) -> Self {
        Self { db, hub }
    }

    pub async fn notify(
        &self,
        user_id: i32,
        actor_id: i32,
        kind: &str,
        target_type: &str,
        target_id: i32,
        message: &str,
    ) -> AppResult<()> {
        // Don't notify yourself
        if user_id == actor_id {
            return Ok(());
        }

        let now = chrono::Utc::now().naive_utc();
        let model = notification::ActiveModel {
            user_id: sea_orm::ActiveValue::Set(user_id),
            kind: sea_orm::ActiveValue::Set(kind.to_string()),
            actor_id: sea_orm::ActiveValue::Set(actor_id),
            target_type: sea_orm::ActiveValue::Set(target_type.to_string()),
            target_id: sea_orm::ActiveValue::Set(target_id),
            message: sea_orm::ActiveValue::Set(message.to_string()),
            is_read: sea_orm::ActiveValue::Set(false),
            created_at: sea_orm::ActiveValue::Set(now),
            ..Default::default()
        };

        let saved = model.insert(&self.db).await?;

        // Push via WebSocket
        let json = serde_json::json!({
            "type": "notification",
            "data": {
                "id": saved.id,
                "kind": &saved.kind,
                "message": &saved.message,
                "target_type": &saved.target_type,
                "target_id": saved.target_id,
                "created_at": saved.created_at.to_string(),
            }
        });
        self.hub.send_to_user(user_id, &json.to_string());

        Ok(())
    }

    pub async fn list_for_user(
        &self,
        user_id: i32,
        page: u64,
        per_page: u64,
    ) -> AppResult<(Vec<NotificationModel>, u64)> {
        let paginator = Notification::find()
            .filter(notification::Column::UserId.eq(user_id))
            .order_by_desc(notification::Column::CreatedAt)
            .paginate(&self.db, per_page);

        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page.saturating_sub(1)).await?;
        Ok((items, total))
    }

    pub async fn unread_count(&self, user_id: i32) -> AppResult<u64> {
        let count = Notification::find()
            .filter(notification::Column::UserId.eq(user_id))
            .filter(notification::Column::IsRead.eq(false))
            .count(&self.db)
            .await?;
        Ok(count)
    }

    pub async fn mark_read(&self, id: i32, user_id: i32) -> AppResult<()> {
        let existing = Notification::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or(crate::error::AppError::NotFound)?;

        if existing.user_id != user_id {
            return Err(crate::error::AppError::Forbidden);
        }

        let mut active: notification::ActiveModel = existing.into();
        active.is_read = sea_orm::ActiveValue::Set(true);
        active.update(&self.db).await?;
        Ok(())
    }

    pub async fn mark_all_read(&self, user_id: i32) -> AppResult<u64> {
        use sea_orm::sea_query::Expr;
        let result = Notification::update_many()
            .col_expr(notification::Column::IsRead, Expr::value(true))
            .filter(notification::Column::UserId.eq(user_id))
            .filter(notification::Column::IsRead.eq(false))
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected)
    }
}

#[cfg(test)]
mod tests {
    fn should_notify(user_id: i32, actor_id: i32) -> bool {
        user_id != actor_id
    }

    #[test]
    fn test_no_self_notification() {
        assert!(!should_notify(1, 1));
    }

    #[test]
    fn test_notify_different_user() {
        assert!(should_notify(1, 2));
    }
}
