use crate::error::{AppError, AppResult};
use crate::middleware::auth::{require_admin, AuthUser};
use crate::models::ForumModel;
use crate::response::ApiResponse;
use crate::services::cache::CacheService;
use crate::services::forum::ForumService;
use axum::{extract::Path, response::IntoResponse, Extension, Json};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateForumRequest {
    /// Forum name (1-100 characters)
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    /// Forum description (max 500 characters)
    #[validate(length(max = 500))]
    pub description: String,
    /// URL slug (1-100 characters)
    #[validate(length(min = 1, max = 100))]
    pub slug: String,
    /// Display sort order
    pub sort_order: Option<i32>,
    /// Icon URL
    pub icon_url: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateForumRequest {
    /// Forum name (1-100 characters)
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    /// Forum description (max 500 characters)
    #[validate(length(max = 500))]
    pub description: String,
    /// Display sort order
    pub sort_order: Option<i32>,
    /// Icon URL
    pub icon_url: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ForumResponse {
    /// Forum ID
    pub id: i32,
    /// Forum name
    pub name: String,
    /// Forum description
    pub description: String,
    /// URL slug
    pub slug: String,
    /// Display sort order
    pub sort_order: i32,
    /// Icon URL
    pub icon_url: Option<String>,
    /// Creation timestamp
    pub created_at: String,
    /// Last update timestamp
    pub updated_at: String,
}

impl From<ForumModel> for ForumResponse {
    fn from(f: ForumModel) -> Self {
        Self {
            id: f.id,
            name: f.name,
            description: f.description,
            slug: f.slug,
            sort_order: f.sort_order,
            icon_url: f.icon_url,
            created_at: f.created_at.to_string(),
            updated_at: f.updated_at.to_string(),
        }
    }
}

fn make_forum_service(db: DatabaseConnection, cache: Option<CacheService>) -> ForumService {
    let service = ForumService::new(db);
    match cache {
        Some(c) => service.with_cache(c),
        None => service,
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/forums",
    responses(
        (status = 200, description = "List all forums", body = Vec<ForumResponse>),
    ),
    tag = "forums"
)]
pub async fn list_forums(
    Extension(db): Extension<DatabaseConnection>,
    cache: Option<Extension<CacheService>>,
) -> AppResult<impl IntoResponse> {
    let service = make_forum_service(db, cache.map(|c| c.0));
    let forums = service.list().await?;
    let response: Vec<ForumResponse> = forums.into_iter().map(ForumResponse::from).collect();
    Ok(ApiResponse::ok(response))
}

#[utoipa::path(
    get,
    path = "/api/v1/forums/{slug}",
    params(("slug" = String, Path, description = "Forum slug")),
    responses(
        (status = 200, description = "Forum details", body = ForumResponse),
        (status = 404, description = "Forum not found", body = AppError),
    ),
    tag = "forums"
)]
pub async fn get_forum(
    Extension(db): Extension<DatabaseConnection>,
    Path(slug): Path<String>,
) -> AppResult<impl IntoResponse> {
    let service = ForumService::new(db);
    let forum = service.get_by_slug(&slug).await?;
    Ok(ApiResponse::ok(ForumResponse::from(forum)))
}

#[utoipa::path(
    post,
    path = "/api/v1/forums",
    security(("jwt_token" = [])),
    request_body = CreateForumRequest,
    responses(
        (status = 200, description = "Forum created", body = ForumResponse),
        (status = 400, description = "Validation error", body = AppError),
        (status = 403, description = "Admin only", body = AppError),
    ),
    tag = "forums"
)]
pub async fn create_forum(
    Extension(db): Extension<DatabaseConnection>,
    cache: Option<Extension<CacheService>>,
    auth_user: AuthUser,
    Json(payload): Json<CreateForumRequest>,
) -> AppResult<impl IntoResponse> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    require_admin(&db, &auth_user).await?;

    let service = make_forum_service(db, cache.map(|c| c.0));
    let forum = service
        .create(
            &payload.name,
            &payload.description,
            &payload.slug,
            payload.sort_order.unwrap_or(0),
            payload.icon_url,
        )
        .await?;

    Ok(ApiResponse::ok(ForumResponse::from(forum)))
}

#[utoipa::path(
    put,
    path = "/api/v1/forums/{slug}",
    security(("jwt_token" = [])),
    params(("slug" = String, Path, description = "Forum slug")),
    request_body = UpdateForumRequest,
    responses(
        (status = 200, description = "Forum updated", body = ForumResponse),
        (status = 400, description = "Validation error", body = AppError),
        (status = 403, description = "Admin only", body = AppError),
    ),
    tag = "forums"
)]
pub async fn update_forum(
    Extension(db): Extension<DatabaseConnection>,
    cache: Option<Extension<CacheService>>,
    auth_user: AuthUser,
    Path(slug): Path<String>,
    Json(payload): Json<UpdateForumRequest>,
) -> AppResult<impl IntoResponse> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    require_admin(&db, &auth_user).await?;

    let service = make_forum_service(db, cache.map(|c| c.0));
    let forum = service
        .update(
            &slug,
            &payload.name,
            &payload.description,
            payload.sort_order.unwrap_or(0),
            payload.icon_url,
        )
        .await?;

    Ok(ApiResponse::ok(ForumResponse::from(forum)))
}

#[utoipa::path(
    delete,
    path = "/api/v1/forums/{slug}",
    security(("jwt_token" = [])),
    params(("slug" = String, Path, description = "Forum slug")),
    responses(
        (status = 200, description = "Forum deleted", body = String),
        (status = 403, description = "Admin only", body = AppError),
    ),
    tag = "forums"
)]
pub async fn delete_forum(
    Extension(db): Extension<DatabaseConnection>,
    cache: Option<Extension<CacheService>>,
    auth_user: AuthUser,
    Path(slug): Path<String>,
) -> AppResult<impl IntoResponse> {
    require_admin(&db, &auth_user).await?;

    let service = make_forum_service(db, cache.map(|c| c.0));
    service.delete(&slug).await?;

    Ok(ApiResponse::ok("Forum deleted"))
}
