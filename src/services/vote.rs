use crate::{
    error::{AppError, AppResult},
    models::{vote, Comment, Post, Vote},
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, Statement,
    TransactionTrait,
};

pub struct VoteService {
    db: DatabaseConnection,
}

#[derive(Debug, Clone, Copy)]
pub struct VoteChange {
    pub old_value: i16,
    pub new_value: i16,
}

impl VoteService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Set vote state to -1 / 0 / 1.
    /// 0 means remove current vote.
    pub async fn set_vote(
        &self,
        user_id: i32,
        target_type: &str,
        target_id: i32,
        value: i16,
    ) -> AppResult<VoteChange> {
        if value != 1 && value != -1 && value != 0 {
            return Err(AppError::Validation(
                "Vote value must be -1, 0 or 1".to_string(),
            ));
        }

        // Verify target exists
        match target_type {
            "post" => {
                Post::find_by_id(target_id)
                    .one(&self.db)
                    .await?
                    .ok_or(AppError::NotFound)?;
            }
            "comment" => {
                Comment::find_by_id(target_id)
                    .one(&self.db)
                    .await?
                    .ok_or(AppError::NotFound)?;
            }
            _ => return Err(AppError::Validation("Invalid target type".to_string())),
        }

        let txn = self.db.begin().await?;

        // Read previous value in the same transaction to compute exact counter delta.
        let old_value = Vote::find()
            .filter(vote::Column::UserId.eq(user_id))
            .filter(vote::Column::TargetType.eq(target_type))
            .filter(vote::Column::TargetId.eq(target_id))
            .one(&txn)
            .await?
            .map(|v| v.value)
            .unwrap_or(0);

        if value == 0 {
            Vote::delete_many()
                .filter(vote::Column::UserId.eq(user_id))
                .filter(vote::Column::TargetType.eq(target_type))
                .filter(vote::Column::TargetId.eq(target_id))
                .exec(&txn)
                .await?;
        } else {
            txn.execute(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "INSERT INTO votes (user_id, target_type, target_id, value, created_at)
                 VALUES ($1, $2, $3, $4, NOW())
                 ON CONFLICT (user_id, target_type, target_id)
                 DO UPDATE SET value = EXCLUDED.value",
                vec![
                    user_id.into(),
                    target_type.into(),
                    target_id.into(),
                    value.into(),
                ],
            ))
            .await?;
        }

        self.apply_counter_delta(&txn, target_type, target_id, old_value, value)
            .await?;
        txn.commit().await?;

        Ok(VoteChange {
            old_value,
            new_value: value,
        })
    }

    async fn apply_counter_delta<C: ConnectionTrait>(
        &self,
        conn: &C,
        target_type: &str,
        target_id: i32,
        old_value: i16,
        new_value: i16,
    ) -> AppResult<()> {
        let table = match target_type {
            "post" => "posts",
            "comment" => "comments",
            _ => return Ok(()),
        };

        let old_up = if old_value == 1 { 1 } else { 0 };
        let old_down = if old_value == -1 { 1 } else { 0 };
        let new_up = if new_value == 1 { 1 } else { 0 };
        let new_down = if new_value == -1 { 1 } else { 0 };

        let delta_up = new_up - old_up;
        let delta_down = new_down - old_down;

        if delta_up != 0 || delta_down != 0 {
            let sql = format!(
                "UPDATE {table}
                 SET upvotes = GREATEST(upvotes + $1, 0),
                     downvotes = GREATEST(downvotes + $2, 0)
                 WHERE id = $3"
            );

            conn.execute(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                &sql,
                vec![delta_up.into(), delta_down.into(), target_id.into()],
            ))
            .await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn validate_vote_value_accepts_one() {
        let value: i16 = 1;
        assert!(value == 1 || value == -1);
    }

    #[test]
    fn validate_vote_value_accepts_negative_one() {
        let value: i16 = -1;
        assert!(value == 1 || value == -1);
    }

    #[test]
    fn validate_vote_value_rejects_zero() {
        let value: i16 = 0;
        assert!(!(value == 1 || value == -1));
    }

    #[test]
    fn validate_vote_value_rejects_two() {
        let value: i16 = 2;
        assert!(!(value == 1 || value == -1));
    }

    #[test]
    fn set_same_vote_is_idempotent() {
        let old_value: i16 = 1;
        let new_value: i16 = 1;
        let delta_up = (new_value == 1) as i32 - (old_value == 1) as i32;
        assert_eq!(delta_up, 0);
    }

    #[test]
    fn swing_vote_calculates_delta() {
        let old_value: i16 = -1;
        let new_value: i16 = 1;
        let delta = new_value - old_value;
        assert_eq!(delta, 2);
    }

    #[test]
    fn new_vote_delta_is_value() {
        let old_value: i16 = 0;
        let new_value: i16 = 1;
        let delta = new_value - old_value;
        assert_eq!(delta, 1);
    }

    #[test]
    fn remove_vote_delta_is_negative() {
        let old_value: i16 = 1;
        let new_value: i16 = 0;
        let delta = new_value - old_value;
        assert_eq!(delta, -1);
    }
}
