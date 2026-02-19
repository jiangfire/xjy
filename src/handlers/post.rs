use crate::error::{AppError, AppResult};
use crate::middleware::auth::{parse_user_id, require_admin, AuthUser};
use crate::models::PostModel;
use crate::response::{ApiResponse, PaginatedResponse};
use crate::services::post::PostService;
use crate::services::tag::TagService;
use crate::utils::render_markdown;
use axum::{extract::Path, extract::Query, response::IntoResponse, Extension, Json};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreatePostRequest {
    /// Forum ID
    pub forum_id: i32,
    /// Post title (1-200 characters)
    #[validate(length(min = 1, max = 200))]
    pub title: String,
    /// Post content (Markdown supported)
    #[validate(length(min = 1))]
    pub content: String,
    /// Tags (up to 5 tags, each max 30 characters)
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdatePostRequest {
    /// Post title (1-200 characters)
    #[validate(length(min = 1, max = 200))]
    pub title: String,
    /// Post content (Markdown supported)
    #[validate(length(min = 1))]
    pub content: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PostResponse {
    /// Post ID
    pub id: i32,
    /// Author user ID
    pub user_id: i32,
    /// Forum ID
    pub forum_id: i32,
    /// Post title
    pub title: String,
    /// Post content (Markdown)
    pub content: String,
    /// Rendered HTML content
    pub content_html: String,
    /// Upvote count
    pub upvotes: i32,
    /// Downvote count
    pub downvotes: i32,
    /// View count
    pub view_count: i32,
    /// Whether post is pinned
    pub is_pinned: bool,
    /// Whether post is locked (no new comments)
    pub is_locked: bool,
    /// Creation timestamp
    pub created_at: String,
    /// Last update timestamp
    pub updated_at: String,
    /// Post tags
    pub tags: Vec<String>,
}

impl From<PostModel> for PostResponse {
    fn from(p: PostModel) -> Self {
        let content_html = render_markdown(&p.content);
        Self {
            id: p.id,
            user_id: p.user_id,
            forum_id: p.forum_id,
            title: p.title,
            content: p.content,
            content_html,
            upvotes: p.upvotes,
            downvotes: p.downvotes,
            view_count: p.view_count,
            is_pinned: p.is_pinned,
            is_locked: p.is_locked,
            created_at: p.created_at.to_string(),
            updated_at: p.updated_at.to_string(),
            tags: Vec::new(),
        }
    }
}

impl PostResponse {
    pub fn with_tags(p: PostModel, tags: Vec<String>) -> Self {
        let content_html = render_markdown(&p.content);
        Self {
            id: p.id,
            user_id: p.user_id,
            forum_id: p.forum_id,
            title: p.title,
            content: p.content,
            content_html,
            upvotes: p.upvotes,
            downvotes: p.downvotes,
            view_count: p.view_count,
            is_pinned: p.is_pinned,
            is_locked: p.is_locked,
            created_at: p.created_at.to_string(),
            updated_at: p.updated_at.to_string(),
            tags,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PostListQuery {
    /// Page number
    pub page: Option<u64>,
    /// Items per page
    pub per_page: Option<u64>,
    /// Sort order: new, top, hot
    pub sort: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/forums/{forum_id}/posts",
    params(
        ("forum_id" = i32, Path, description = "Forum ID"),
        ("page" = Option<u64>, Query, description = "Page number"),
        ("per_page" = Option<u64>, Query, description = "Items per page"),
        ("sort" = Option<String>, Query, description = "Sort order: new, top, hot"),
    ),
    responses(
        (status = 200, description = "List of posts", body = PaginatedResponse<PostResponse>),
    ),
    tag = "posts"
)]
pub async fn list_posts(
    Extension(db): Extension<DatabaseConnection>,
    Path(forum_id): Path<i32>,
    Query(params): Query<PostListQuery>,
) -> AppResult<impl IntoResponse> {
    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(20).min(100);
    let sort = params.sort.as_deref().unwrap_or("new");

    let service = PostService::new(db.clone());
    let (posts, total) = service.list_by_forum(forum_id, page, per_page, sort).await?;

    // Batch-fetch tags for all posts in the page
    let post_ids: Vec<i32> = posts.iter().map(|p| p.id).collect();
    let tag_service = TagService::new(db);
    let tags_map = tag_service.get_tags_for_posts(&post_ids).await?;

    let items: Vec<PostResponse> = posts
        .into_iter()
        .map(|p| {
            let tags = tags_map.get(&p.id).cloned().unwrap_or_default();
            PostResponse::with_tags(p, tags)
        })
        .collect();

    Ok(ApiResponse::ok(PaginatedResponse::new(
        items, total, page, per_page,
    )))
}

#[utoipa::path(
    get,
    path = "/api/v1/posts/{id}",
    params(("id" = i32, Path, description = "Post ID")),
    responses(
        (status = 200, description = "Post details", body = PostResponse),
        (status = 404, description = "Post not found", body = AppError),
    ),
    tag = "posts"
)]
pub async fn get_post(
    Extension(db): Extension<DatabaseConnection>,
    Path(id): Path<i32>,
) -> AppResult<impl IntoResponse> {
    let service = PostService::new(db.clone());
    service.increment_view_count(id).await?;
    let post = service.get_by_id(id).await?;

    let tag_service = TagService::new(db);
    let tags = tag_service.get_post_tags(id).await?;
    let tag_names: Vec<String> = tags.into_iter().map(|t| t.name).collect();

    Ok(ApiResponse::ok(PostResponse::with_tags(post, tag_names)))
}

#[utoipa::path(
    post,
    path = "/api/v1/posts",
    security(("jwt_token" = [])),
    request_body = CreatePostRequest,
    responses(
        (status = 200, description = "Post created", body = PostResponse),
        (status = 400, description = "Validation error", body = AppError),
        (status = 401, description = "Unauthorized", body = AppError),
    ),
    tag = "posts"
)]
pub async fn create_post(
    Extension(db): Extension<DatabaseConnection>,
    auth_user: AuthUser,
    Json(payload): Json<CreatePostRequest>,
) -> AppResult<impl IntoResponse> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // Validate tags
    let tag_names = payload.tags.unwrap_or_default();
    if tag_names.len() > 5 {
        return Err(AppError::Validation("Maximum 5 tags allowed".to_string()));
    }
    for tag in &tag_names {
        if tag.trim().is_empty() || tag.len() > 30 {
            return Err(AppError::Validation(
                "Each tag must be 1-30 characters".to_string(),
            ));
        }
    }

    let user_id = parse_user_id(&auth_user)?;

    let service = PostService::new(db.clone());
    let post = service
        .create(user_id, payload.forum_id, &payload.title, &payload.content)
        .await?;

    // Assign tags
    let mut response_tags = Vec::new();
    if !tag_names.is_empty() {
        let tag_service = TagService::new(db);
        let tags = tag_service.get_or_create_tags(tag_names).await?;
        response_tags = tags.iter().map(|t| t.name.clone()).collect();
        let tag_ids: Vec<i32> = tags.into_iter().map(|t| t.id).collect();
        tag_service.set_post_tags(post.id, tag_ids).await?;
    }

    Ok(ApiResponse::ok(PostResponse::with_tags(post, response_tags)))
}

