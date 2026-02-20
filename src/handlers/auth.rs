use crate::error::{AppError, AppResult};
use crate::middleware::auth::parse_user_id;
use crate::middleware::AuthUser;
use crate::models::UserModel;
use crate::response::ApiResponse;
use crate::services::auth::AuthService;
use crate::services::email::EmailService;
use anyhow::anyhow;
use axum::{
    http::{header, HeaderMap, HeaderValue},
    response::{IntoResponse, Response},
    Extension, Json,
};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RegisterRequest {
    /// Username (3-50 characters)
    #[validate(length(min = 3, max = 50))]
    pub username: String,
    /// Email address
    #[validate(email)]
    pub email: String,
    /// Password (min 8 characters)
    #[validate(length(min = 8))]
    pub password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    /// Username or email
    pub username: String,
    /// User password
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    /// JWT access token
    pub token: String,
    /// JWT refresh token
    pub refresh_token: String,
    /// User ID
    pub user_id: i32,
    /// Username
    pub username: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RegisterResponse {
    /// JWT access token
    pub token: String,
    /// JWT refresh token
    pub refresh_token: String,
    /// User ID
    pub user_id: i32,
    /// Username
    pub username: String,
    /// Success message
    pub message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserResponse {
    /// User ID
    pub id: i32,
    /// Username
    pub username: String,
    /// Email address
    pub email: String,
    /// Avatar URL
    pub avatar_url: Option<String>,
    /// User bio/description
    pub bio: Option<String>,
    /// User karma score
    pub karma: i32,
    /// User role (user, admin, moderator)
    pub role: String,
}

impl From<UserModel> for UserResponse {
    fn from(user: UserModel) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            avatar_url: user.avatar_url,
            bio: user.bio,
            karma: user.karma,
            role: user.role,
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "User registered successfully", body = RegisterResponse),
        (status = 400, description = "Validation error", body = AppError),
        (status = 409, description = "Username or email already exists", body = AppError),
    ),
    tag = "auth"
)]
pub async fn register(
    Extension(db): Extension<DatabaseConnection>,
    Extension(email_service): Extension<EmailService>,
    Json(payload): Json<RegisterRequest>,
) -> AppResult<impl IntoResponse> {
    // Validate input
    payload
        .validate()
        .map_err(|e| AppError::Validation(format!("Validation error: {e}")))?;

    let service = AuthService::new(db);
    let (user, access_token, refresh_token) = service
        .register(
            &payload.username,
            &payload.email,
            &payload.password,
            &email_service,
        )
        .await?;

    let auth_config = crate::config::auth::AuthConfig::from_env();
    let message = if auth_config.require_email_verification {
        "Registration successful. Please check your email to verify your account.".to_string()
    } else {
        "Registration successful.".to_string()
    };

    let response = RegisterResponse {
        token: access_token.clone(),
        refresh_token: refresh_token.clone(),
        user_id: user.id,
        username: user.username,
        message,
    };

    let mut http_response = ApiResponse::ok(response).into_response();
    set_auth_cookies(&mut http_response, &access_token, &refresh_token)?;
    Ok(http_response)
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = AuthResponse),
        (status = 400, description = "Invalid credentials", body = AppError),
        (status = 401, description = "Account not verified", body = AppError),
    ),
    tag = "auth"
)]
pub async fn login(
    Extension(db): Extension<DatabaseConnection>,
    Json(payload): Json<LoginRequest>,
) -> AppResult<impl IntoResponse> {
    let service = AuthService::new(db);
    let (user, access_token, refresh_token) =
        service.login(&payload.username, &payload.password).await?;

    let response = AuthResponse {
        token: access_token.clone(),
        refresh_token: refresh_token.clone(),
        user_id: user.id,
        username: user.username,
    };

    let mut http_response = ApiResponse::ok(response).into_response();
    set_auth_cookies(&mut http_response, &access_token, &refresh_token)?;
    Ok(http_response)
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    security(("jwt_token" = [])),
    responses(
        (status = 200, description = "Current user retrieved successfully", body = UserResponse),
        (status = 401, description = "Unauthorized", body = AppError),
    ),
    tag = "auth"
)]
pub async fn get_current_user(
    Extension(db): Extension<DatabaseConnection>,
    auth_user: AuthUser,
) -> AppResult<impl IntoResponse> {
    let user_id = parse_user_id(&auth_user)?;

    let service = AuthService::new(db);
    let user = service.get_user_by_id(user_id).await?;

    Ok(ApiResponse::ok(UserResponse::from(user)))
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ChangePasswordRequest {
    /// Current password
    pub current_password: String,
    /// New password (min 8 characters)
    #[validate(length(min = 8))]
    pub new_password: String,
}

#[utoipa::path(
    put,
    path = "/api/v1/auth/password",
    security(("jwt_token" = [])),
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "Password changed successfully", body = String),
        (status = 400, description = "Validation error", body = AppError),
        (status = 401, description = "Unauthorized", body = AppError),
    ),
    tag = "auth"
)]
pub async fn change_password(
    Extension(db): Extension<DatabaseConnection>,
    auth_user: AuthUser,
    Json(payload): Json<ChangePasswordRequest>,
) -> AppResult<impl IntoResponse> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let user_id = parse_user_id(&auth_user)?;

    let service = AuthService::new(db);
    service
        .change_password(user_id, &payload.current_password, &payload.new_password)
        .await?;

    Ok(ApiResponse::ok("Password changed successfully"))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyEmailRequest {
    /// Email verification token
    pub token: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/verify-email",
    request_body = VerifyEmailRequest,
    responses(
        (status = 200, description = "Email verified successfully", body = String),
        (status = 400, description = "Invalid token", body = AppError),
    ),
    tag = "auth"
)]
pub async fn verify_email(
    Extension(db): Extension<DatabaseConnection>,
    Json(payload): Json<VerifyEmailRequest>,
) -> AppResult<impl IntoResponse> {
    let service = AuthService::new(db);
    service.verify_email(&payload.token).await?;
    Ok(ApiResponse::ok("Email verified successfully"))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/resend-verification",
    security(("jwt_token" = [])),
    responses(
        (status = 200, description = "Verification email sent", body = serde_json::Value),
        (status = 401, description = "Unauthorized", body = AppError),
    ),
    tag = "auth"
)]
pub async fn resend_verification(
    Extension(db): Extension<DatabaseConnection>,
    Extension(email_service): Extension<EmailService>,
    auth_user: AuthUser,
) -> AppResult<impl IntoResponse> {
    let user_id = parse_user_id(&auth_user)?;

    let service = AuthService::new(db);
    service.resend_verification(user_id, &email_service).await?;
    Ok(ApiResponse::ok(
        serde_json::json!({ "message": "Verification email sent" }),
    ))
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ForgotPasswordRequest {
    /// Email address
    #[validate(email)]
    pub email: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/forgot-password",
    request_body = ForgotPasswordRequest,
    responses(
        (status = 200, description = "Password reset email sent if account exists", body = serde_json::Value),
        (status = 400, description = "Validation error", body = AppError),
    ),
    tag = "auth"
)]
pub async fn forgot_password(
    Extension(db): Extension<DatabaseConnection>,
    Extension(email_service): Extension<EmailService>,
    Json(payload): Json<ForgotPasswordRequest>,
) -> AppResult<impl IntoResponse> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let service = AuthService::new(db);
    service
        .forgot_password(&payload.email, &email_service)
        .await?;

    // Always return success to prevent email enumeration
    Ok(ApiResponse::ok(
        serde_json::json!({ "message": "If an account with that email exists, a password reset link has been sent." }),
    ))
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ResetPasswordRequest {
    /// Password reset token
    pub token: String,
    /// New password (min 8 characters)
    #[validate(length(min = 8))]
    pub new_password: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/reset-password",
    request_body = ResetPasswordRequest,
    responses(
        (status = 200, description = "Password reset successfully", body = serde_json::Value),
        (status = 400, description = "Validation error", body = AppError),
        (status = 400, description = "Invalid token", body = AppError),
    ),
    tag = "auth"
)]
pub async fn reset_password(
    Extension(db): Extension<DatabaseConnection>,
    Json(payload): Json<ResetPasswordRequest>,
) -> AppResult<impl IntoResponse> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let service = AuthService::new(db);
    service
        .reset_password(&payload.token, &payload.new_password)
        .await?;

    Ok(ApiResponse::ok(
        serde_json::json!({ "message": "Password has been reset successfully" }),
    ))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshTokenRequest {
    /// Refresh token
    pub refresh_token: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TokenResponse {
    /// New JWT access token
    pub token: String,
    /// New JWT refresh token
    pub refresh_token: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "New access token generated", body = TokenResponse),
        (status = 401, description = "Invalid or expired refresh token", body = AppError),
    ),
    tag = "auth"
)]
pub async fn refresh_token(
    Extension(db): Extension<DatabaseConnection>,
    headers: HeaderMap,
    payload: Option<Json<RefreshTokenRequest>>,
) -> AppResult<impl IntoResponse> {
    let refresh_token = payload
        .and_then(|Json(body)| body.refresh_token)
        .or_else(|| {
            crate::utils::cookie::extract_cookie(
                &headers,
                crate::utils::cookie::REFRESH_TOKEN_COOKIE,
            )
        })
        .ok_or(AppError::Unauthorized)?;

    // Decode the refresh token
    let claims = crate::utils::jwt::decode_jwt(&refresh_token)?;

    // Verify it's a refresh token
    if !crate::utils::jwt::is_refresh_token(&claims) {
        return Err(AppError::Unauthorized);
    }

    // Get user ID from claims
    let user_id_str = claims.sub;
    let user_id: i32 = user_id_str.parse().map_err(|_| AppError::Unauthorized)?;

    // Verify user exists
    let service = AuthService::new(db);
    let _user = service.get_user_by_id(user_id).await?;

    // Generate new tokens
    let new_access_token = crate::utils::jwt::encode_access_token(&user_id_str)?;
    let new_refresh_token = crate::utils::jwt::encode_refresh_token(&user_id_str)?;

    let response = TokenResponse {
        token: new_access_token.clone(),
        refresh_token: new_refresh_token.clone(),
    };

    let mut http_response = ApiResponse::ok(response).into_response();
    set_auth_cookies(&mut http_response, &new_access_token, &new_refresh_token)?;
    Ok(http_response)
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    security(("jwt_token" = [])),
    responses(
        (status = 200, description = "Logout successful", body = String),
    ),
    tag = "auth"
)]
pub async fn logout() -> AppResult<impl IntoResponse> {
    let mut response = ApiResponse::ok("Logout successful").into_response();
    clear_auth_cookies(&mut response)?;
    Ok(response)
}

