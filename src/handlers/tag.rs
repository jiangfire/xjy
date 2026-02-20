use crate::error::AppResult;
use crate::handlers::post::PostResponse;
use crate::middleware::auth::require_admin;
use crate::middleware::AuthUser;
use crate::models::TagModel;
use crate::response::{ApiResponse, PaginatedResponse};
use crate::services::tag::TagService;
use axum::{extract::Path, extract::Query, response::IntoResponse, Extension, Json};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Serialize, ToSchema)]
pub struct TagResponse {
    /// Tag ID
    pub id: i32,
    /// Tag name
    pub name: String,
    /// URL slug
    pub slug: String,
}

impl From<TagModel> for TagResponse {
    fn from(t: TagModel) -> Self {
        Self {
            id: t.id,
            name: t.name,
            slug: t.slug,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TagPostsQuery {
    /// Page number
    pub page: Option<u64>,
    /// Items per page
    pub per_page: Option<u64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/tags",
    responses(
        (status = 200, description = "List all tags", body = Vec<TagResponse>),
    ),
    tag = "tags"
)]
pub async fn list_tags(
    Extension(db): Extension<DatabaseConnection>,
) -> AppResult<impl IntoResponse> {
    let service = TagService::new(db);
    let tags = service.list_tags().await?;
    let items: Vec<TagResponse> = tags.into_iter().map(TagResponse::from).collect();
    Ok(ApiResponse::ok(items))
}

#[utoipa::path(
    get,
    path = "/api/v1/tags/{slug}/posts",
    params(
        ("slug" = String, Path, description = "Tag slug"),
        ("page" = Option<u64>, Query, description = "Page number"),
        ("per_page" = Option<u64>, Query, description = "Items per page"),
    ),
    responses(
        (status = 200, description = "Posts with this tag", body = PaginatedResponse<PostResponse>),
        (status = 404, description = "Tag not found", body = crate::error::AppError),
    ),
    tag = "tags"
)]
pub async fn get_posts_by_tag(
    Extension(db): Extension<DatabaseConnection>,
    Path(slug): Path<String>,
    Query(params): Query<TagPostsQuery>,
) -> AppResult<impl IntoResponse> {
    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(20).min(100);

    let service = TagService::new(db);
    let (posts, total) = service.get_posts_by_tag(&slug, page, per_page).await?;
    let items: Vec<PostResponse> = posts.into_iter().map(PostResponse::from).collect();

    Ok(ApiResponse::ok(PaginatedResponse::new(
        items, total, page, per_page,
    )))
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateTagRequest {
    /// Tag name (1-30 characters)
    #[validate(length(min = 1, max = 30))]
    pub name: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/admin/tags",
    security(("jwt_token" = [])),
    request_body = CreateTagRequest,
    responses(
        (status = 200, description = "Tag created", body = TagResponse),
        (status = 400, description = "Validation error", body = crate::error::AppError),
        (status = 403, description = "Admin only", body = crate::error::AppError),
    ),
    tag = "tags"
)]
pub async fn create_tag(
    Extension(db): Extension<DatabaseConnection>,
    auth_user: AuthUser,
    Json(payload): Json<CreateTagRequest>,
) -> AppResult<impl IntoResponse> {
    payload
        .validate()
        .map_err(|e| crate::error::AppError::Validation(e.to_string()))?;
    require_admin(&db, &auth_user).await?;

    let service = TagService::new(db);
    let tag = service.create_tag(&payload.name).await?;
    Ok(ApiResponse::ok(TagResponse::from(tag)))
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateTagRequest {
    /// Tag name (1-30 characters)
    #[validate(length(min = 1, max = 30))]
    pub name: String,
}

#[utoipa::path(
    put,
    path = "/api/v1/admin/tags/{id}",
    security(("jwt_token" = [])),
    params(("id" = i32, Path, description = "Tag ID")),
    request_body = UpdateTagRequest,
    responses(
        (status = 200, description = "Tag updated", body = TagResponse),
        (status = 400, description = "Validation error", body = crate::error::AppError),
        (status = 403, description = "Admin only", body = crate::error::AppError),
    ),
    tag = "tags"
)]
pub async fn update_tag(
    Extension(db): Extension<DatabaseConnection>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateTagRequest>,
) -> AppResult<impl IntoResponse> {
    payload
        .validate()
        .map_err(|e| crate::error::AppError::Validation(e.to_string()))?;
    require_admin(&db, &auth_user).await?;

    let service = TagService::new(db);
    let tag = service.update_tag(id, &payload.name).await?;
    Ok(ApiResponse::ok(TagResponse::from(tag)))
}

#[utoipa::path(
    delete,
    path = "/api/v1/admin/tags/{id}",
    security(("jwt_token" = [])),
    params(("id" = i32, Path, description = "Tag ID")),
    responses(
        (status = 200, description = "Tag deleted", body = String),
        (status = 403, description = "Admin only", body = crate::error::AppError),
    ),
    tag = "tags"
)]
pub async fn delete_tag(
    Extension(db): Extension<DatabaseConnection>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> AppResult<impl IntoResponse> {
    require_admin(&db, &auth_user).await?;

    let service = TagService::new(db);
    service.delete_tag(id).await?;
    Ok(ApiResponse::ok("Tag deleted successfully"))
}
