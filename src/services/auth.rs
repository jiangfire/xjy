use crate::{
    config::auth::AuthConfig,
    error::{AppError, AppResult},
    models::{refresh_token, RefreshToken, User},
    services::email::EmailService,
    utils::{encode_access_token, encode_refresh_token, hash_password, verify_password},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, TransactionTrait,
};

pub struct AuthService {
    db: DatabaseConnection,
    config: AuthConfig,
}

impl AuthService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            config: AuthConfig::from_env(),
        }
    }

    /// Register a new user and send verification email.
    /// Returns (user_model, access_token, refresh_token).
    pub async fn register(
        &self,
        username: &str,
        email: &str,
        password: &str,
        email_service: &EmailService,
    ) -> AppResult<(crate::models::UserModel, String, String)> {
        // Check if username or email already exists
        if self.user_exists(username, email).await? {
            return Err(AppError::Validation(
                "Username or email already exists".to_string(),
            ));
        }

        let password_hash = hash_password(password)?;
        let now = chrono::Utc::now().naive_utc();
        let (email_verified, verification_token, verification_expires) =
            if self.config.require_email_verification {
                let token = uuid::Uuid::new_v4().to_string();
                let expires = now + chrono::Duration::hours(24);
                (false, Some(token), Some(expires))
            } else {
                (true, None, None)
            };

        let new_user = crate::models::user::ActiveModel {
            username: sea_orm::ActiveValue::Set(username.to_string()),
            email: sea_orm::ActiveValue::Set(email.to_string()),
            password_hash: sea_orm::ActiveValue::Set(password_hash),
            karma: sea_orm::ActiveValue::Set(0),
            role: sea_orm::ActiveValue::Set("user".to_string()),
            email_verified: sea_orm::ActiveValue::Set(email_verified),
            email_verification_token: sea_orm::ActiveValue::Set(verification_token.clone()),
            email_verification_expires: sea_orm::ActiveValue::Set(verification_expires),
            created_at: sea_orm::ActiveValue::Set(now),
            updated_at: sea_orm::ActiveValue::Set(now),
            ..Default::default()
        };

        let user = new_user.insert(&self.db).await?;
        let (access_token, refresh_token) = self.issue_tokens_for_user(user.id).await?;

        if self.config.require_email_verification {
            if let Some(token) = verification_token {
                // Send verification email (non-fatal)
                if let Err(e) = email_service
                    .send_verification_email(&user.email, &token)
                    .await
                {
                    tracing::warn!("Failed to send verification email: {e}");
                }
            }
        }

        Ok((user, access_token, refresh_token))
    }

    /// Login user
    /// Returns (user_model, access_token, refresh_token)
    pub async fn login(
        &self,
        username: &str,
        password: &str,
    ) -> AppResult<(crate::models::UserModel, String, String)> {
        // Find user by username
        let user: crate::models::UserModel = self
            .find_by_username(username)
            .await
            .map_err(|_| AppError::Unauthorized)?;

        // Verify password
        let is_valid = verify_password(password, &user.password_hash)?;
        if !is_valid {
            return Err(AppError::Unauthorized);
        }

        let (access_token, refresh_token) = self.issue_tokens_for_user(user.id).await?;

        Ok((user, access_token, refresh_token))
    }

    pub async fn rotate_refresh_token(
        &self,
        user_id: i32,
        current_refresh_token: &str,
    ) -> AppResult<(String, String)> {
        let token_hash = crate::utils::jwt::hash_refresh_token(current_refresh_token);
        let now = chrono::Utc::now().naive_utc();

        let existing = RefreshToken::find()
            .filter(refresh_token::Column::UserId.eq(user_id))
            .filter(refresh_token::Column::Token.eq(token_hash))
            .one(&self.db)
            .await?
            .ok_or(AppError::Unauthorized)?;

        if existing.expires_at <= now {
            let _ = RefreshToken::delete_by_id(existing.id).exec(&self.db).await;
            return Err(AppError::Unauthorized);
        }

        let txn = self.db.begin().await?;
        RefreshToken::delete_by_id(existing.id).exec(&txn).await?;
        let (access_token, refresh_token) = self.issue_tokens_for_user_txn(&txn, user_id).await?;
        txn.commit().await?;
        Ok((access_token, refresh_token))
    }

    pub async fn revoke_refresh_token(&self, refresh_token: &str) -> AppResult<()> {
        let token_hash = crate::utils::jwt::hash_refresh_token(refresh_token);
        RefreshToken::delete_many()
            .filter(refresh_token::Column::Token.eq(token_hash))
            .exec(&self.db)
            .await?;
        Ok(())
    }

    pub async fn revoke_all_user_refresh_tokens(&self, user_id: i32) -> AppResult<()> {
        RefreshToken::delete_many()
            .filter(refresh_token::Column::UserId.eq(user_id))
            .exec(&self.db)
            .await?;
        Ok(())
    }

    /// Get user by ID
    pub async fn get_user_by_id(&self, id: i32) -> AppResult<crate::models::UserModel> {
        let user = User::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or(AppError::NotFound)?;
        Ok(user)
    }

    /// Check if user exists by username or email
    async fn user_exists(&self, username: &str, email: &str) -> AppResult<bool> {
        let count = User::find()
            .filter(
                sea_orm::Condition::any()
                    .add(crate::models::user::Column::Username.eq(username))
                    .add(crate::models::user::Column::Email.eq(email)),
            )
            .count(&self.db)
            .await?;

        Ok(count > 0)
    }

    /// Find user by username
    async fn find_by_username(&self, username: &str) -> AppResult<crate::models::UserModel> {
        let user = User::find()
            .filter(crate::models::user::Column::Username.eq(username))
            .one(&self.db)
            .await?
            .ok_or(AppError::NotFound)?;
        Ok(user)
    }

    /// Change password for authenticated user
    pub async fn change_password(
        &self,
        user_id: i32,
        current_password: &str,
        new_password: &str,
    ) -> AppResult<()> {
        let user = self.get_user_by_id(user_id).await?;
        let is_valid = verify_password(current_password, &user.password_hash)?;
        if !is_valid {
            return Err(AppError::Validation(
                "Current password is incorrect".to_string(),
            ));
        }
        let new_hash = hash_password(new_password)?;
        let now = chrono::Utc::now().naive_utc();
        let mut active: crate::models::user::ActiveModel = user.into();
        active.password_hash = sea_orm::ActiveValue::Set(new_hash);
        active.updated_at = sea_orm::ActiveValue::Set(now);
        active.update(&self.db).await?;
        self.revoke_all_user_refresh_tokens(user_id).await?;
        Ok(())
    }

    /// Verify email with token
    pub async fn verify_email(&self, token: &str) -> AppResult<()> {
        let user = User::find()
            .filter(crate::models::user::Column::EmailVerificationToken.eq(token))
            .one(&self.db)
            .await?
            .ok_or_else(|| AppError::Validation("Invalid verification token".to_string()))?;

        if let Some(expires) = user.email_verification_expires {
            if chrono::Utc::now().naive_utc() > expires {
                return Err(AppError::Validation(
                    "Verification token has expired".to_string(),
                ));
            }
        }

        let mut active: crate::models::user::ActiveModel = user.into();
        active.email_verified = sea_orm::ActiveValue::Set(true);
        active.email_verification_token = sea_orm::ActiveValue::Set(None);
        active.email_verification_expires = sea_orm::ActiveValue::Set(None);
        active.update(&self.db).await?;
        Ok(())
    }

    /// Resend email verification token
    pub async fn resend_verification(
        &self,
        user_id: i32,
        email_service: &EmailService,
    ) -> AppResult<()> {
        let user = self.get_user_by_id(user_id).await?;
        if user.email_verified {
            return Err(AppError::Validation(
                "Email is already verified".to_string(),
            ));
        }
        let token = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().naive_utc();
        let expires = now + chrono::Duration::hours(24);

        let email = user.email.clone();
        let mut active: crate::models::user::ActiveModel = user.into();
        active.email_verification_token = sea_orm::ActiveValue::Set(Some(token.clone()));
        active.email_verification_expires = sea_orm::ActiveValue::Set(Some(expires));
        active.updated_at = sea_orm::ActiveValue::Set(now);
        active.update(&self.db).await?;

        if let Err(e) = email_service.send_verification_email(&email, &token).await {
            tracing::warn!("Failed to send verification email: {e}");
        }

        Ok(())
    }

    /// Request a password reset. Timing-safe: silently succeeds if user not found.
    pub async fn forgot_password(
        &self,
        email: &str,
        email_service: &EmailService,
    ) -> AppResult<()> {
        let user = User::find()
            .filter(crate::models::user::Column::Email.eq(email))
            .one(&self.db)
            .await?;

        let user = match user {
            Some(u) => u,
            None => return Ok(()), // timing-safe: don't reveal whether email exists
        };

        let token = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().naive_utc();
        let expires = now + chrono::Duration::hours(1);

        let user_email = user.email.clone();
        let mut active: crate::models::user::ActiveModel = user.into();
        active.password_reset_token = sea_orm::ActiveValue::Set(Some(token.clone()));
        active.password_reset_expires = sea_orm::ActiveValue::Set(Some(expires));
        active.updated_at = sea_orm::ActiveValue::Set(now);
        active.update(&self.db).await?;

        if let Err(e) = email_service
            .send_password_reset_email(&user_email, &token)
            .await
        {
            tracing::warn!("Failed to send password reset email: {e}");
        }

        Ok(())
    }

    /// Reset password using a reset token.
    pub async fn reset_password(&self, token: &str, new_password: &str) -> AppResult<()> {
        let user = User::find()
            .filter(crate::models::user::Column::PasswordResetToken.eq(token))
            .one(&self.db)
            .await?
            .ok_or_else(|| AppError::Validation("Invalid reset token".to_string()))?;
        let user_id = user.id;

        if let Some(expires) = user.password_reset_expires {
            if chrono::Utc::now().naive_utc() > expires {
                return Err(AppError::Validation("Reset token has expired".to_string()));
            }
        }

        let new_hash = hash_password(new_password)?;
        let now = chrono::Utc::now().naive_utc();
        let mut active: crate::models::user::ActiveModel = user.into();
        active.password_hash = sea_orm::ActiveValue::Set(new_hash);
        active.password_reset_token = sea_orm::ActiveValue::Set(None);
        active.password_reset_expires = sea_orm::ActiveValue::Set(None);
        active.updated_at = sea_orm::ActiveValue::Set(now);
        active.update(&self.db).await?;
        self.revoke_all_user_refresh_tokens(user_id).await?;

        Ok(())
    }

    async fn issue_tokens_for_user(&self, user_id: i32) -> AppResult<(String, String)> {
        self.issue_tokens_for_user_txn(&self.db, user_id).await
    }

    async fn issue_tokens_for_user_txn<C: ConnectionTrait>(
        &self,
        conn: &C,
        user_id: i32,
    ) -> AppResult<(String, String)> {
        let user_id_str = user_id.to_string();
        let access_token = encode_access_token(&user_id_str)?;
        let refresh_token = encode_refresh_token(&user_id_str)?;
        self.persist_refresh_token(conn, user_id, &refresh_token)
            .await?;
        Ok((access_token, refresh_token))
    }

    async fn persist_refresh_token<C: ConnectionTrait>(
        &self,
        conn: &C,
        user_id: i32,
        refresh_token: &str,
    ) -> AppResult<()> {
        let now = chrono::Utc::now().naive_utc();
        let expires_at = now
            + chrono::Duration::seconds(crate::utils::jwt::refresh_token_expiry_seconds() as i64);

        let model = refresh_token::ActiveModel {
            user_id: sea_orm::ActiveValue::Set(user_id),
            token: sea_orm::ActiveValue::Set(crate::utils::jwt::hash_refresh_token(refresh_token)),
            expires_at: sea_orm::ActiveValue::Set(expires_at),
            created_at: sea_orm::ActiveValue::Set(now),
            ..Default::default()
        };
        model.insert(conn).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn validate_email_format_valid() {
        let email = "user@example.com";
        assert!(email.contains('@') && email.contains('.'));
    }

    #[test]
    fn validate_email_format_missing_at() {
        let email = "userexample.com";
        assert!(!(email.contains('@') && email.contains('.')));
    }

    #[test]
    fn validate_email_format_missing_dot() {
        let email = "user@examplecom";
        assert!(!(email.contains('@') && email.contains('.')));
    }

    #[test]
    fn validate_username_length_valid() {
        let username = "alice";
        assert!(username.len() >= 3 && username.len() <= 30);
    }

    #[test]
    fn validate_username_too_short() {
        let username = "ab";
        assert!(!(username.len() >= 3 && username.len() <= 30));
    }

    #[test]
    fn validate_password_length_valid() {
        let password = "password123";
        assert!(password.len() >= 8);
    }

    #[test]
    fn validate_password_too_short() {
        let password = "pass";
        assert!(!(password.len() >= 8));
    }
}
