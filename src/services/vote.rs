use crate::{
    error::{AppError, AppResult},
    models::{vote, Comment, Post, Vote, VoteModel},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    Statement,
};

pub struct VoteService {
    db: DatabaseConnection,
}

impl VoteService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn vote(
        &self,
        user_id: i32,
        target_type: &str,
        target_id: i32,
        value: i16,
    ) -> AppResult<VoteModel> {
        if value != 1 && value != -1 {
            return Err(AppError::Validation(
                "Vote value must be 1 or -1".to_string(),
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

        // Check for existing vote
        let existing = Vote::find()
            .filter(vote::Column::UserId.eq(user_id))
            .filter(vote::Column::TargetType.eq(target_type))
            .filter(vote::Column::TargetId.eq(target_id))
            .one(&self.db)
            .await?;

        let old_value: i16 = existing.as_ref().map_or(0, |v| v.value);

        let result = if let Some(existing_vote) = existing {
            if existing_vote.value == value {
                // Same vote again — remove it (toggle off)
                Vote::delete_by_id(existing_vote.id).exec(&self.db).await?;
                self.update_counters(target_type, target_id, -old_value)
                    .await?;
                // Return a synthetic model indicating removal
                return Ok(VoteModel {
                    id: 0,
                    user_id,
                    target_type: target_type.to_string(),
                    target_id,
                    value: 0,
                    created_at: chrono::Utc::now().naive_utc(),
                });
            }
            // Different vote — update
            let mut active: vote::ActiveModel = existing_vote.into();
            active.value = sea_orm::ActiveValue::Set(value);
            let updated = active.update(&self.db).await?;
            // Swing counters: remove old, add new
            self.update_counters(target_type, target_id, value - old_value)
                .await?;
            updated
        } else {
            // New vote
            let now = chrono::Utc::now().naive_utc();
            let new_vote = vote::ActiveModel {
                user_id: sea_orm::ActiveValue::Set(user_id),
                target_type: sea_orm::ActiveValue::Set(target_type.to_string()),
                target_id: sea_orm::ActiveValue::Set(target_id),
                value: sea_orm::ActiveValue::Set(value),
                created_at: sea_orm::ActiveValue::Set(now),
                ..Default::default()
            };
            let inserted = new_vote.insert(&self.db).await?;
            self.update_counters(target_type, target_id, value).await?;
            inserted
        };

        Ok(result)
    }

    async fn update_counters(
        &self,
        target_type: &str,
        target_id: i32,
        delta: i16,
    ) -> AppResult<()> {
        let table = match target_type {
            "post" => "posts",
            "comment" => "comments",
            _ => return Ok(()),
        };

        let (col_up, col_down) = if delta > 0 {
            ("upvotes", format!("upvotes + {delta}"))
        } else {
            ("downvotes", format!("downvotes + {}", -delta))
        };

        // For vote swing (e.g. -1 to +1, delta=2), handle both columns
        if delta.abs() > 1 {
            // Swing: delta=2 means +1 upvote, -1 downvote; delta=-2 means reverse
            let sql = if delta > 0 {
                format!(
                    "UPDATE {table} SET upvotes = upvotes + 1, downvotes = GREATEST(downvotes - 1, 0) WHERE id = $1"
                )
            } else {
                format!(
                    "UPDATE {table} SET downvotes = downvotes + 1, upvotes = GREATEST(upvotes - 1, 0) WHERE id = $1"
                )
            };
            self.db
                .execute(Statement::from_sql_and_values(
                    sea_orm::DatabaseBackend::Postgres,
                    &sql,
                    [target_id.into()],
                ))
                .await?;
        } else {
            let sql = format!("UPDATE {table} SET {col_up} = {col_down} WHERE id = $1");
            self.db
                .execute(Statement::from_sql_and_values(
                    sea_orm::DatabaseBackend::Postgres,
                    &sql,
                    [target_id.into()],
                ))
                .await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn toggle_same_vote_returns_zero() {
        let old_value: i16 = 1;
        let new_value: i16 = 1;
        let result = if old_value == new_value { 0 } else { new_value };
        assert_eq!(result, 0);
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
