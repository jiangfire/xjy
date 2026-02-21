use crate::error::AppResult;
use crate::handlers::post::PostResponse;
use crate::middleware::auth::parse_user_id;
use crate::middleware::AuthUser;
use crate::response::{ApiResponse, PaginatedResponse, PaginationQuery};
use crate::services::bookmark::BookmarkService;
use axum::{extract::Path, extract::Query, response::IntoResponse, Extension};
use sea_orm::DatabaseConnection;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct BookmarkToggleResponse {
    /// Whether post is now bookmarked
    pub bookmarked: bool,
}

#[utoipa::path(
    put,
    path = "/api/v1/posts/{id}/bookmark",
    security(("jwt_token" = [])),
    params(("id" = i32, Path, description = "Post ID")),
    responses(
        (status = 200, description = "Bookmarked", body = BookmarkToggleResponse),
        (status = 401, description = "Unauthorized", body = crate::error::AppError),
    ),
    tag = "bookmarks"
)]
pub async fn add_bookmark(
    Extension(db): Extension<DatabaseConnection>,
    auth_user: AuthUser,
    Path(post_id): Path<i32>,
) -> AppResult<impl IntoResponse> {
    let user_id = parse_user_id(&auth_user)?;
    let service = BookmarkService::new(db);
    let bookmarked = service.add_bookmark(user_id, post_id).await?;
    Ok(ApiResponse::ok(BookmarkToggleResponse { bookmarked }))
}

#[utoipa::path(
    delete,
    path = "/api/v1/posts/{id}/bookmark",
    security(("jwt_token" = [])),
    params(("id" = i32, Path, description = "Post ID")),
    responses(
        (status = 200, description = "Bookmark removed", body = BookmarkToggleResponse),
        (status = 401, description = "Unauthorized", body = crate::error::AppError),
    ),
    tag = "bookmarks"
)]
pub async fn remove_bookmark(
    Extension(db): Extension<DatabaseConnection>,
    auth_user: AuthUser,
    Path(post_id): Path<i32>,
) -> AppResult<impl IntoResponse> {
    let user_id = parse_user_id(&auth_user)?;
    let service = BookmarkService::new(db);
    let bookmarked = service.remove_bookmark(user_id, post_id).await?;
    Ok(ApiResponse::ok(BookmarkToggleResponse { bookmarked }))
}

#[utoipa::path(
    post,
    path = "/api/v1/posts/{id}/bookmark",
    security(("jwt_token" = [])),
    params(("id" = i32, Path, description = "Post ID")),
    responses(
        (status = 200, description = "Bookmark toggled", body = BookmarkToggleResponse),
        (status = 401, description = "Unauthorized", body = crate::error::AppError),
    ),
    tag = "bookmarks"
)]
pub async fn toggle_bookmark(
    Extension(db): Extension<DatabaseConnection>,
    auth_user: AuthUser,
    Path(post_id): Path<i32>,
) -> AppResult<impl IntoResponse> {
    let user_id = parse_user_id(&auth_user)?;
    let service = BookmarkService::new(db);
    let bookmarked = service.toggle(user_id, post_id).await?;
    Ok(ApiResponse::ok(BookmarkToggleResponse { bookmarked }))
}

#[utoipa::path(
    get,
    path = "/api/v1/bookmarks",
    security(("jwt_token" = [])),
    params(
        ("page" = Option<u64>, Query, description = "Page number"),
        ("per_page" = Option<u64>, Query, description = "Items per page"),
    ),
    responses(
        (status = 200, description = "Bookmarked posts", body = PaginatedResponse<PostResponse>),
        (status = 401, description = "Unauthorized", body = crate::error::AppError),
    ),
    tag = "bookmarks"
)]
pub async fn list_bookmarks(
    Extension(db): Extension<DatabaseConnection>,
    auth_user: AuthUser,
    Query(params): Query<PaginationQuery>,
) -> AppResult<impl IntoResponse> {
    let user_id = parse_user_id(&auth_user)?;
    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(20).min(100);

    let service = BookmarkService::new(db);
    let (posts, total) = service.list_user_bookmarks(user_id, page, per_page).await?;
    let items = posts.into_iter().map(PostResponse::from).collect();
    Ok(ApiResponse::ok(PaginatedResponse::new(
        items, total, page, per_page,
    )))
}
