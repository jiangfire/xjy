use crate::error::AppResult;
use crate::middleware::AuthUser;
use crate::models::NotificationModel;
use crate::response::{ApiResponse, PaginatedResponse, PaginationQuery};
use crate::services::notification::NotificationService;
use crate::websocket::hub::NotificationHub;
use axum::{extract::Path, extract::Query, response::IntoResponse, Extension};
use sea_orm::DatabaseConnection;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct NotificationResponse {
    pub id: i32,
    pub kind: String,
    pub actor_id: i32,
    pub target_type: String,
    pub target_id: i32,
    pub message: String,
    pub is_read: bool,
    pub created_at: String,
}

impl From<NotificationModel> for NotificationResponse {
    fn from(n: NotificationModel) -> Self {
        Self {
            id: n.id,
            kind: n.kind,
            actor_id: n.actor_id,
            target_type: n.target_type,
            target_id: n.target_id,
            message: n.message,
            is_read: n.is_read,
            created_at: n.created_at.to_string(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UnreadCountResponse {
    pub count: u64,
}

fn get_user_id(auth_user: &AuthUser) -> AppResult<i32> {
    crate::middleware::auth::parse_user_id(auth_user)
}

#[utoipa::path(
    get,
    path = "/api/v1/notifications",
    security(("jwt_token" = [])),
    params(
        ("page" = Option<u64>, Query, description = "Page number"),
        ("per_page" = Option<u64>, Query, description = "Items per page"),
    ),
    responses(
        (status = 200, description = "List of notifications", body = PaginatedResponse<NotificationResponse>),
        (status = 401, description = "Unauthorized", body = crate::error::AppError),
    ),
    tag = "notifications"
)]
pub async fn list_notifications(
    Extension(db): Extension<DatabaseConnection>,
    Extension(hub): Extension<NotificationHub>,
    auth_user: AuthUser,
    Query(params): Query<PaginationQuery>,
) -> AppResult<impl IntoResponse> {
    let user_id = get_user_id(&auth_user)?;
    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(20).min(100);

    let service = NotificationService::new(db, hub);
    let (notifications, total) = service.list_for_user(user_id, page, per_page).await?;
    let items = notifications
        .into_iter()
        .map(NotificationResponse::from)
        .collect();

    Ok(ApiResponse::ok(PaginatedResponse::new(
        items, total, page, per_page,
    )))
}

#[utoipa::path(
    get,
    path = "/api/v1/notifications/unread-count",
    security(("jwt_token" = [])),
    responses(
        (status = 200, description = "Unread notification count", body = UnreadCountResponse),
        (status = 401, description = "Unauthorized", body = crate::error::AppError),
    ),
    tag = "notifications"
)]
pub async fn unread_count(
    Extension(db): Extension<DatabaseConnection>,
    Extension(hub): Extension<NotificationHub>,
    auth_user: AuthUser,
) -> AppResult<impl IntoResponse> {
    let user_id = get_user_id(&auth_user)?;
    let service = NotificationService::new(db, hub);
    let count = service.unread_count(user_id).await?;
    Ok(ApiResponse::ok(UnreadCountResponse { count }))
}

#[utoipa::path(
    put,
    path = "/api/v1/notifications/{id}/read",
    security(("jwt_token" = [])),
    params(("id" = i32, Path, description = "Notification ID")),
    responses(
        (status = 200, description = "Notification marked as read", body = String),
        (status = 401, description = "Unauthorized", body = crate::error::AppError),
    ),
    tag = "notifications"
)]
pub async fn mark_read(
    Extension(db): Extension<DatabaseConnection>,
    Extension(hub): Extension<NotificationHub>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> AppResult<impl IntoResponse> {
    let user_id = get_user_id(&auth_user)?;
    let service = NotificationService::new(db, hub);
    service.mark_read(id, user_id).await?;
    Ok(ApiResponse::ok("Notification marked as read"))
}

#[utoipa::path(
    put,
    path = "/api/v1/notifications/read-all",
    security(("jwt_token" = [])),
    responses(
        (status = 200, description = "All notifications marked as read", body = serde_json::Value),
        (status = 401, description = "Unauthorized", body = crate::error::AppError),
    ),
    tag = "notifications"
)]
pub async fn mark_all_read(
    Extension(db): Extension<DatabaseConnection>,
    Extension(hub): Extension<NotificationHub>,
    auth_user: AuthUser,
) -> AppResult<impl IntoResponse> {
    let user_id = get_user_id(&auth_user)?;
    let service = NotificationService::new(db, hub);
    let count = service.mark_all_read(user_id).await?;
    Ok(ApiResponse::ok(serde_json::json!({ "marked_read": count })))
}
