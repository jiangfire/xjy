use crate::error::{AppError, AppResult};
use crate::middleware::auth::parse_user_id;
use crate::middleware::AuthUser;
use crate::response::ApiResponse;
use crate::services::upload::{UploadConfig, UploadService};
use crate::services::user::UserService;
use axum::{extract::Multipart, response::IntoResponse, Extension};
use sea_orm::DatabaseConnection;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct UploadResponse {
    /// URL of the uploaded file
    pub url: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/upload/avatar",
    security(("jwt_token" = [])),
    responses(
        (status = 200, description = "Avatar uploaded", body = UploadResponse),
        (status = 400, description = "Invalid file", body = AppError),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 413, description = "File too large", body = AppError),
    ),
    tag = "uploads"
)]
pub async fn upload_avatar(
    Extension(db): Extension<DatabaseConnection>,
    Extension(config): Extension<UploadConfig>,
    auth_user: AuthUser,
    mut multipart: Multipart,
) -> AppResult<impl IntoResponse> {
    let user_id = parse_user_id(&auth_user)?;

    let field = multipart
        .next_field()
        .await
        .map_err(|e| AppError::Validation(format!("Failed to read upload: {}", e)))?
        .ok_or_else(|| AppError::Validation("No file provided".to_string()))?;

    let content_type = field
        .content_type()
        .unwrap_or("application/octet-stream")
        .to_string();

    let data = field
        .bytes()
        .await
        .map_err(|e| AppError::Validation(format!("Failed to read file data: {}", e)))?;

    let url = UploadService::save_file(&config, &data, &content_type, "avatars").await?;

    // Update user avatar_url
    let service = UserService::new(db);
    service.update_avatar_url(user_id, &url).await?;

    Ok(ApiResponse::ok(UploadResponse { url }))
}

#[utoipa::path(
    post,
    path = "/api/v1/upload/image",
    security(("jwt_token" = [])),
    responses(
        (status = 200, description = "Image uploaded", body = UploadResponse),
        (status = 400, description = "Invalid file", body = AppError),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 413, description = "File too large", body = AppError),
    ),
    tag = "uploads"
)]
pub async fn upload_image(
    Extension(config): Extension<UploadConfig>,
    _auth_user: AuthUser,
    mut multipart: Multipart,
) -> AppResult<impl IntoResponse> {
    let field = multipart
        .next_field()
        .await
        .map_err(|e| AppError::Validation(format!("Failed to read upload: {}", e)))?
        .ok_or_else(|| AppError::Validation("No file provided".to_string()))?;

    let content_type = field
        .content_type()
        .unwrap_or("application/octet-stream")
        .to_string();

    let data = field
        .bytes()
        .await
        .map_err(|e| AppError::Validation(format!("Failed to read file data: {}", e)))?;

    let url = UploadService::save_file(&config, &data, &content_type, "images").await?;

    Ok(ApiResponse::ok(UploadResponse { url }))
}