fn set_auth_cookies(
    response: &mut Response,
    access_token: &str,
    refresh_token: &str,
) -> AppResult<()> {
    let access_cookie = crate::utils::cookie::build_auth_cookie(
        crate::utils::cookie::ACCESS_TOKEN_COOKIE,
        access_token,
        crate::utils::jwt::access_token_expiry_seconds(),
    );
    let refresh_cookie = crate::utils::cookie::build_auth_cookie(
        crate::utils::cookie::REFRESH_TOKEN_COOKIE,
        refresh_token,
        crate::utils::jwt::refresh_token_expiry_seconds(),
    );

    append_set_cookie(response, &access_cookie)?;
    append_set_cookie(response, &refresh_cookie)?;
    Ok(())
}

fn clear_auth_cookies(response: &mut Response) -> AppResult<()> {
    append_set_cookie(
        response,
        &crate::utils::cookie::build_clear_cookie(crate::utils::cookie::ACCESS_TOKEN_COOKIE),
    )?;
    append_set_cookie(
        response,
        &crate::utils::cookie::build_clear_cookie(crate::utils::cookie::REFRESH_TOKEN_COOKIE),
    )?;
    Ok(())
}

fn append_set_cookie(response: &mut Response, cookie_value: &str) -> AppResult<()> {
    let value = HeaderValue::from_str(cookie_value).map_err(|e| {
        AppError::Internal(anyhow!("Failed to build Set-Cookie header value: {}", e))
    })?;
    response.headers_mut().append(header::SET_COOKIE, value);
    Ok(())
}
