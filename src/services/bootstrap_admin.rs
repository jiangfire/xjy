use crate::error::AppResult;
use crate::models::User;
use crate::utils::hash_password;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::env;

#[derive(Debug, Clone)]
pub struct BootstrapAdminConfig {
    pub username: String,
    pub email: String,
    pub password: String,
}

impl BootstrapAdminConfig {
    pub fn from_env() -> Option<Self> {
        let enabled = env::var("BOOTSTRAP_ADMIN_ENABLED")
            .ok()
            .map(|v| v.trim().to_ascii_lowercase())
            .map(|v| matches!(v.as_str(), "1" | "true" | "yes" | "y" | "on"))
            .unwrap_or(false);

        if !enabled {
            return None;
        }

        Some(Self {
            username: env::var("BOOTSTRAP_ADMIN_USERNAME").ok()?,
            email: env::var("BOOTSTRAP_ADMIN_EMAIL").ok()?,
            password: env::var("BOOTSTRAP_ADMIN_PASSWORD").ok()?,
        })
    }
}

/// 启动时自动创建/提升管理员：
/// - 若库中已存在任意 admin：不做任何事
/// - 否则若配置的 email/username 已存在：提升为 admin
/// - 否则创建一个新的 admin（email_verified=true）
pub async fn ensure_bootstrap_admin(db: &DatabaseConnection) -> AppResult<()> {
    let Some(cfg) = BootstrapAdminConfig::from_env() else {
        return Ok(());
    };

    let admin_exists = User::find()
        .filter(crate::models::user::Column::Role.eq("admin"))
        .one(db)
        .await?
        .is_some();
    if admin_exists {
        return Ok(());
    }

    let existing = User::find()
        .filter(
            sea_orm::Condition::any()
                .add(crate::models::user::Column::Email.eq(cfg.email.clone()))
                .add(crate::models::user::Column::Username.eq(cfg.username.clone())),
        )
        .one(db)
        .await?;

    let now = chrono::Utc::now().naive_utc();

    if let Some(user) = existing {
        let mut active: crate::models::user::ActiveModel = user.into();
        active.role = sea_orm::ActiveValue::Set("admin".to_string());
        active.updated_at = sea_orm::ActiveValue::Set(now);
        active.update(db).await?;
        return Ok(());
    }

    let password_hash = hash_password(&cfg.password)?;

    let new_user = crate::models::user::ActiveModel {
        username: sea_orm::ActiveValue::Set(cfg.username),
        email: sea_orm::ActiveValue::Set(cfg.email),
        password_hash: sea_orm::ActiveValue::Set(password_hash),
        karma: sea_orm::ActiveValue::Set(0),
        role: sea_orm::ActiveValue::Set("admin".to_string()),
        email_verified: sea_orm::ActiveValue::Set(true),
        email_verification_token: sea_orm::ActiveValue::Set(None),
        email_verification_expires: sea_orm::ActiveValue::Set(None),
        created_at: sea_orm::ActiveValue::Set(now),
        updated_at: sea_orm::ActiveValue::Set(now),
        ..Default::default()
    };

    new_user.insert(db).await?;
    Ok(())
}

