use crate::error::AppResult;
use crate::middleware::auth::parse_user_id;
use crate::middleware::AuthUser;
use crate::response::ApiResponse;
use crate::services::comment::CommentService;
use crate::services::notification::NotificationService;
use crate::services::post::PostService;
use crate::services::vote::VoteService;
use crate::websocket::hub::NotificationHub;
use axum::{extract::Path, response::IntoResponse, Extension, Json};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct VoteRequest {
    pub value: i16,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VoteResponse {
    pub target_type: String,
    pub target_id: i32,
    pub value: i16,
}

#[utoipa::path(
    post,
    path = "/api/v1/posts/{id}/vote",
    security(("jwt_token" = [])),
    params(("id" = i32, Path, description = "Post ID")),
    request_body = VoteRequest,
    responses(
        (status = 200, description = "Vote recorded", body = VoteResponse),
        (status = 401, description = "Unauthorized", body = crate::error::AppError),
    ),
    tag = "votes"
)]
pub async fn vote_post(
    Extension(db): Extension<DatabaseConnection>,
    Extension(hub): Extension<NotificationHub>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
    Json(payload): Json<VoteRequest>,
) -> AppResult<impl IntoResponse> {
    let user_id = parse_user_id(&auth_user)?;

    let service = VoteService::new(db.clone());
    let vote = service.vote(user_id, "post", id, payload.value).await?;

    // Notify post author on vote (not on toggle-off)
    if vote.value != 0 {
        let post_service = PostService::new(db.clone());
        if let Ok(post) = post_service.get_by_id(id).await {
            let notif = NotificationService::new(db, hub);
            let _ = notif
                .notify(
                    post.user_id,
                    user_id,
                    "vote_on_post",
                    "post",
                    id,
                    "Someone voted on your post",
                )
                .await;
        }
    }

    Ok(ApiResponse::ok(VoteResponse {
        target_type: "post".to_string(),
        target_id: id,
        value: vote.value,
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/comments/{id}/vote",
    security(("jwt_token" = [])),
    params(("id" = i32, Path, description = "Comment ID")),
    request_body = VoteRequest,
    responses(
        (status = 200, description = "Vote recorded", body = VoteResponse),
        (status = 401, description = "Unauthorized", body = crate::error::AppError),
    ),
    tag = "votes"
)]
pub async fn vote_comment(
    Extension(db): Extension<DatabaseConnection>,
    Extension(hub): Extension<NotificationHub>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
    Json(payload): Json<VoteRequest>,
) -> AppResult<impl IntoResponse> {
    let user_id = parse_user_id(&auth_user)?;

    let service = VoteService::new(db.clone());
    let vote = service.vote(user_id, "comment", id, payload.value).await?;

    // Notify comment author on vote (not on toggle-off)
    if vote.value != 0 {
        let comment_service = CommentService::new(db.clone());
        if let Ok(comment) = comment_service.get_by_id(id).await {
            let notif = NotificationService::new(db, hub);
            let _ = notif
                .notify(
                    comment.user_id,
                    user_id,
                    "vote_on_comment",
                    "comment",
                    id,
                    "Someone voted on your comment",
                )
                .await;
        }
    }

    Ok(ApiResponse::ok(VoteResponse {
        target_type: "comment".to_string(),
        target_id: id,
        value: vote.value,
    }))
}
