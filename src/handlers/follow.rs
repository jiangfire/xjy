use crate::error::AppResult;
use crate::handlers::user::UserProfileResponse;
use crate::middleware::auth::parse_user_id;
use crate::middleware::AuthUser;
use crate::response::{ApiResponse, PaginatedResponse, PaginationQuery};
use crate::services::follow::FollowService;
use axum::{extract::Path, extract::Query, response::IntoResponse, Extension};
use sea_orm::DatabaseConnection;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct FollowToggleResponse {
    /// Whether user is now being followed
    pub following: bool,
}

#[utoipa::path(
    post,
    path = "/api/v1/users/{id}/follow",
    security(("jwt_token" = [])),
    params(("id" = i32, Path, description = "User ID to follow/unfollow")),
    responses(
        (status = 200, description = "Follow toggled", body = FollowToggleResponse),
        (status = 401, description = "Unauthorized", body = crate::error::AppError),
    ),
    tag = "follows"
)]
pub async fn toggle_follow(
    Extension(db): Extension<DatabaseConnection>,
    auth_user: AuthUser,
    Path(user_id): Path<i32>,
) -> AppResult<impl IntoResponse> {
    let follower_id = parse_user_id(&auth_user)?;
    let service = FollowService::new(db);
    let following = service.toggle(follower_id, user_id).await?;
    Ok(ApiResponse::ok(FollowToggleResponse { following }))
}

#[utoipa::path(
    get,
    path = "/api/v1/users/{id}/followers",
    params(
        ("id" = i32, Path, description = "User ID"),
        ("page" = Option<u64>, Query, description = "Page number"),
        ("per_page" = Option<u64>, Query, description = "Items per page"),
    ),
    responses(
        (status = 200, description = "List of followers", body = PaginatedResponse<UserProfileResponse>),
    ),
    tag = "follows"
)]
pub async fn list_followers(
    Extension(db): Extension<DatabaseConnection>,
    Path(user_id): Path<i32>,
    Query(params): Query<PaginationQuery>,
) -> AppResult<impl IntoResponse> {
    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(20).min(100);

    let service = FollowService::new(db);
    let (users, total) = service.list_followers(user_id, page, per_page).await?;
    let items = users.into_iter().map(UserProfileResponse::from).collect();
    Ok(ApiResponse::ok(PaginatedResponse::new(
        items, total, page, per_page,
    )))
}

#[utoipa::path(
    get,
    path = "/api/v1/users/{id}/following",
    params(
        ("id" = i32, Path, description = "User ID"),
        ("page" = Option<u64>, Query, description = "Page number"),
        ("per_page" = Option<u64>, Query, description = "Items per page"),
    ),
    responses(
        (status = 200, description = "List of following", body = PaginatedResponse<UserProfileResponse>),
    ),
    tag = "follows"
)]
pub async fn list_following(
    Extension(db): Extension<DatabaseConnection>,
    Path(user_id): Path<i32>,
    Query(params): Query<PaginationQuery>,
) -> AppResult<impl IntoResponse> {
    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(20).min(100);

    let service = FollowService::new(db);
    let (users, total) = service.list_following(user_id, page, per_page).await?;
    let items = users.into_iter().map(UserProfileResponse::from).collect();
    Ok(ApiResponse::ok(PaginatedResponse::new(
        items, total, page, per_page,
    )))
}
