use crate::error::{AppError, AppResult};
use crate::middleware::auth::{require_admin, AuthUser};
use crate::models::UserModel;
use crate::response::{ApiResponse, PaginatedResponse, PaginationQuery};
use crate::services::admin::AdminService;
use axum::{extract::Path, extract::Query, response::IntoResponse, Extension, Json};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateRoleRequest {
    #[validate(length(min = 1, max = 20))]
    pub role: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StatsResponse {
    pub total_users: u64,
    pub total_posts: u64,
    pub total_comments: u64,
    pub total_forums: u64,
    pub users_today: u64,
    pub posts_today: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminUserResponse {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub karma: i32,
    pub role: String,
    pub created_at: String,
}

impl From<UserModel> for AdminUserResponse {
    fn from(u: UserModel) -> Self {
        Self {
            id: u.id,
            username: u.username,
            email: u.email,
            avatar_url: u.avatar_url,
            bio: u.bio,
            karma: u.karma,
            role: u.role,
            created_at: u.created_at.to_string(),
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/stats",
    security(("jwt_token" = [])),
    responses(
        (status = 200, description = "Platform statistics", body = StatsResponse),
        (status = 403, description = "Admin only", body = AppError),
    ),
    tag = "admin"
)]
pub async fn get_stats(
    Extension(db): Extension<DatabaseConnection>,
    auth_user: AuthUser,
) -> AppResult<impl IntoResponse> {
    require_admin(&db, &auth_user).await?;

    let service = AdminService::new(db);
    let stats = service.get_stats().await?;

    Ok(ApiResponse::ok(StatsResponse {
        total_users: stats.total_users,
        total_posts: stats.total_posts,
        total_comments: stats.total_comments,
        total_forums: stats.total_forums,
        users_today: stats.users_today,
        posts_today: stats.posts_today,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/users",
    security(("jwt_token" = [])),
    params(
        ("page" = Option<u64>, Query, description = "Page number"),
        ("per_page" = Option<u64>, Query, description = "Items per page"),
    ),
    responses(
        (status = 200, description = "List of users", body = PaginatedResponse<AdminUserResponse>),
        (status = 403, description = "Admin only", body = AppError),
    ),
    tag = "admin"
)]
pub async fn list_users(
    Extension(db): Extension<DatabaseConnection>,
    auth_user: AuthUser,
    Query(params): Query<PaginationQuery>,
) -> AppResult<impl IntoResponse> {
    require_admin(&db, &auth_user).await?;

    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(20).min(100);

    let service = AdminService::new(db);
    let (users, total) = service.list_users(page, per_page).await?;
    let items = users.into_iter().map(AdminUserResponse::from).collect();

    Ok(ApiResponse::ok(PaginatedResponse::new(
        items, total, page, per_page,
    )))
}

#[utoipa::path(
    put,
    path = "/api/v1/admin/users/{id}/role",
    security(("jwt_token" = [])),
    params(("id" = i32, Path, description = "User ID")),
    request_body = UpdateRoleRequest,
    responses(
        (status = 200, description = "User role updated", body = AdminUserResponse),
        (status = 400, description = "Validation error", body = AppError),
        (status = 403, description = "Admin only", body = AppError),
    ),
    tag = "admin"
)]
pub async fn update_user_role(
    Extension(db): Extension<DatabaseConnection>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateRoleRequest>,
) -> AppResult<impl IntoResponse> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    require_admin(&db, &auth_user).await?;

    let service = AdminService::new(db);
    let user = service.update_user_role(id, &payload.role).await?;

    Ok(ApiResponse::ok(AdminUserResponse::from(user)))
}

#[utoipa::path(
    delete,
    path = "/api/v1/admin/posts/{id}",
    security(("jwt_token" = [])),
    params(("id" = i32, Path, description = "Post ID")),
    responses(
        (status = 200, description = "Post deleted by admin", body = String),
        (status = 403, description = "Admin only", body = AppError),
        (status = 404, description = "Post not found", body = AppError),
    ),
    tag = "admin"
)]
pub async fn admin_delete_post(
    Extension(db): Extension<DatabaseConnection>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> AppResult<impl IntoResponse> {
    require_admin(&db, &auth_user).await?;

    let service = AdminService::new(db);
    service.admin_delete_post(id).await?;

    Ok(ApiResponse::ok("Post deleted by admin"))
}

#[utoipa::path(
    delete,
    path = "/api/v1/admin/comments/{id}",
    security(("jwt_token" = [])),
    params(("id" = i32, Path, description = "Comment ID")),
    responses(
        (status = 200, description = "Comment deleted by admin", body = String),
        (status = 403, description = "Admin only", body = AppError),
        (status = 404, description = "Comment not found", body = AppError),
    ),
    tag = "admin"
)]
pub async fn admin_delete_comment(
    Extension(db): Extension<DatabaseConnection>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> AppResult<impl IntoResponse> {
    require_admin(&db, &auth_user).await?;

    let service = AdminService::new(db);
    service.admin_delete_comment(id).await?;

    Ok(ApiResponse::ok("Comment deleted by admin"))
}
