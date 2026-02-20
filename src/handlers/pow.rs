use crate::error::AppResult;
use crate::middleware::auth::parse_user_id;
use crate::middleware::AuthUser;
use crate::response::ApiResponse;
use crate::utils::pow::{
    generate_salt, now_epoch_seconds, sign_challenge, PowChallenge, PowConfig,
};
use axum::{response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct PowChallengeRequest {
    pub action: String,
    pub target_type: String,
    pub target_id: i32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PowChallengeResponse {
    pub pow_token: String,
    pub difficulty: u8,
    pub expires_at: i64,
}

#[utoipa::path(
    post,
    path = "/api/v1/pow/challenge",
    security(("jwt_token" = [])),
    request_body = PowChallengeRequest,
    responses(
        (status = 200, description = "PoW challenge", body = PowChallengeResponse),
        (status = 401, description = "Unauthorized", body = crate::error::AppError),
    ),
    tag = "pow"
)]
pub async fn create_pow_challenge(
    auth_user: AuthUser,
    Json(payload): Json<PowChallengeRequest>,
) -> AppResult<impl IntoResponse> {
    let user_id = parse_user_id(&auth_user)?;
    let cfg = PowConfig::from_env()?;

    let now = now_epoch_seconds();
    let expires_at = now + cfg.ttl_seconds;

    let challenge = PowChallenge {
        v: cfg.version,
        action: payload.action,
        target_type: payload.target_type,
        target_id: payload.target_id,
        user_id,
        issued_at: now,
        expires_at,
        difficulty: cfg.difficulty,
        salt: generate_salt(),
    };

    let pow_token = sign_challenge(&cfg.secret, &challenge)?;

    Ok(ApiResponse::ok(PowChallengeResponse {
        pow_token,
        difficulty: challenge.difficulty,
        expires_at: challenge.expires_at,
    }))
}
