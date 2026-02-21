use crate::{
    error::{AppError, AppResult},
    models::{user, user_points_ledger, User, UserPointsLedger},
};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};

pub struct PointsService {
    db: DatabaseConnection,
}

impl PointsService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn apply_vote_points(
        &self,
        actor_user_id: i32,
        target_type: &str,
        target_id: i32,
        delta_points: i32,
    ) -> AppResult<()> {
        if delta_points == 0 {
            return Ok(());
        }

        let (author_user_id, ref_type, reason) = match target_type {
            "post" => {
                let post = crate::models::Post::find_by_id(target_id)
                    .one(&self.db)
                    .await?
                    .ok_or(AppError::NotFound)?;
                (post.user_id, "post", "vote_on_post")
            }
            "comment" => {
                let comment = crate::models::Comment::find_by_id(target_id)
                    .one(&self.db)
                    .await?
                    .ok_or(AppError::NotFound)?;
                (comment.user_id, "comment", "vote_on_comment")
            }
            _ => return Err(AppError::Validation("Invalid target type".to_string())),
        };

        // 不给自己加分（防刷的最小规则之一）
        if author_user_id == actor_user_id {
            return Ok(());
        }

        let txn = self.db.begin().await?;

        // 1) 记账（可审计/可回滚）
        let ledger = user_points_ledger::ActiveModel {
            user_id: Set(author_user_id),
            delta: Set(delta_points),
            reason: Set(reason.to_string()),
            ref_type: Set(ref_type.to_string()),
            ref_id: Set(target_id),
            actor_user_id: Set(actor_user_id),
            ..Default::default()
        };
        ledger.insert(&txn).await?;

        // 2) 汇总到 users.karma
        let result = User::update_many()
            .col_expr(
                user::Column::Karma,
                Expr::col(user::Column::Karma).add(delta_points),
            )
            .filter(user::Column::Id.eq(author_user_id))
            .exec(&txn)
            .await?;
        if result.rows_affected == 0 {
            return Err(AppError::NotFound);
        }

        txn.commit().await?;
        Ok(())
    }

    /// 将指定引用（ref_type/ref_id）产生的积分全部回滚（用于删帖/删评论等场景）。
    pub async fn rollback_by_ref(&self, ref_type: &str, ref_id: i32) -> AppResult<i64> {
        let txn = self.db.begin().await?;

        let entries = UserPointsLedger::find()
            .filter(user_points_ledger::Column::RefType.eq(ref_type))
            .filter(user_points_ledger::Column::RefId.eq(ref_id))
            .all(&txn)
            .await?;

        for e in &entries {
            User::update_many()
                .col_expr(
                    user::Column::Karma,
                    Expr::col(user::Column::Karma).sub(e.delta),
                )
                .filter(user::Column::Id.eq(e.user_id))
                .exec(&txn)
                .await?;
        }

        // 删除账本记录（也可以改为打标“rolled_back”，这里先做最小实现）
        UserPointsLedger::delete_many()
            .filter(user_points_ledger::Column::RefType.eq(ref_type))
            .filter(user_points_ledger::Column::RefId.eq(ref_id))
            .exec(&txn)
            .await?;

        txn.commit().await?;
        Ok(entries.len() as i64)
    }
}
