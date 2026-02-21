use crate::error::AppResult;
use crate::middleware::auth::parse_user_id;
use crate::middleware::AuthUser;
use crate::response::ApiResponse;
use crate::services::comment::CommentService;
use crate::services::notification::NotificationService;
use crate::services::points::PointsService;
use crate::services::post::PostService;
use crate::services::vote::VoteService;
use crate::utils::pow::{validate_pow_solution, verify_and_decode_challenge, PowConfig};
use crate::websocket::hub::NotificationHub;
use axum::{extract::Path, response::IntoResponse, Extension, Json};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct VoteRequest {
    /// Vote value: -1 (downvote), 0 (remove vote), 1 (upvote)
    pub value: i16,
    /// PoW token from /api/v1/pow/challenge
    pub pow_token: String,
    /// PoW nonce computed on client
    pub pow_nonce: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VoteResponse {
    /// Target type (post or comment)
    pub target_type: String,
    /// Target ID
    pub target_id: i32,
    /// Current vote value
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

    // PoW verify (bind to user/action/target)
    let pow_cfg = PowConfig::from_env()?;
    let challenge = verify_and_decode_challenge(&pow_cfg.secret, &payload.pow_token)?;
    if challenge.user_id != user_id
        || challenge.action != "vote"
        || challenge.target_type != "post"
        || challenge.target_id != id
    {
        return Err(crate::error::AppError::Validation(
            "pow_token mismatch".to_string(),
        ));
    }
    validate_pow_solution(&challenge, &payload.pow_nonce)?;

    let service = VoteService::new(db.clone());
    let change = service.set_vote(user_id, "post", id, payload.value).await?;

    // 按状态迁移结算积分，确保可加可减且不被重复请求刷分。
    let points_delta = match (change.old_value, change.new_value) {
        (0, 1) | (-1, 1) => 1,
        (1, 0) | (1, -1) => -1,
        _ => 0,
    };
    if points_delta != 0 {
        let points = PointsService::new(db.clone());
        // 忽略结算失败，避免影响核心投票流程（可在日志中追踪）
        if let Err(e) = points
            .apply_vote_points(user_id, "post", id, points_delta)
            .await
        {
            tracing::warn!("Failed to apply vote points: {:?}", e);
        }
    }

    // Notify post author on vote (not on toggle-off)
    if change.new_value != 0 {
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
        value: change.new_value,
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

    // PoW verify (bind to user/action/target)
    let pow_cfg = PowConfig::from_env()?;
    let challenge = verify_and_decode_challenge(&pow_cfg.secret, &payload.pow_token)?;
    if challenge.user_id != user_id
        || challenge.action != "vote"
        || challenge.target_type != "comment"
        || challenge.target_id != id
    {
        return Err(crate::error::AppError::Validation(
            "pow_token mismatch".to_string(),
        ));
    }
    validate_pow_solution(&challenge, &payload.pow_nonce)?;

    let service = VoteService::new(db.clone());
    let change = service
        .set_vote(user_id, "comment", id, payload.value)
        .await?;

    // 按状态迁移结算积分，确保可加可减且不被重复请求刷分。
    let points_delta = match (change.old_value, change.new_value) {
        (0, 1) | (-1, 1) => 1,
        (1, 0) | (1, -1) => -1,
        _ => 0,
    };
    if points_delta != 0 {
        let points = PointsService::new(db.clone());
        if let Err(e) = points
            .apply_vote_points(user_id, "comment", id, points_delta)
            .await
        {
            tracing::warn!("Failed to apply vote points: {:?}", e);
        }
    }

    // Notify comment author on vote (not on toggle-off)
    if change.new_value != 0 {
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
        value: change.new_value,
    }))
}