#[utoipa::path(
    put,
    path = "/api/v1/posts/{id}",
    security(("jwt_token" = [])),
    params(("id" = i32, Path, description = "Post ID")),
    request_body = UpdatePostRequest,
    responses(
        (status = 200, description = "Post updated", body = PostResponse),
        (status = 400, description = "Validation error", body = AppError),
        (status = 401, description = "Unauthorized", body = AppError),
    ),
    tag = "posts"
)]
pub async fn update_post(
    Extension(db): Extension<DatabaseConnection>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
    Json(payload): Json<UpdatePostRequest>,
) -> AppResult<impl IntoResponse> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let user_id = parse_user_id(&auth_user)?;

    let service = PostService::new(db);
    let post = service
        .update(id, user_id, &payload.title, &payload.content)
        .await?;

    Ok(ApiResponse::ok(PostResponse::from(post)))
}

#[utoipa::path(
    delete,
    path = "/api/v1/posts/{id}",
    security(("jwt_token" = [])),
    params(("id" = i32, Path, description = "Post ID")),
    responses(
        (status = 200, description = "Post deleted", body = String),
        (status = 401, description = "Unauthorized", body = AppError),
    ),
    tag = "posts"
)]
pub async fn delete_post(
    Extension(db): Extension<DatabaseConnection>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> AppResult<impl IntoResponse> {
    let user_id = parse_user_id(&auth_user)?;

    let service = PostService::new(db);
    service.delete(id, user_id).await?;

    Ok(ApiResponse::ok("Post deleted"))
}

#[utoipa::path(
    put,
    path = "/api/v1/posts/{id}/pin",
    security(("jwt_token" = [])),
    params(("id" = i32, Path, description = "Post ID")),
    responses(
        (status = 200, description = "Post pin toggled", body = PostResponse),
        (status = 403, description = "Admin only", body = AppError),
    ),
    tag = "posts"
)]
pub async fn pin_post(
    Extension(db): Extension<DatabaseConnection>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> AppResult<impl IntoResponse> {
    require_admin(&db, &auth_user).await?;

    let service = PostService::new(db);
    let post = service.toggle_pin(id).await?;
    Ok(ApiResponse::ok(PostResponse::from(post)))
}

#[utoipa::path(
    put,
    path = "/api/v1/posts/{id}/lock",
    security(("jwt_token" = [])),
    params(("id" = i32, Path, description = "Post ID")),
    responses(
        (status = 200, description = "Post lock toggled", body = PostResponse),
        (status = 403, description = "Admin only", body = AppError),
    ),
    tag = "posts"
)]
pub async fn lock_post(
    Extension(db): Extension<DatabaseConnection>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> AppResult<impl IntoResponse> {
    require_admin(&db, &auth_user).await?;

    let service = PostService::new(db);
    let post = service.toggle_lock(id).await?;
    Ok(ApiResponse::ok(PostResponse::from(post)))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SearchPostsQuery {
    /// Search query
    pub q: String,
    /// Filter by forum ID
    pub forum_id: Option<i32>,
    /// Page number
    pub page: Option<u64>,
    /// Items per page
    pub per_page: Option<u64>,
    /// Sort order: relevance, new, top
    pub sort: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/search",
    params(
        ("q" = String, Query, description = "Search query"),
        ("forum_id" = Option<i32>, Query, description = "Filter by forum"),
        ("page" = Option<u64>, Query, description = "Page number"),
        ("per_page" = Option<u64>, Query, description = "Items per page"),
        ("sort" = Option<String>, Query, description = "Sort: relevance, new, top"),
    ),
    responses(
        (status = 200, description = "Search results", body = PaginatedResponse<PostResponse>),
        (status = 400, description = "Invalid query", body = AppError),
    ),
    tag = "posts"
)]
pub async fn search_posts(
    Extension(db): Extension<DatabaseConnection>,
    Query(params): Query<SearchPostsQuery>,
) -> AppResult<impl IntoResponse> {
    let q = params.q.trim();
    if q.is_empty() || q.len() > 200 {
        return Err(AppError::Validation(
            "Search query must be 1-200 characters".to_string(),
        ));
    }

    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(20).min(100);
    let sort = params.sort.as_deref().unwrap_or("relevance");

    let service = PostService::new(db);
    let (posts, total) = service.search(q, params.forum_id, page, per_page, sort).await?;
    let items = posts.into_iter().map(PostResponse::from).collect();

    Ok(ApiResponse::ok(PaginatedResponse::new(
        items, total, page, per_page,
    )))
}
