use crate::error::AppResult;
use crate::handlers::post::PostResponse;
use crate::middleware::auth::parse_user_id;
use crate::middleware::AuthUser;
use crate::response::{ApiResponse, PaginatedResponse, PaginationQuery};
use crate::services::bookmark::BookmarkService;
use axum::{extract::Path, extract::Query, response::IntoResponse, Extension};
use sea_orm::DatabaseConnection;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct BookmarkToggleResponse {
    pub bookmarked: bool,
}

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
