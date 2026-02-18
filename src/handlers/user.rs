use crate::error::{AppError, AppResult};
use crate::middleware::auth::parse_user_id;
use crate::middleware::AuthUser;
use crate::models::UserModel;
use crate::response::ApiResponse;
use crate::services::user::UserService;
use axum::{extract::Path, response::IntoResponse, Extension, Json};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Serialize, ToSchema)]
pub struct UserProfileResponse {
    pub id: i32,
    pub username: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub karma: i32,
    pub created_at: String,
}

impl From<UserModel> for UserProfileResponse {
    fn from(u: UserModel) -> Self {
        Self {
            id: u.id,
            username: u.username,
            avatar_url: u.avatar_url,
            bio: u.bio,
            karma: u.karma,
            created_at: u.created_at.to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateProfileRequest {
    #[validate(length(max = 500))]
    pub bio: Option<String>,
    #[validate(length(max = 500))]
    pub avatar_url: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/users/{username}",
    params(("username" = String, Path, description = "Username")),
    responses(
        (status = 200, description = "User profile", body = UserProfileResponse),
        (status = 404, description = "User not found", body = AppError),
    ),
    tag = "users"
)]
pub async fn get_user_profile(
    Extension(db): Extension<DatabaseConnection>,
    Path(username): Path<String>,
) -> AppResult<impl IntoResponse> {
    let service = UserService::new(db);
    let user = service.get_by_username(&username).await?;
    Ok(ApiResponse::ok(UserProfileResponse::from(user)))
}

#[utoipa::path(
    put,
    path = "/api/v1/auth/profile",
    security(("jwt_token" = [])),
    request_body = UpdateProfileRequest,
    responses(
        (status = 200, description = "Profile updated", body = UserProfileResponse),
        (status = 400, description = "Validation error", body = AppError),
        (status = 401, description = "Unauthorized", body = AppError),
    ),
    tag = "users"
)]
pub async fn update_profile(
    Extension(db): Extension<DatabaseConnection>,
    auth_user: AuthUser,
    Json(payload): Json<UpdateProfileRequest>,
) -> AppResult<impl IntoResponse> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let user_id = parse_user_id(&auth_user)?;

    let service = UserService::new(db);
    let user = service
        .update_profile(user_id, payload.bio, payload.avatar_url)
        .await?;

    Ok(ApiResponse::ok(UserProfileResponse::from(user)))
}
